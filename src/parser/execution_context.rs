
use std::collections::HashMap;
use std::f64;

use super::shy_scalar::ShyScalar;
use super::shy_token::ShyValue;
use super::shy_object::ShyObject;

//..................................................................

/// ExecutionContext holds variables and functions needed when executing expressions.
///   - Some variables are loaded for use in the formulas.
///   - Some variables are used to store the results of formulas after execution. 
///   - The functions may be called in the expressions.
pub struct ExecutionContext<'a> {
    pub variables: HashMap<String, ShyValue>,

    functions: HashMap<String, ShyFunction<'a>>
}

type ShyFunction<'a> = Box<(Fn(ShyValue) -> ShyValue + 'a)>;

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

    fn standard_functions() -> HashMap<String, ShyFunction<'a>> {
        let mut map = HashMap::new();
        map.insert("abs".to_string(), Ctx::shy_double_func(|x| x.abs()));
        map.insert("acos".to_string(), Ctx::shy_double_func(|x| x.acos()));
        map.insert("asin".to_string(), Ctx::shy_double_func(|x| x.asin()));
        map.insert("atan".to_string(), Ctx::shy_double_func(|x| x.atan()));
        map.insert("cos".to_string(), Ctx::shy_double_func(|x| x.cos()));
        map.insert("exp".to_string(), Ctx::shy_double_func(|x| x.exp()));
        map.insert("ln".to_string(), Ctx::shy_double_func(|x| x.ln()));
        map.insert("sin".to_string(), Ctx::shy_double_func(|x| x.sin()));
        map.insert("sqrt".to_string(), Ctx::shy_double_func(|x| x.sqrt()));
        map.insert("tan".to_string(), Ctx::shy_double_func(|x| x.tan()));
        map
    }

    fn standard_variables() ->  HashMap<String, ShyValue> {
        let mut map = HashMap::new();
        map.insert("PI".to_string(), f64::consts::PI.into());
        map.insert("π".to_string(), f64::consts::PI.into());
        map.insert("e".to_string(), f64::consts::E.into());
        map.insert("φ".to_string(), ( (1.0 + 5_f64.sqrt())/2.0).into());
        map.insert("PHI".to_string(), ( (1.0 + 5_f64.sqrt())/2.0).into());
        map
    }

    pub fn new(mut vars: HashMap<String, ShyValue>, mut funcs: HashMap<String, ShyFunction<'a>>) -> Self {
        vars.extend(ExecutionContext::standard_variables());
        funcs.extend(ExecutionContext::standard_functions());
        ExecutionContext {
            variables: vars,
            functions: funcs
        }
    }

    /// Create a default context that only defines math functions and constants.
    pub fn default() -> Self {
        ExecutionContext {
            variables: ExecutionContext::standard_variables(),
            functions: ExecutionContext::standard_functions()
        }
    }    

    /// Store a new value for the variable in the context.
    pub fn store(&mut self, name: &String, val: ShyValue) {
        self.variables.insert(name.clone(), val);
    }

    /// Store a new value in the object indicated by the path of property names. 
    pub fn store_chain(&mut self, path: &Vec<&'static str>, val: ShyValue) {
        let path_len = path.len();
        match path_len {
            0 => (),
            1 => { self.variables.insert(path[0].into(), val); () }
            _ => {
                // We strip one property off the path, because the lvalue must be one link back in the chain 
                // so that we can perform the final assignment using the last property in the chain.
                if let Some(ShyValue::Object(lvalue)) = self.vivify(&path[..path_len-1].to_vec()) {
                    lvalue.as_deref_mut().set(path[path_len-1], val);
                }
                ()
            }
        }
        
    }

    /// Obtain an lvalue for the given property chain, creating any intervening missing objects if possible. 
    /// This is analogous to ensuring that a directory in a file system exists by creating its 
    /// missing parent, grandparent, etc. directories, then creating the bottommost directory. 
    ///   path ... A series of property names that must have one or more entries. 
    /// Returns None in these cases: 
    ///     - if part of the path already exists but is not a ShyObject
    ///     - if part of the path refers to a ShyObject whose underlying ShyAssociation does not permit that property to be set 
    /// Otherwise, returns a Some(ShyValue::Object). This may be used as an lvalue for setting a property.
    fn vivify(&mut self, path: &Vec<&'static str>) -> Option<ShyValue> {
        let path_len = path.len();
        match path_len {
            0 => None, 
            1 => {
                let variable = path[0];
                match self.variables.get(variable) {
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
    pub fn load_chain(&self, chain: &Vec<&'static str>) -> Option<ShyValue> { 
        match chain.first() {
            None => None,
            Some(variable) => {
                let variable_str: &String = &(*variable).into();
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
