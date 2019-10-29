use std::f64;
use serde_json::{Value, Number, Map};
use super::shy_token::ShyValue;
use super::shy_scalar::ShyScalar;
use super::shy_object::ShyObject;
use super::execution_context::ExecutionContext;

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
    /// encode unsupported values as Value::Strings, often by prepending a suffix.
    /// The reverse conversion will need to parse these strings out and reconstruct the proper ShyValue.
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

impl From<ShyValue> for Value {
    fn from(v : ShyValue) -> Self {
        (&v).into()
    }
}

impl From<&Value> for ShyValue {
    fn from(v : &Value) -> Self {
        match v {
            Value::Null => ShyValue::Scalar(ShyScalar::Null),
            Value::Bool(b) => b.into(),

            // A Value::String may represent any of several ShyValue variants, so must be parsed. 
            // The prefix (if any) usually determines which variant to create. 
            Value::String(ref nan) if nan.to_lowercase() == "nan" => ShyValue::Scalar(f64::NAN.into()),
            Value::String(ref err) if err.starts_with("Error: ") => ShyValue::error(err[7..].into()),
            Value::String(ref func_name) if func_name.starts_with("FunctionName: ") => ShyValue::FunctionName(func_name[13..].into()),
            Value::String(ref prop_chain) if prop_chain.starts_with("PropertyChain: ") => ShyValue::property_chain(prop_chain[15..].into()),
            Value::String(ref variable) if variable.starts_with("Variable: ") => ShyValue::Variable(variable[10..].into()),
            Value::String(s) => s.clone().into(),

            Value::Number(ref n) if (*n).is_i64() => ShyValue::Scalar(ShyScalar::Integer(n.as_i64().unwrap())),
            Value::Number(ref f) => ShyValue::Scalar(ShyScalar::Rational(f.as_f64().unwrap())),
            Value::Array(a) => ShyValue::Vector(
                a.iter()
                .map(|item|
                  match item {
                      Value::Null => ShyScalar::Null,
                      Value::Bool(b) => (*b).into(),
                      Value::String(s) => s.clone().into(),
                      Value::Number(ref n) if (*n).is_i64() => ShyScalar::Integer(n.as_i64().unwrap()),
                      Value::Number(ref f) => ShyScalar::Rational(f.as_f64().unwrap()),
                      _ => ShyScalar::Error("Unsupported type of scalar".into())
                  })
                .collect()
            ),
            Value::Object(o) => {
                let shy_object = ShyObject::empty();
                {
                    let mut assoc = shy_object.as_deref_mut();
                    for (key, value) in o {
                        assoc.set(key, value.into());
                    }
                }
                ShyValue::Object(shy_object)
            }
        }
    }
}

impl From<Value> for ShyValue {
    fn from(v : Value) -> Self {
        (&v).into()
    }
}

/// Convert ExecutionContext into a serde-json Value, omitting the functions.
impl<'a> From<ExecutionContext<'a>> for Value {
    fn from(ctx : ExecutionContext<'a>) -> Self {
        (&ctx).into()
    }
}

impl<'a> From<&ExecutionContext<'a>> for Value {
    fn from(ctx : &ExecutionContext<'a>) -> Self {
        let mut variables_serde_map : Map<String, Value> = Map::with_capacity(ctx.variables.len());
        for (key, val) in &ctx.variables {
            variables_serde_map.insert(key.clone(), val.into());
        }
        let mut ctx_serde_map : Map<String, Value> = Map::with_capacity(2);
        ctx_serde_map.insert("is_applicable".into(), Value::Bool(ctx.is_applicable));
        ctx_serde_map.insert("variables".into(), Value::Object(variables_serde_map));
        Value::Object(ctx_serde_map)
    }
}

