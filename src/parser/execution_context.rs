
use std::collections::HashMap;
use std::f64;

use super::shy_scalar::ShyScalar;
use super::shy_token::ShyValue;

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

    /// Retrieve the current value of the variable from the context, or None.
    pub fn load(&self, name: &String) -> Option<ShyValue> { 
        match self.variables.get(name) {
            Some(val) => Some(val.clone()),
            None => None
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
