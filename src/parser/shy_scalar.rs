
use std::convert::TryFrom;

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