#[cfg(test)]
/// Tests of the Json Conversions.
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    #[test]
    /// Test conversion from several ShyValue variants into Serde Values using the From/Into Traits.
    fn shy_value_to_serde_value() {
        let null_shy_value : ShyValue = ShyValue::Scalar(ShyScalar::Null);
        let null_value : Value = null_shy_value.into();
        asserting("null Value conversion")
          .that(&(match null_value { Value::Null => true, _ => false }))
          .is_equal_to(true);
        
        let bool_shy_value : ShyValue = true.into();
        let bool_value : Value = bool_shy_value.into();
        asserting("bool Value conversion")
          .that(&(match bool_value { Value::Bool(true) => true, _ => false }))
          .is_equal_to(true);
        
        let string_shy_value : ShyValue = "Shy".into();
        let string_value : Value = string_shy_value.into();
        asserting("string Value conversion")
          .that(&(match string_value { Value::String(s) => s == "Shy".to_string(), _ => false }))
          .is_equal_to(true);

        let integer_shy_value : ShyValue = 99.into();
        let integer_value : Value = integer_shy_value.into();
        asserting("integer Value conversion")
          .that(&(match integer_value { Value::Number(n) => n.as_i64().unwrap() == 99, _ => false }))
          .is_equal_to(true);

        let shy_object = ShyObject::empty();
        {
            let mut deref_shy_object = shy_object.as_deref_mut();
            (*deref_shy_object).set("depth".into(), 1500.0.into());
        }
        let object_value : Value = ShyValue::Object(shy_object).into();
        asserting("object Value conversion")
          .that(&(
              match object_value {
                  Value::Object(map) => {
                      if let Some(Value::Number(depth)) = map.get("depth".into()){
                          depth.as_f64().unwrap() == 1500.0
                      }
                      else {
                          false
                      }
                  },
                  _ => false
              })
          )
          .is_equal_to(true);

    }

    #[test]
    fn serde_value_to_shy_value() {
        let null_serde_value = Value::Null;
        let null_shy_value : ShyValue = null_serde_value.into();
        asserting("null Value conversion")
          .that(&(match null_shy_value { ShyValue::Scalar(ShyScalar::Null) => true, _ => false }))
          .is_equal_to(true);

        let bool_serde_value = Value::Bool(true);
        let bool_shy_value : ShyValue = bool_serde_value.into();
        asserting("bool Value conversion")
          .that(&(match bool_shy_value { ShyValue::Scalar(ShyScalar::Boolean(true)) => true, _ => false }))
          .is_equal_to(true);
  
        let string_serde_value = Value::String("serde".into());
        let string_shy_value : ShyValue = string_serde_value.into();
        asserting("string Value conversion")
          .that(&(match string_shy_value { ShyValue::Scalar(ShyScalar::String(s)) => s == "serde", _ => false }))
          .is_equal_to(true);

        let rational_serde_value : Value = Value::Number(Number::from_f64(3.5).unwrap());
        let rational_shy_value : ShyValue = rational_serde_value.into();
        asserting("rational Value conversion")
          .that(&(match rational_shy_value { ShyValue::Scalar(ShyScalar::Rational(r)) => r == 3.5, _ => false }))
          .is_equal_to(true);

        let error_serde_value : Value = Value::String("Error: You did something bad!".into());
        let error_shy_value : ShyValue = error_serde_value.into();
        asserting("error Value conversion")
          .that(&(match error_shy_value { ShyValue::Scalar(ShyScalar::Error(err)) => err == "You did something bad!", _ => false }))
          .is_equal_to(true);

        let nan_serde_value : Value = Value::String("NaN".into());
        let nan_shy_value : ShyValue = nan_serde_value.into();
        asserting("NaN Value conversion")
          .that(&(match nan_shy_value { ShyValue::Scalar(ShyScalar::Rational(r)) => r.is_nan(), _ => false }))
          .is_equal_to(true);
    }

    #[test]
    /// Test conversion from ExecutionContext into Serde Values using the From/Into Traits.
    fn execution_context_to_serde_value() {
        let ctx = ExecutionContext::default();
        // Just ensure that it does not panic.
        let _value : Value = ctx.into();
    }

}
