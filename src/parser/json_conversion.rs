use serde_json::{Value, Number, Map};
use super::shy_token::ShyValue;
use super::shy_scalar::ShyScalar;

//  Convert ShyValues to and from Value enums in the serde crate.
//  Not all ShyValue variants can be expressed as a Value in serde, and vice versa.
//  Here are the mappings for ShyScalar (to be stored in ShyValue::Scalar): 
// 
//     Shy (ShyValue, ShyScalar)    Serde (Value, Number)
//     ---------------------------  ------------------------
//     ShyScalar::Null              Value::Null
//     ShyScalar::Boolean           Value::Bool
//     ShyScalar::Integer           Value::Number, Number::PosInt or Number::NegInt
//     ShyScalar::Rational          Value::Number, Number::Float
//     ShyScalar::String            Value::String (String does not start with special prefix)
//     ShyScalar::Error             Value::String (String starts with "Error:")
//     ShyValue::Vector             Value::Array
//     ShyValue::Object             Value::Object
//     ShyValue::Variable           Value::String (String starts with "Variable:")
//     ShyValue::PropertyChain      Value::String (String starts with "PropertyChain:")
//     ShyValue::FunctionName       Value::String (String starts with "FunctionName:")
//  
//  ShyValue::Vector is limited in Shy, because it only holds ShyScalars.
//  That needs to be refactored so that it can hold other ShyValue::Vectors and ShyObjects.
//  

impl From<&ShyScalar> for Value { 
    /// Create a Serde Value from a ShyScalar. 
    /// Since Serde Values can only represent what is valid for JSON (e.g. no NaN values),
    /// encode unsupported values as Value::Strings, often by prepending a string.
    fn from(s : &ShyScalar) -> Self { 
        match s {
            ShyScalar::Null => Value::Null,
            ShyScalar::Boolean(b) => Value::Bool(*b),
            ShyScalar::String(s) => Value::String(s.clone()),
            ShyScalar::Rational(ref r) if r.is_nan() => Value::String("NaN".into()),
            ShyScalar::Rational(r) => Value::Number(Number::from_f64(*r).unwrap()),
            ShyScalar::Integer(ref i) if *i >= 0 => Value::Number((*i).into()),
            ShyScalar::Integer(i) => Value::Number((*i as u64).into()),
            ShyScalar::Error(e) => Value::String(format!("Error: {}", e))
        }
    } 
}

impl From<&ShyValue> for Value {
    fn from(v : &ShyValue) -> Self {
        match v {
            ShyValue::Scalar(scalar) => scalar.into(),
            ShyValue::Vector(shy_vec) => Value::Array(shy_vec.iter().map(|shy_val| shy_val.into()).collect()),
            ShyValue::FunctionName(func_name) => Value::String(format!("FunctionName: {}", func_name)),
            ShyValue::PropertyChain(prop_chain) => Value::String(format!("PropertyChain: {}", prop_chain.join("."))),
            ShyValue::Variable(var_name) => Value::String(format!("Variable: {}", var_name)),
            ShyValue::Object(shy_obj) => {
                let deref = shy_obj.as_deref();
                let property_count = deref.keys().count();
                let mut serde_map : Map<String, Value> = Map::with_capacity(property_count);
                for key in deref.keys() {
                    match deref.get(&key) {
                      Some(shy_value) => {
                          serde_map.insert(key, shy_value.into());
                      },
                      None => panic!("key '{}' in ShyObject has no value", key)
                    }
                }
                Value::Object(serde_map)
            }
        }
    }
}


