
use std::collections::HashMap;
use std::f64;
use std::fmt;

use super::shy_scalar::ShyScalar;
use super::shy_token::ShyValue;
use super::shy_object::ShyObject;
use super::voting_rule::VotingRule;

//..................................................................

/// ExecutionContext holds variables and functions needed when executing expressions.
///   - Some variables are loaded for use in the formulas.
///   - Some variables are used to store the results of formulas after execution. 
///   - The functions may be called in the expressions.
pub struct ExecutionContext<'a> {
    pub variables: HashMap<String, ShyValue>,

    functions: HashMap<String, ShyFunction<'a>>
}

type ShyFunction<'a> = Box<(dyn Fn(ShyValue) -> ShyValue + 'a)>;

type Ctx<'a> = ExecutionContext<'a>;

impl<'a> ExecutionContext<'a> {

    pub fn shy_func<F>(f: F) -> ShyFunction<'a>
        where F: Fn(ShyValue) -> ShyValue + 'a {
            Box::new(f) as ShyFunction
    }

    /// Define a context function that assumes the argument is a float or integer or a vector
    /// that holds a single float or integer and returns a double.
    pub fn shy_double_func<G>(g: G) -> ShyFunction<'a>
        where G: Fn(f64) -> f64 + 'a {
            Ctx::shy_func(move |v| {
                match v {
                    ShyValue::Scalar(ShyScalar::Rational(x)) => g(x).into(),
                    ShyValue::Scalar(ShyScalar::Integer(i)) => g(i as f64).into(),
                    ShyValue::Vector(ref vect) if vect.len() == 1 => match vect[0] {
                        ShyScalar::Rational(x) => g(x).into(),
                        ShyScalar::Integer(i) => g(i as f64).into(),
                        _ => f64::NAN.into()
                    },
                    _ => f64::NAN.into()
                }
            })
    }

    /// Define a context function that assumes the argument is a float or integer or a vector
    /// that holds a single float or integer and returns a bool.
    pub fn shy_double_to_bool_func<G>(g: G) -> ShyFunction<'a>
        where G: Fn(f64) -> bool + 'a {
            Ctx::shy_func(move |v| {
                match v {
                    ShyValue::Scalar(ShyScalar::Rational(x)) => g(x).into(),
                    ShyValue::Scalar(ShyScalar::Integer(i)) => g(i as f64).into(),
                    ShyValue::Vector(ref vect) if vect.len() == 1 => match vect[0] {
                        ShyScalar::Rational(x) => g(x).into(),
                        ShyScalar::Integer(i) => g(i as f64).into(),
                        _ => ShyValue::error("Vector holding non-numeric value passed to function expecting numbers".into())
                    },
                    _ => ShyValue::error("Non-numeric value passed to function expecting a number".into())
                }
            })
    }

    /// Define a context function that acts like an if-then-else statement. It must be passed an array of three elements: 
    ///    - the boolean test
    ///    - the value to return if the test is true
    ///    - the value to return if the test is false.
    pub fn shy_if_func() -> ShyFunction<'a>
    {
        Ctx::shy_func(move |v| {
            match v {
                ShyValue::Vector(ref vect) if vect.len() == 3 => match vect[0] {
                    ShyScalar::Boolean(test) => ShyValue::Scalar(if test { vect[1].clone() } else { vect[2].clone() }),
                    _ => ShyValue::error("'if' function first argument must be a boolean value".into())
                },
                _ => ShyValue::error("'if' function requires exactly three arguments".into())
            }
        })
    }

    /// Define a context function that checks if the first argument is null and has two behaviours, 
    /// depending on whether it is called with one argument or true. 
    ///   - If two arguments: 
    ///     return the value of the first argument if it is not null, 
    ///     or the value of the second argument if the first is null.
    ///   - If one argument: 
    ///     return true if the sole argument is null and false otherwise. 
    /// 
    /// The behavior for two arguments is similar to the SQL Server SQL function ISNULL. 
    pub fn shy_isnull_func() -> ShyFunction<'a>
    {
        Ctx::shy_func(move |v| {
            match v {
                ShyValue::Vector(ref vect) if vect.len() == 1 => match vect[0] {
                    ShyScalar::Null => true.into(),
                    _ => false.into()
                },
                ShyValue::Vector(ref vect) if vect.len() == 2 => match vect[0] {
                    ShyScalar::Null => ShyValue::Scalar(vect[1].clone()),
                    _ => ShyValue::Scalar(vect[0].clone())
                },
                ShyValue::Scalar(ShyScalar::Null) => true.into(),
                ShyValue::Scalar(_) => false.into(),
                _ => ShyValue::error("'isnull' function requires one or two arguments".into())
            }
        })
    }    

    pub fn shy_voting_func(function_name : String, rule : VotingRule) -> ShyFunction<'a>
    {
        Ctx::shy_func(move |v| {
            match v {
                ShyValue::Vector(ref vect) if vect.len() == 0 => {
                    let vote = match rule {
                        VotingRule::None => true,
                        VotingRule::Unanimous =>  true,
                        _ => false
                    };
                    vote.into()
                },
                ShyValue::Vector(ref vect) if vect.len() > 0 => {
                    let full_count = vect.len();
                    let true_count = vect.iter().filter(|&v| v.is_truthy()).count();
                    let vote = match rule {
                        VotingRule::None => true_count == 0,
                        VotingRule::One => true_count == 1,
                        VotingRule::Any => true_count > 0,
                        VotingRule::Minority => true_count > 0 && true_count < (full_count + 1) / 2,
                        VotingRule::Half => true_count * 2 == full_count,
                        VotingRule::Majority => true_count > full_count / 2,
                        VotingRule::TwoThirds => true_count >= full_count * 2 / 3,
                        VotingRule::AllButOne => true_count > 0 && true_count == full_count - 1,
                        VotingRule::All => true_count == full_count,
                        VotingRule::Unanimous =>  true_count == 0 || true_count == full_count
                    };
                    vote.into()
                },
                _ => ShyValue::error(format!("'{}' function requires a vector as argument", function_name).into())
            }
        })
    }  

    pub fn standard_functions() -> HashMap<String, ShyFunction<'a>> {
        let mut map = HashMap::new();

        // Functions that take a double and return a double
        map.insert("abs".to_string(), Ctx::shy_double_func(|x| x.abs()));
        map.insert("acos".to_string(), Ctx::shy_double_func(|x| x.acos()));
        map.insert("acosh".to_string(), Ctx::shy_double_func(|x| x.acosh()));
        map.insert("asin".to_string(), Ctx::shy_double_func(|x| x.asin()));
        map.insert("asinh".to_string(), Ctx::shy_double_func(|x| x.asinh()));
        map.insert("atan".to_string(), Ctx::shy_double_func(|x| x.atan()));
        map.insert("ceil".to_string(), Ctx::shy_double_func(|x| x.ceil()));
        map.insert("cos".to_string(), Ctx::shy_double_func(|x| x.cos()));
        map.insert("cosh".to_string(), Ctx::shy_double_func(|x| x.cosh()));
        map.insert("exp".to_string(), Ctx::shy_double_func(|x| x.exp()));
        map.insert("floor".to_string(), Ctx::shy_double_func(|x| x.floor()));
        map.insert("fract".to_string(), Ctx::shy_double_func(|x| x.fract()));
        map.insert("ln".to_string(), Ctx::shy_double_func(|x| x.ln()));
        map.insert("log10".to_string(), Ctx::shy_double_func(|x| x.log10()));
        map.insert("log2".to_string(), Ctx::shy_double_func(|x| x.log2()));
        map.insert("sin".to_string(), Ctx::shy_double_func(|x| x.sin()));
        map.insert("sqrt".to_string(), Ctx::shy_double_func(|x| x.sqrt()));
        map.insert("tan".to_string(), Ctx::shy_double_func(|x| x.tan()));
        map.insert("tanh".to_string(), Ctx::shy_double_func(|x| x.tanh()));
        map.insert("trunc".to_string(), Ctx::shy_double_func(|x| x.trunc()));

        // Functions that take a double and return a boolean
        map.insert("is_finite".to_string(), Ctx::shy_double_to_bool_func(|x| x.is_finite()));
        map.insert("is_infinite".to_string(), Ctx::shy_double_to_bool_func(|x| x.is_infinite()));
        map.insert("is_nan".to_string(), Ctx::shy_double_to_bool_func(|x| x.is_nan()));
        map.insert("is_normal".to_string(), Ctx::shy_double_to_bool_func(|x| x.is_normal()));
        map.insert("is_sign_negative".to_string(), Ctx::shy_double_to_bool_func(|x| x.is_sign_negative()));
        map.insert("is_sign_positive".to_string(), Ctx::shy_double_to_bool_func(|x| x.is_sign_positive()));

        // The 'if' and 'isnull' functions
        map.insert("if".to_string(), Ctx::shy_if_func());
        map.insert("isnull".to_string(), Ctx::shy_isnull_func());

        // Voting functions, that count how many true versus false values are among the arguments
        map.insert("none".into(), Ctx::shy_voting_func("none".into(), VotingRule::None));
        map.insert("one".into(), Ctx::shy_voting_func("one".into(), VotingRule::One));
        map.insert("any".into(), Ctx::shy_voting_func("any".into(), VotingRule::Any));
        map.insert("minority".into(), Ctx::shy_voting_func("minority".into(), VotingRule::Minority));
        map.insert("half".into(), Ctx::shy_voting_func("half".into(), VotingRule::Half));
        map.insert("majority".into(), Ctx::shy_voting_func("majority".into(), VotingRule::Majority));
        map.insert("twothirds".into(), Ctx::shy_voting_func("twothirds".into(), VotingRule::TwoThirds));
        map.insert("allbutone".into(), Ctx::shy_voting_func("allbutone".into(), VotingRule::AllButOne));
        map.insert("all".into(), Ctx::shy_voting_func("all".into(), VotingRule::All));
        map.insert("unanimous".into(), Ctx::shy_voting_func("unanimous".into(), VotingRule::Unanimous));

        map
    }

    pub fn standard_variables() ->  HashMap<String, ShyValue> {
        let mut map = HashMap::new();
        map.insert("PI".to_string(), f64::consts::PI.into());
        map.insert("π".to_string(), f64::consts::PI.into());
        map.insert("e".to_string(), f64::consts::E.into());
        map.insert("φ".to_string(), ( (1.0 + 5_f64.sqrt())/2.0).into());
        map.insert("PHI".to_string(), ( (1.0 + 5_f64.sqrt())/2.0).into());
        map
    }

    /// Construct an ExecutionContext that adds the standard variables (like PI) and functions (like sin and exp) to those already defined by the caller.
    pub fn new(mut vars: HashMap<String, ShyValue>, mut funcs: HashMap<String, ShyFunction<'a>>) -> Self {
        vars.extend(ExecutionContext::standard_variables());
        funcs.extend(ExecutionContext::standard_functions());
        ExecutionContext {
            variables: vars,
            functions: funcs
        }
    }

    /// Create a default context that only defines the standard math functions and constants.
    pub fn default() -> Self {
        ExecutionContext {
            variables: ExecutionContext::standard_variables(),
            functions: ExecutionContext::standard_functions()
        }
    }    

    /// Store a new value for the variable in the context.
    pub fn store<V>(&mut self, name: &String, val: V)
    where V : Into<ShyValue>
    {
        self.variables.insert(name.clone(), val.into());
    }

    /// Store a new value in the object indicated by the path of property names. 
    /// Returns a Result to indicate success or failure, since it may not be possible to set the value for the given path.
    pub fn store_chain(&mut self, path: &Vec<String>, val: ShyValue) -> Result<(), ShyValue> {
        let path_len = path.len();
        match path_len {
            0 => Ok(()),
            1 => { self.variables.insert(path[0].clone(), val); Ok(()) }
            _ => {
                // We strip one property off the path, because the lvalue must be one link back in the chain 
                // so that we can perform the final assignment using the last property in the chain.
                if let Some(ShyValue::Object(lvalue)) = self.vivify(&path[..path_len-1].to_vec()) {
                    lvalue.as_deref_mut().set(&path[path_len-1], val);
                    Ok(())
                }
                else {
                    Err(ShyValue::bad_property_chain(path))
                }
            }
        }
        
    }

    fn string_to_static_str(s: String) -> &'static str {
        Box::leak(s.into_boxed_str())
    }

    /// Obtain an lvalue for the given property chain, creating any intervening missing objects if possible. 
    /// This is analogous to ensuring that a directory in a file system exists by creating its 
    /// missing parent, grandparent, etc. directories, then creating the bottommost directory. 
    ///   path ... A series of property names that must have one or more entries. 
    /// Returns None in these cases: 
    ///     - if part of the path already exists but is not a ShyObject
    ///     - if part of the path refers to a ShyObject whose underlying ShyAssociation does not permit that property to be set 
    /// Otherwise, returns a Some(ShyValue::Object). This may be used as an lvalue for setting a property.
    fn vivify(&mut self, path: &Vec<String>) -> Option<ShyValue> {
        let path_len = path.len();
        match path_len {
            0 => None, 
            1 => {
                let variable = path[0].clone();
                match self.variables.get(&variable) {
                    None => {
                        let obj = ShyObject::empty();
                        self.variables.insert(variable.into(), ShyValue::Object(obj.shallow_clone()));
                        Some(ShyValue::Object(obj))
                    },
                    Some(ShyValue::Object(obj)) => Some(ShyValue::Object(obj.shallow_clone())),
                    _ => None
                }
            }
            _ => {
                // We can't use ShyObject.vivify from the top, because self.variables is a HashMap.
                // We must manually vivify the first level, then we can use ShyObject.vivify for the rest.
                let top_key = &path[0].to_string();
                match self.variables.get(top_key) {
                    Some(ShyValue::Object(top_obj)) => {
                        let deep_obj_option = top_obj.shallow_clone().vivify(path[1..].to_vec(), path_len - 1, || ShyObject::empty());
                        match deep_obj_option {
                            Some(deep_obj) => Some(ShyValue::Object(deep_obj.shallow_clone())),
                            None => None
                        }
                    },
                    None => {
                        let mut top_obj = ShyObject::empty();
                        let new_top = ShyValue::Object(top_obj.shallow_clone());
                        self.variables.insert(top_key.clone(), new_top);
                        let deep_obj_option = top_obj.vivify(path[1..].to_vec(), path_len - 1, || ShyObject::empty());
                        match deep_obj_option {
                            Some(deep_obj) => Some(ShyValue::Object(deep_obj.shallow_clone())),
                            None => None
                        }
                    },
                    _ => None
                }
            }
        }
    }

    /// Retrieve the current value of the variable from the context, or None.
    /// Scalar values are cloned; ShyOjects are shallow cloned, because we need changes
    /// made to the context to be visible to the caller.
    pub fn load<T>(&self, name: &T) -> Option<ShyValue>
    where T : Into<String> + Sized + Clone { 
        let string_name : &String = &(name.clone()).into();
        match self.variables.get(string_name) {
            Some(ShyValue::Object(obj)) => Some(ShyValue::Object(obj.shallow_clone())),
            Some(val) => Some(val.clone()),
            None => None
        }
    }

    /// Retrieve the current value of the variable and property chain from the context, or None.
    /// Scalar values are cloned; ShyObjects are shallow cloned, because we need changes
    /// made to the context to be visible to the caller.
    /// The first name in the chain must be a variable name in the context. 
    /// The remaining names must be property names that can be traversed from object to object via get.
    pub fn load_chain(&self, chain: &Vec<String>) -> Option<ShyValue> { 
        match chain.first() {
            None => None,
            Some(variable) => {
                let variable_str: &String = &variable;
                match self.load(variable_str) {
                    Some(ShyValue::Object(ref obj)) if chain.len() == 1 => {
                        Some(ShyValue::Object(obj.shallow_clone()))
                    },
                    Some(ShyValue::Object(ref obj)) if chain.len() > 1 => {
                        let value_clone = ShyValue::Object(obj.shallow_clone());
                        let properties = &chain[1..];
                        Some(value_clone.get_chain(properties))
                    },
                    Some(_) if chain.len() > 1 => { None },
                    Some(ref first_value) if chain.len() == 1 => { Some(first_value.clone()) }
                    _ => None,
                }
            }
        }
    }

    /// Retrieve the value indicated by the given property chain, given as a string with properties separated by periods.
    pub fn load_str_chain(&self, str_chain: &str) -> Option<ShyValue> {
        self.load_chain(&Self::str_to_property_chain(str_chain))
    }

    /// Convert a string slice into a property chain vector.
    pub fn str_to_property_chain(chain_as_string : &str) -> Vec<String> {
        chain_as_string.split(".").map(|s| s.to_string()).collect()
    }

    /// Perform the common tasks associated with updating a variable associated with a property chain.
    /// Address these situations, where the chain...
    /// 
    ///    - had a previous value and can be changed, and the previous value must be combined with right_operand
    ///      (captured in the closure) to form a new value (e.g. x += 1) by calling present_cb
    ///    - had a previous value that cannot be changed, because can_set_property is false, so an error is returned
    ///    - had no previous value but can be set to one, so the value should be based on right_operand 
    ///      (captured by closure) and calling absent_cb. (absent_cb may choose to generate an error or not.)
    ///    - had no previous value and none can be set, because can_set_property is false, so an error is returned
    /// 
    /// In addition, it is possible that though can_set_property is true, the setting of the property may fail.
    /// In that case, an error is also returned. 
    /// 
    /// When an error is returned, it is as a ShyValue::Scalar(ShyScalar::Error).
    /// The return_cb callback decides what value to return: the previous value or the new value.
    /// If there is no previous value, use infer_previous_value as that value to be returned, if needed.
    pub fn property_chain_update(
        &mut self, 
        path: &Vec<String>,
        infer_prior_value: &ShyValue,
        present_cb: &dyn Fn(&ShyValue) -> ShyValue, 
        absent_cb: &dyn Fn() -> ShyValue,
        return_cb: &dyn Fn(&ShyValue, &ShyValue) -> ShyValue
        ) -> ShyValue {
        // TODO: Refactor and make property_chain_update a method on ExecutionContext. 
        // Follow path to get current value (if any)
        match self.load_chain(path) {
            // No current value (but no error)? Use absent_cb and compute a new value
            None => {
                // Compute result using absent_cb
                let new_value = absent_cb();
                if new_value.is_error() {
                    return new_value;
                }
                
                // Store result in context using path
                match self.store_chain(path, new_value.clone()) {
                    Err(error_value) => error_value,
                    _ => return_cb(infer_prior_value, &new_value)
                }
            },

            // Error retrieving current value? Forward the error.
            Some(ShyValue::Scalar(ShyScalar::Error(message))) => ShyValue::error(message),

            // Has a current value that is not an error? Use present_cb and compute new value.
            Some(current_value) => {
                // Compute result using present_cb
                let new_value = present_cb(&current_value);
                if new_value.is_error() {
                    return new_value;
                }
                
                // Store result in context using path
                match self.store_chain(path, new_value.clone()) {
                    Err(error_value) => error_value,
                    _ => return_cb(&current_value, &new_value)
                }
            }
        }
    }

    /// Call a function that is stored in the context.
    pub fn call(&self, function_name: String, args: ShyValue) -> ShyValue {
        match self.functions.get(&function_name) {
            Some(func) => func(args),
            None => ShyValue::error(format!("No function named {} in context", function_name))
        }
    }

}

impl<'a> From<&HashMap<String,f64>> for ExecutionContext<'a> {
    /// Create an ExecutionContext from a simple map of string-float pairs.
    fn from(initial_values: &HashMap<String,f64>) -> Self {
        let mut context = ExecutionContext::default();
        for (key, value) in &*initial_values {
            let wrapped_value: ShyValue = (*value).into();
            context.variables.insert(key.clone(), wrapped_value);
        }
        context
    }
}

impl<'a> fmt::Debug for ExecutionContext<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Exec Context: {:?}", self.variables)
    }
}
