
use std::convert::TryFrom;
use super::shy_token;

//..................................................................

/// ShyScalars are the atomic values that can be used as operands to operators and arguments to functions,
/// or returned as results.
#[derive(Clone, PartialEq, Debug)]
pub enum ShyScalar {
    Boolean(bool),
    Integer(i64),
    Rational(f64),
    String(String),
    Error(String)
}

impl ShyScalar {
    pub fn is_truthy(&self) -> bool {
        match self {
            ShyScalar::Boolean(value) => *value,
            ShyScalar::Integer(value) => *value != 0,
            ShyScalar::Rational(value) => *value != 0.0,
            ShyScalar::String(value) => shy_token::is_truthy(value),
            _ => false
        }
    }
}

impl From<bool> for ShyScalar { fn from(b: bool) -> Self { ShyScalar::Boolean(b) } }
impl From<i64> for ShyScalar { fn from(i: i64) -> Self { ShyScalar::Integer(i) } }
impl From<i32> for ShyScalar { fn from(i: i32) -> Self { ShyScalar::Integer(i.into()) } }
impl From<f64> for ShyScalar { fn from(f: f64) -> Self { ShyScalar::Rational(f) } }
impl From<String> for ShyScalar { fn from(s: String) -> Self { ShyScalar::String(s) } }
impl From<&str> for ShyScalar { fn from(s: &str) -> Self { ShyScalar::String(s.to_string()) } }

impl TryFrom<ShyScalar> for bool { 
    type Error = &'static str;
    fn try_from(value: ShyScalar) -> Result<Self, Self::Error> {
        match value {
            ShyScalar::Error(_) => Err("Value is an error, not a boolean"),
            _ => Ok(value.is_truthy())
        }
    }
}

impl TryFrom<ShyScalar> for i64 { 
    type Error = &'static str;
    fn try_from(value: ShyScalar) -> Result<Self, Self::Error> {
        match value {
            ShyScalar::Boolean(_) => Err("Value is a boolean, not an integer"),
            ShyScalar::Integer(i) => Ok(i),
            ShyScalar::Rational(r) => {
                let i = r as i64;
                let r2 = i as f64;
                if r2 == r { Ok(i) }
                else { Err("Value is a floating point that cannot be converted to an integer without loss of precision")}
            },
            ShyScalar::String(_) => Err("Value is a string, not an integer"),
            _ => Err("Value is not an integer")
        }
    }
}

impl TryFrom<ShyScalar> for f64 { 
    type Error = &'static str;
    fn try_from(value: ShyScalar) -> Result<Self, Self::Error> {
        match value {
            ShyScalar::Boolean(_) => Err("Value is a boolean, not a floating point number"),
            ShyScalar::Integer(i) => Ok(i as f64),
            ShyScalar::Rational(r) => Ok(r),
            ShyScalar::String(_) => Err("Value is a string, not a floating point number"),
            _ => Err("Value is not a floating point number")
        }
    }
}

impl TryFrom<ShyScalar> for String { 
    type Error = &'static str;
    fn try_from(value: ShyScalar) -> Result<Self, Self::Error> {
        match value {
            ShyScalar::Boolean(true) => Ok("true".to_string()),
            ShyScalar::Boolean(false) => Ok("false".to_string()),
            ShyScalar::Integer(i) => Ok(i.to_string()),
            ShyScalar::Rational(r) => Ok(r.to_string()),
            ShyScalar::String(s) => Ok(s),
            _ => Err("Value is an error")
        }
    }
}


