#![allow(dead_code)]

#[allow(unused_imports)]

use crate::lexer::parser_token::ParserToken;
use std::mem::discriminant;
use std::f64;
use std::convert::TryFrom;
use std::collections::HashSet;
use std::cmp::Ordering;
use regex::Regex;

use super::factorial::factorial;
use super::factorial::factorial_approx;
use super::shy_operator::ShyOperator;
use super::shy_scalar::ShyScalar;
use super::execution_context::ExecutionContext;
use super::shy_object::ShyObject;


/*
    Data used in the ShuntingYard parser:

        - Associativity (used by ShyOperator, defined in associativity.rs)
        - ShyOperator (used by ShyToken, defined in shy_operator.rs)
        - ShyScalar (used by ShyValue, defined in shy_scalar.rs)
        - ShyValue (used by ShyToken)
        - ShyToken (parsed from ParserToken)

    1. The Lexer reads a string and yields ParserTokens.
    2. ShuntingYard converts ParserTokens into ShyTokens, whether Value variants (ShyValue) or Operator variants (ShyOperator). 
    3. ShuntingYard then resequences the ShyTokens, changing them from infix to postfix order. 

*/

//..................................................................

// Checking a string to see if it is truthy or falsy.

lazy_static! {
    static ref FALSEY: HashSet<&'static str> = {
        let mut falsey_values = HashSet::new();
        falsey_values.insert("F");
        falsey_values.insert("f");
        falsey_values.insert("false");
        falsey_values.insert("False");
        falsey_values.insert("FALSE");
        falsey_values.insert("n");
        falsey_values.insert("N");
        falsey_values.insert("no");
        falsey_values.insert("No");
        falsey_values.insert("NO");
        falsey_values.insert("0");
        falsey_values.insert("");
        falsey_values
    };
}

pub fn is_falsey(s: &str) -> bool {
    FALSEY.contains(s)
}

pub fn is_truthy(s: &str) -> bool {
    !FALSEY.contains(s)
}

//..................................................................

/// The Output stack of the Shunting Yard parser holds ShyValues wrapped inside a ShyToken.
///   - The final result of evaluating expressions is a single ShyValue, either a Scalar or a Vector.
///   - The Variable and FunctionName variants are intermediate tokens on the output stack that will be
///     removed in the process of evaluating functions, or loading and storing values in the evaluation context.
#[derive(Clone, PartialEq, Debug)]
pub enum ShyValue {
    /// A scalar value
    Scalar(ShyScalar),

    /// A vector value
    Vector(Vec<ShyScalar>),

    /// Name of a variable in the context to be read from or written to.
    Variable(String),

    /// A variable name followed by a series of nested property references
    PropertyChain(Vec<String>),

    /// Name of a function in the context to be called.
    FunctionName(String),

    /// An object which has properties that you can get and set. 
    Object(ShyObject)
}
const TRUE_STRING: &str = "True";
const FALSE_STRING: &str = "False";
impl PartialOrd for ShyValue {

    fn partial_cmp(&self, right_operand: &Self) -> Option<Ordering> {
        let t = &TRUE_STRING.to_string();
        let f = &FALSE_STRING.to_string();
        match (self, right_operand) {
            // Floating point comparison
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Rational(right))) 
                => Some(left.partial_cmp(right).unwrap_or(Ordering::Less)),

            // Floating point to integer comparison
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Rational(right))) 
                => Some((*left as f64).partial_cmp(right).unwrap_or(Ordering::Less)),
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Integer(right))) 
                => Some(left.partial_cmp(&(*right as f64)).unwrap_or(Ordering::Less)),

            // Integer comparison
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Integer(right))) => Some(left.cmp(right)),

            // String comparison
            (ShyValue::Scalar(ShyScalar::String(left)), ShyValue::Scalar(ShyScalar::String(right))) => Some(left.cmp(right)),

            // Bool comparison
            (ShyValue::Scalar(ShyScalar::Boolean(left)), ShyValue::Scalar(ShyScalar::Boolean(right))) => Some(left.cmp(right)),

            // Bool to String comparison - assume false is "False" and true is "True"
            (ShyValue::Scalar(ShyScalar::Boolean(left)), ShyValue::Scalar(ShyScalar::String(right))) 
                => Some(if *left { t.cmp(right) } else { f.cmp(right) } ),
            (ShyValue::Scalar(ShyScalar::String(left)), ShyValue::Scalar(ShyScalar::Boolean(right))) 
                => Some(left.cmp( if *right { t } else { f } )),

            // Bool to integer comparison - assume false is zero and true is one.
            (ShyValue::Scalar(ShyScalar::Boolean(left)), ShyValue::Scalar(ShyScalar::Integer(right))) 
                => Some(if *left { 1_i64.cmp(right) } else { 0_i64.cmp(right) } ),
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Boolean(right))) 
                => Some(left.cmp( if *right { &1_i64 } else { &0_i64 } )),

            (ShyValue::Object(obj1), ShyValue::Object(obj2)) 
                => if obj1 == obj2 { Some(Ordering::Equal) } else { None }

            _ => None
        }
    }
}

impl<'a> From<ParserToken> for ShyValue {
    fn from(parser_token: ParserToken) -> Self {
        match parser_token {
            ParserToken::Function(s) => ShyValue::FunctionName(s),
            ParserToken::Identifier(ref s) if *s == "true"  => ShyValue::Scalar(ShyScalar::Boolean(true)),
            ParserToken::Identifier(ref s) if *s == "false" => ShyValue::Scalar(ShyScalar::Boolean(false)),
            ParserToken::Identifier(s) => ShyValue::Variable(s),
            ParserToken::PropertyChain(s) => ShyValue::PropertyChain(s),
            ParserToken::Integer(s) => ShyValue::Scalar(ShyScalar::Integer(s.parse::<i64>().unwrap())),
            ParserToken::Rational(s) => ShyValue::Scalar(ShyScalar::Rational(s.parse::<f64>().unwrap())),
            ParserToken::StringLiteral(s) => ShyValue::Scalar(ShyScalar::String(s)),

            // Two tokens will be made from a PowerOp, an operator and this scalar value
            ParserToken::PowerOp(s) => ShyValue::Scalar(ShyScalar::Integer(s.parse::<i64>().unwrap())),

            // TODO: Create ShyScalar::Regex to use in place of String.
            ParserToken::Regex(s) => ShyValue::Scalar(ShyScalar::String(s)),
            _ => ShyValue::error(format!("Error parsing token '{}'", parser_token))
        }
    }
}

impl ShyValue {

    /// Clone all parts of the ShyValue except any ShyValue::Object parts, which will be shallow cloned.
    /// This permits multiple viewers of the data in the ShyObject, permitting it to be used a a context for expressions. 
    pub fn shallow_clone(&self) -> Self {
        match self {
            ShyValue::Object(child_obj) => ShyValue::Object(child_obj.shallow_clone()),
            _ => self.clone()
        }        
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            ShyValue::FunctionName(_) => "FunctionName",
            ShyValue::Variable(_) => "Variable",
            ShyValue::PropertyChain(_) => "PropertyChain",
            ShyValue::Vector(_) => "Vector",
            ShyValue::Object(_) => "Object",
            ShyValue::Scalar(ShyScalar::Boolean(_)) => "Boolean",
            ShyValue::Scalar(ShyScalar::Integer(_)) => "Integer",
            ShyValue::Scalar(ShyScalar::Rational(_)) => "Rational",
            ShyValue::Scalar(ShyScalar::String(_)) => "String",
            ShyValue::Scalar(ShyScalar::Error(_)) => "Error",
        }
    }

    pub fn empty() -> Self {
        ShyValue::Object(ShyObject::empty())
    }

    //..................................................................

    // Convenience methods that create or check error messages

    pub fn error(message: String) -> Self {
        ShyValue::Scalar(ShyScalar::Error(message))
    }

    pub fn is_error(&self) -> bool {
        match self {
            ShyValue::Scalar(ShyScalar::Error(_)) => true,
            _ => false
        }
    }

    /// Asserts that the two operands are incompatible when used with the given binary operator
    /// and formats an appropriate error message.
    fn incompatible(left: &Self, right: &Self, operator_name: &str) -> Self {
        ShyValue::error(format!("Operands for {} operator have incompatible types {} and {}", operator_name, left.type_name(), right.type_name()))
    }

    fn out_of_range(left: &Self, operator_name: &str) -> Self {
        ShyValue::error(format!("Operand for {} operator has {} value {:?} that is out of range", operator_name, left.type_name(), left))
    }

    fn not_an_object(&self) -> Self {
        ShyValue::error(format!("Cannot get or set properties on a non-object of type {}", self.type_name()))
    }

    fn invalid_property(property_name: &str) -> Self {
        ShyValue::error(format!("No such property '{}'", property_name))
    }

    fn invalid_path(path: &Vec<String>) -> Self {
        ShyValue::error(format!("Invalid or uninitialized property chain '{}'", path.join(".")))
    }

    fn incomparable_types() -> Self {
        ShyValue::error("Incomparable types".to_string())
    }

    fn cannot_infer_previous_value(path: &Vec<String>) -> Self {
        ShyValue::error(format!("Cannot infer previous value for property chain '{}'", path.join(".")))
    }


    //..................................................................

    // Get and set properties (applies only to ShyValue::Object)

    /// Get the value of a ShyValue::Object's property.
    ///   On Success, return the value wrapped as Ok(value).
    ///   On failure, return an error (a ShyValue::Scalar(ShyScalar::Error)) wrapped as Err(err).
    pub fn get(&self, property_name: &'static str) -> Result<ShyValue,ShyValue> {
        match self {
            ShyValue::Object(obj) => match obj.as_deref().get(property_name) {
                Some(value) => Result::Ok(value.shallow_clone()),
                _ => Result::Err(ShyValue::invalid_property(property_name))
            },
            _ => Result::Err(self.not_an_object())
        }
    }

    /// Set a property of the object to a new value.
    ///   On success, null is returned wrapped as Ok.
    ///   If the property may not be set or the ShyValue is not a ShyValue::Object, return an error (a ShyValue::Scalar(ShyScalar::Error)).
    pub fn set(&self, property_name: &'static str, property_value: ShyValue) -> Result<(), Self> {
        match self {
            ShyValue::Object(obj) => {
                let set_returned_a_value =
                    match obj.as_deref_mut().set(property_name, property_value) {
                        Some(_) => true,
                        None => false
                    };
                if set_returned_a_value { Result::Ok(()) }
                else {
                    // No value returned can mean that set failed or that there was no previous value to return.
                    // Check to see if there is now a value.
                    match obj.as_deref().can_get_property(property_name) {
                        true => Result::Ok(()),
                        false => Result::Err(ShyValue::invalid_property(property_name))
                    }
                }
            },
            _ => Result::Err(self.not_an_object())
        }
    }

    /// Safe Object property navigation.
    /// Attempt to regard self as a ShyValue::Object and get the value of the property corresponding to the given key.
    /// If an object does not have the given property, a ShyScalar::Error is returned.
    /// If a ShyValue::Object is returned, a shallow_clone is created, so that the underlying ShyAssociation is shared.
    pub fn get_safe(&self, key: &str) -> ShyValue {
        match self {
            ShyValue::Object(obj) => { 
                let deref = obj.as_deref();
                if !deref.can_get_property(key) {
                    ShyValue::invalid_property(key)
                }
                else {
                    match deref.get(key) {
                        Some(value) => value.shallow_clone(),
                        _ => ShyValue::invalid_property(key)
                    }
                }
            }
            _ => ShyValue::invalid_property(key)
        }
    }

    /// Given a series of property names, recursively perform a series of gets to obtain the value at the end of the chain.
    pub fn get_chain(&self, keys: &[String]) -> ShyValue {
        match keys.first() {
            Some(key) => {
                let next_value = self.get_safe(key);
                next_value.get_chain(&keys[1..])
            },
            None => self.shallow_clone()
        }
    }

    //..................................................................

    // Checks for special values: is_nan, is_false, is_true, is_falsey, is_truthy, is_number, is_zero

    pub fn is_nan(&self) -> bool {
        if let ShyValue::Scalar(ShyScalar::Rational(value)) = self { value.is_nan() }
        else { false }
    }

    pub fn is_false(&self) -> bool {
        if let ShyValue::Scalar(ShyScalar::Boolean(value)) = self { !value }
        else { false }
    }

    pub fn is_true(&self) -> bool {
        if let ShyValue::Scalar(ShyScalar::Boolean(value)) = self { *value }
        else { false }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            ShyValue::Scalar(ShyScalar::Boolean(value)) => *value,
            ShyValue::Scalar(ShyScalar::Integer(value)) => *value != 0,
            ShyValue::Scalar(ShyScalar::Rational(value)) => *value != 0.0,
            ShyValue::Scalar(ShyScalar::String(value)) => is_truthy(value),
            _ => false
        }
    }

    pub fn is_falsey(&self) -> bool {
        !self.is_truthy()
    }

    pub fn is_number(&self) -> bool {
        match self {
            ShyValue::Scalar(ShyScalar::Integer(_)) => true,
            ShyValue::Scalar(ShyScalar::Rational(value)) => !value.is_nan(),
            _ => false
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            ShyValue::Scalar(ShyScalar::Integer(value)) => *value == 0i64,
            ShyValue::Scalar(ShyScalar::Rational(value)) => *value == 0.0f64,
            _ => false
        }
    }

    //..................................................................

    // Arithmetic Operators (add, subtract, multiply, divide, modulo, power, sqrt, factorial)

    // Methods to perform operations
    // Note: They will not load a Variable value from the context. Caller must take care of that first.

    /// Add two ShyValues.
    pub fn add(left_operand: &Self, right_operand: &Self) -> Self {
        match (left_operand, right_operand) {
            // Floating point addition (with optional cast of integer to float)
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Rational(right))) => (left + right).into(),
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Rational(right))) => (*left as f64 + right).into(),
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Integer(right))) => (left + *right as f64).into(),

            // Integer addition
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Integer(right))) => (left + right).into(),

            // String concatenation
            (ShyValue::Scalar(ShyScalar::String(left)), ShyValue::Scalar(ShyScalar::String(right))) => format!("{}{}", left , right).into(),

            _ => ShyValue::incompatible(left_operand, right_operand, "add")
        }
    }

    /// Subtract two ShyValues.
    pub fn subtract(left_operand: &Self, right_operand: &Self) -> Self {
        match (left_operand, right_operand) {
            // Floating point subtraction (with optional cast of integer to float)
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Rational(right))) => (left - right).into(),
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Rational(right))) => (*left as f64 - right).into(),
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Integer(right))) => (left - *right as f64).into(),

            // Integer subtraction
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Integer(right))) => (left - right).into(),

            _ => ShyValue::incompatible(left_operand, right_operand, "subtract")
        }
    }

    /// Multiply two ShyValues.
    pub fn multiply(left_operand: &Self, right_operand: &Self) -> Self {
        match (left_operand, right_operand) {
            // Floating point multiplication (with optional cast of integer to float)
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Rational(right))) => (left * right).into(),
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Rational(right))) => (*left as f64 * right).into(),
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Integer(right))) => (left * *right as f64).into(),

            // Integer multiplication
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Integer(right))) => (left * right).into(),

            // String replication
            (ShyValue::Scalar(ShyScalar::String(left)), ShyValue::Scalar(ShyScalar::Integer(right))) => { 
                let mut s = String::new();
                for _index in 1..=*right {
                    s.push_str(left);
                }
                s.into()
            },

            _ => ShyValue::incompatible(left_operand, right_operand, "multiply")
        }
    }

    /// Divide two ShyValues.
    pub fn divide(left_operand: &Self, right_operand: &Self) -> Self {
        match (left_operand, right_operand) {
            // Floating point division (with cast of integer to float)
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Rational(right))) => (left / right).into(),
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Rational(right))) => (*left as f64 / right).into(),
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Integer(right))) => (left / *right as f64).into(),

            // Integers are divided using floating point division
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Integer(right))) => (*left as f64 / *right as f64).into(),

            _ => ShyValue::incompatible(left_operand, right_operand, "divide")
        }
    }

    /// Divide one ShyValue modulo a second ShyValue.
    pub fn modulo(left_operand: &Self, right_operand: &Self) -> Self {
        match (left_operand, right_operand) {
            // Floating point mod (with cast of integer to float)
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Rational(right))) => (left % right).into(),
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Rational(right))) => (*left as f64 % right).into(),
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Integer(right))) => (left % *right as f64).into(),

            // Integers use integer modular division
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Integer(right))) => (*left % *right).into(),

            _ => ShyValue::incompatible(left_operand, right_operand, "modulo")
        }
    }

    /// Exponentiation operator (any base). 
    pub fn power(left_operand: &Self, right_operand: &Self) -> Self {
        match (left_operand, right_operand) {
            // Floating point exponentiation (with cast of integer to float)
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Rational(right))) => left.powf(*right).into(),
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Rational(right))) => (*left as f64).powf(*right).into(),
            (ShyValue::Scalar(ShyScalar::Rational(left)), ShyValue::Scalar(ShyScalar::Integer(right))) 
                => {
                    if let Ok(ipower) = i32::try_from(*right) {
                        return left.powi(ipower).into();
                    }
                    left.powf(*right as f64).into()
                },

            // Integers use pow or powi when possible
            (ShyValue::Scalar(ShyScalar::Integer(left)), ShyValue::Scalar(ShyScalar::Integer(right)))
                => {
                    if let Ok(upower) = u32::try_from(*right) {
                        // Integer raised to non-negative integer power. Return an Integer.
                        return left.pow(upower).into();
                    }
                    if let Ok(ipower) = i32::try_from(*right) {
                        // Integer possibly raised to negative integer power. Return a Rational.
                        return (*left as f64).powi(ipower).into();
                    }
                    (*left as f64).powf(*right as f64).into()
                },

            _ => ShyValue::incompatible(left_operand, right_operand, "power")
        }
    }

    /// Square root operator.
    pub fn sqrt(left_operand: &Self) -> Self {
        Self::power(left_operand, &0.5.into())
    }
 
    /// Factorial operator.
    pub fn factorial(left_operand: &Self) -> Self {
        match left_operand {
            ShyValue::Scalar(ShyScalar::Integer(value)) if *value <= 20 => {
                match factorial(*value) {
                    Some(fact) => fact.into(),
                    _ => ShyValue::out_of_range(left_operand, "factorial")
                }
            },
            ShyValue::Scalar(ShyScalar::Integer(value)) if *value > 20 => {
                match factorial_approx(*value) {
                    Some(fact) => fact.into(),
                    _ => ShyValue::out_of_range(left_operand, "factorial")
                }
            },
            ShyValue::Scalar(ShyScalar::Rational(value)) if value.fract() == 0.0 => {
                match factorial(*value as i64) {
                    Some(fact) => fact.into(),
                    _ => ShyValue::out_of_range(left_operand, "factorial")
                }
            },
            _ => ShyValue::out_of_range(left_operand, "factorial")
        }
    }

    //..................................................................

    // Logical and Relational Operators: and, or, not, less_than, less_than_or_equal_to, greater_than, greater_than_or_equal_to, equals, not_equals

    /// Logical AND of two ShyValues.
    pub fn and(left_operand: &Self, right_operand: &Self) -> Self {
        // If some operands are errors, propagate the error, unless the
        // non-error operand is sufficient to determine the truth or falsehood of the expression.
        if left_operand.is_error() { 
            if right_operand.is_error() { return left_operand.clone(); }
            if right_operand.is_falsey() { return false.into(); }
            return left_operand.clone(); 
        }
        if left_operand.is_falsey() { return false.into(); }
        if right_operand.is_error() { return right_operand.clone(); }
        right_operand.is_truthy().into()
    }

    /// Logical OR of two ShyValues.
    pub fn or(left_operand: &Self, right_operand: &Self) -> Self {
        if left_operand.is_error() { 
            if right_operand.is_error() { return left_operand.clone(); }
            if right_operand.is_truthy() { return true.into(); }
            return left_operand.clone(); 
        }
        if left_operand.is_truthy() { return true.into(); }
        if right_operand.is_error() { return right_operand.clone(); }
        right_operand.is_truthy().into()
    }

    /// Logical NOT of one ShyValue.
    pub fn not(left_operand: &Self) -> Self {
        if left_operand.is_error() { 
            return left_operand.clone(); 
        }
        (left_operand.is_falsey()).into()
    }

    /// Less than operator for ShyValues.
    pub fn less_than(left_operand: &Self, right_operand: &Self) -> Self {
        match left_operand.partial_cmp(right_operand) {
            Some(Ordering::Less) => true.into(),
            Some(Ordering::Equal) => false.into(),
            Some(Ordering::Greater) => false.into(),
            None => ShyValue::incomparable_types()
        }
    }

    /// Less than or equal to operator for ShyValues.
    pub fn less_than_or_equal_to(left_operand: &Self, right_operand: &Self) -> Self {
        match left_operand.partial_cmp(right_operand) {
            Some(Ordering::Less) => true.into(),
            Some(Ordering::Equal) => true.into(),
            Some(Ordering::Greater) => false.into(),
            None => ShyValue::incomparable_types()
        }
    }

    /// Greater than operator for ShyValues.
    pub fn greater_than(left_operand: &Self, right_operand: &Self) -> Self {
        match left_operand.partial_cmp(right_operand) {
            Some(Ordering::Less) => false.into(),
            Some(Ordering::Equal) => false.into(),
            Some(Ordering::Greater) => true.into(),
            None => ShyValue::incomparable_types()
        }
    }

    /// Greater than operator for ShyValues.
    pub fn greater_than_or_equal_to(left_operand: &Self, right_operand: &Self) -> Self {
        match left_operand.partial_cmp(right_operand) {
            Some(Ordering::Less) => false.into(),
            Some(Ordering::Equal) => true.into(),
            Some(Ordering::Greater) => true.into(),
            None => ShyValue::incomparable_types()
        }
    }

    /// Equals operator for ShyValues.
    pub fn equals(left_operand: &Self, right_operand: &Self) -> Self {
        if left_operand == right_operand { true.into() }
        else { false.into() }
    }

    /// Not equals operator for ShyValues.
    pub fn not_equals(left_operand: &Self, right_operand: &Self) -> Self {
        if left_operand == right_operand { false.into() }
        else { true.into() }
    }

    //..................................................................

    // Load operator

    /// Load the value of a variable or a property chain from the context.
    /// Return an error (ShyValue::Error) if the left_operand is not a Variable 
    /// or no variable with the given name is found in the context.
    pub fn load(left_operand: &Self, ctx: &mut ExecutionContext) -> Self {
        match left_operand {
            ShyValue::Variable(name) => {
                match ctx.load(name) {
                    Some(value) => value,
                    None => Self::no_such_variable(name)
                }
            },
            ShyValue::PropertyChain(name_vec) => {
                match ctx.load_chain(name_vec) {
                    None => Self::bad_property_chain(name_vec),
                    Some(loaded_value) => loaded_value
                }
            },
            _ => Self::not_a_variable(left_operand)
        }
    }

    //..................................................................

    /*
       Assignment Operators: 
       
       assign (=)
       plus_assign (+=)
       minus_assign (-=)
       multiply_assign (*=)
       divide_assign (/=)
       mod_assign (%=)
       and_assign (&&=)
       or_assign (||=)
       post_increment (++)
       post_decrement (--)
    */ 

    fn not_a_variable(left_operand: &Self) -> Self {
        ShyValue::error(format!("Left operand must be a variable, not {}", left_operand.type_name()))
    }

    pub fn bad_property_chain(property_chain: &Vec<String>) -> Self {
        ShyValue::error(format!("Property chain includes an invalid property: {}", property_chain.join(".")))
    }

    fn no_such_variable(var_name: &String) -> Self {
        ShyValue::error(format!("No variable named {}", var_name))
    }
    
    pub fn assign(left_operand: &Self, right_operand: &Self, ctx: &mut ExecutionContext) -> Self {
        match left_operand {
            ShyValue::Variable(name) => {
                ctx.store(name, right_operand.clone());
                right_operand.clone()
            },
            ShyValue::PropertyChain(path) => {
                match ctx.store_chain(path, right_operand.clone()) {
                    Err(msg) => msg,
                    _ => right_operand.clone()
                }
            },
            _ => Self::not_a_variable(left_operand)
        }
    }

    /// Add the right_operand to the current value of the variable in the context (ctx) 
    /// referenced by the left_operand and store the result back in the context as the 
    /// new value for that variable.
    /// If the variable is not defined, initialize it to the value of the right_operand.
    pub fn plus_assign(left_operand: &Self, right_operand: &Self, ctx: &mut ExecutionContext) -> Self {
        Self::common_assign(left_operand, right_operand, ctx, &Self::add)
    }

    pub fn minus_assign(left_operand: &Self, right_operand: &Self, ctx: &mut ExecutionContext) -> Self {
        match left_operand {
            ShyValue::Variable(name) => {
                let current_value = ctx.load(name);
                match current_value {
                    Some(current) => {
                        let difference = ShyValue::subtract(&current, right_operand);
                        ctx.store(name, difference.clone());
                        difference
                    },
                    None => {
                        let negation = ShyValue::prefix_minus(right_operand);
                        ctx.store(name, negation.clone());
                        negation
                    }
                }
            },
            ShyValue::PropertyChain(path) => {
                ctx.property_chain_update(
                    path, 
                    &0.into(),
                    &|previous_value| ShyValue::subtract(previous_value, right_operand),
                    &|| ShyValue::prefix_minus(right_operand),
                    &|_previous_value, new_value| new_value.clone())
            },
            _ => Self::not_a_variable(left_operand)
        }
    }

    /// Multiply a value loaded from the context by the right_operand.
    /// If no value has yet been stored for that variable, set the value to the right_operand,
    /// as if the value was originally one.
    pub fn multiply_assign(left_operand: &Self, right_operand: &Self, ctx: &mut ExecutionContext) -> Self {
        Self::common_assign(left_operand, right_operand, ctx, &Self::multiply)
    }

    /// Divide a value loaded from the context by the right_operand.
    /// If no value has yet been stored for that variable, set the value to the inverse of the right_operand,
    /// as if the value was originally one.
    pub fn divide_assign(left_operand: &Self, right_operand: &Self, ctx: &mut ExecutionContext) -> Self {
        match left_operand {
            ShyValue::Variable(name) => {
                let current_value = ctx.load(name);
                match current_value {
                    Some(current) => {
                        let quotient = ShyValue::divide(&current, right_operand);
                        ctx.store(name, quotient.clone());
                        quotient
                    },
                    None => {
                        let quotient = ShyValue::divide(&1_i64.into(), right_operand);
                        ctx.store(name, quotient.clone());
                        quotient.clone()
                    }
                }
            },
            ShyValue::PropertyChain(path) => {
                ctx.property_chain_update(
                    path, 
                    &1.into(),
                    &|previous_value| ShyValue::divide(previous_value, right_operand),
                    &|| ShyValue::divide(&1_i64.into(), right_operand),
                    &|_previous_value, new_value| new_value.clone())
            },
            _ => Self::not_a_variable(left_operand)
        }
    }

    /// Perform the modular division of a value loaded from the context by the right_operand.
    /// If no value has yet been stored for that variable, return an error wrapped by a ShyValue.
    pub fn modulo_assign(left_operand: &Self, right_operand: &Self, ctx: &mut ExecutionContext) -> Self {
        match left_operand {
            ShyValue::Variable(name) => {
                let current_value = ctx.load(name);
                match current_value {
                    Some(current) => {
                        let remainder = ShyValue::modulo(&current, right_operand);
                        ctx.store(name, remainder.clone());
                        remainder
                    },
                    None => ShyValue::invalid_property(name)
                }
            },
            ShyValue::PropertyChain(path) => {
                ctx.property_chain_update(
                    path, 
                    &ShyValue::cannot_infer_previous_value(path),
                    &|previous_value| ShyValue::modulo(previous_value, right_operand),
                    &|| ShyValue::invalid_path(path),
                    &|_previous_value, new_value| new_value.clone())
            },

            _ => Self::not_a_variable(left_operand)
        }
    }

    /// Load a value from the context corresponding to the variable named by the left_operand,
    /// then perform a logical "and" operation between that value and the right_operand.
    /// Store the result in the context for the same variable.
    /// If the variable is not initially defined, initialize it to the value of the right_operand.
    pub fn and_assign(left_operand: &Self, right_operand: &Self, ctx: &mut ExecutionContext) -> Self {
        Self::common_assign(left_operand, right_operand, ctx, &Self::and)
    }

    /// Load a value from the context corresponding to the variable named by the left_operand,
    /// then perform a logical "or" operation between that value and the right_operand.
    /// Store the result in the context for the same variable.
    /// If the variable is not initially defined, initialize it to the value of the right_operand.
    pub fn or_assign(left_operand: &Self, right_operand: &Self, ctx: &mut ExecutionContext) -> Self {
        Self::common_assign(left_operand, right_operand, ctx, &Self::or)
    }

    /// Increment by one the value loaded from the context for the variable indicated by left_operand,
    /// store the incremented value back in the context under the same variable,
    /// and return the value it held prior to the increment operation.
    /// If the variable was previously uninitialized, pretend that it started with the value zero,
    /// return zero, and store a one.
    pub fn post_increment(left_operand: &Self, ctx: &mut ExecutionContext) -> Self {
        match left_operand {
            ShyValue::Variable(name) => {
                let current_value = ctx.load(name);
                match current_value {
                    Some(current) => {
                        let result = ShyValue::add(&current, &1.into());
                        ctx.store(name, result.clone());
                        current
                    },
                    None => {
                        ctx.store(name, 1.into());
                        0.into()
                    }
                }
            },
            ShyValue::PropertyChain(path) => {
                ctx.property_chain_update(
                    path, 
                    &0.into(),
                    &|previous_value| ShyValue::add(previous_value, &1.into()),
                    &|| ShyValue::invalid_path(path),
                    &|previous_value, _new_value| previous_value.clone())
            },
            _ => Self::not_a_variable(left_operand)
        }
    }

    /// Decrement by one the value loaded from the context for the variable indicated by left_operand,
    /// store the decremented value back in the context under the same variable,
    /// and return the value it held prior to the decrement operation.
    /// If the variable was previously uninitialized, return an error wrapped in a ShyValue.
    pub fn post_decrement(left_operand: &Self, ctx: &mut ExecutionContext) -> Self {
        match left_operand {
            ShyValue::Variable(name) => {
                let current_value = ctx.load(name);
                match current_value {
                    Some(current) => {
                        let result = ShyValue::subtract(&current, &1.into());
                        ctx.store(name, result.clone());
                        current
                    },
                    None => {
                        let result = ShyValue::error(format!("Uninitialized variable {}", name));
                        ctx.store(name, result.clone());
                        result
                    }
                }
            },
            ShyValue::PropertyChain(path) => {
                ctx.property_chain_update(
                    path, 
                    &0.into(),
                    &|previous_value| ShyValue::subtract(previous_value, &1.into()),
                    &|| ShyValue::invalid_path(path),
                    &|previous_value, _new_value| previous_value.clone())
            },

            _ => Self::not_a_variable(left_operand)
        }
    }

    /// Load a value from the context corresponding to the variable named by the left_operand,
    /// then perform "op" (a given operation) between that value and the right_operand.
    /// Store the result in the context for the same variable.
    /// If the variable is not initially defined, initialize it to the value of the right_operand.
    fn common_assign(left_operand: &Self, right_operand: &Self, ctx: &mut ExecutionContext, 
        op: &dyn Fn(&Self, &Self) -> Self) -> Self {
        match left_operand {
            ShyValue::Variable(name) => {
                let current_value = ctx.load(name);
                match current_value {
                    Some(current) => {
                        let result = op(&current, right_operand);
                        ctx.store(name, result.clone());
                        result
                    },
                    None => {
                        ctx.store(name, right_operand.clone());
                        right_operand.clone()
                    }
                }
            },
            ShyValue::PropertyChain(path) => {
                ctx.property_chain_update(
                    path, 
                    &ShyValue::cannot_infer_previous_value(path),
                    &|previous_value| op(previous_value, right_operand),
                    &|| ShyValue::invalid_path(path),
                    &|_previous_value, new_value| new_value.clone())
            },
            _ => Self::not_a_variable(left_operand)
        }
    }

    //..................................................................

    // Miscellaneous Operators: call, comma, member, prefix_plus, prefix_minus, matches, not_matches, ternary

    /*
        OpenBracket,
        CloseBracket,
    */

    /// call operator - performs a function call.
    pub fn call(left_operand: &Self, right_operand: &Self, ctx: &mut ExecutionContext) -> Self {
        match left_operand {
            ShyValue::FunctionName(name) => ctx.call(name.clone(), right_operand.clone()),
            _ => ShyValue::error(format!("Cannot call a function because the name is type {}", left_operand.type_name()))
        }
    }

    /// Comma operator for ShyValues (combines arguments into a list).
    /// The right_operand must be a ShyValue::Scalar.
    /// If the left_operand is not a ShyValue::Vector, return a ShyValue::Vector containing both operands.
    /// If the left_operand is a ShyValue::Vector, append a clone of the right_operand to a clone of that Vector.
    /// Return a new Vector.
    pub fn comma(left_operand: &Self, right_operand: &Self) -> Self {
        match (left_operand, right_operand) {
            (ShyValue::Vector(v), ShyValue::Scalar(right_scalar)) => {
                let mut v_clone = v.clone();
                v_clone.push(right_scalar.clone());
                ShyValue::Vector(v_clone)
            } ,
            (ShyValue::Scalar(left_scalar), ShyValue::Scalar(right_scalar)) => {
                ShyValue::Vector(vec![left_scalar.clone(), right_scalar.clone()])
            },
            _ => ShyValue::error(
                format!("wrong type of arguments for comma operator: {} and {}", 
                    left_operand.type_name(), 
                    right_operand.type_name()))
        }
    }

    /// Prefix plus of one ShyValue.
    pub fn prefix_plus(left_operand: &Self) -> Self {
        left_operand.clone()
    }

    /// Prefix minus of one ShyValue.
    pub fn prefix_minus(left_operand: &Self) -> Self {
        match *left_operand {
            ShyValue::Scalar(ShyScalar::Integer(i)) => (-i).into(),
            ShyValue::Scalar(ShyScalar::Rational(r)) => (-r).into(),
            ShyValue::Scalar(ShyScalar::Boolean(b)) => (!b).into(),
            _ => ShyValue::error("cannot negate a non-number".to_string())
        }
    }

    /// Regex matching operator.
    pub fn matches(left_operand: &Self, right_operand: &Self) -> Self {
        match (left_operand, right_operand) {
            (ShyValue::Scalar(ShyScalar::String(s)), ShyValue::Scalar(ShyScalar::String(regex_string))) => {
                match Regex::new(regex_string) {
                    Ok(regex) => {
                        regex.is_match(s).into()
                    },
                    Err(_) => ShyValue::error(format!("malformed regular expression {}", regex_string))
                }
            } ,
            _ => ShyValue::error(
                format!("wrong type of arguments for matches operator: {} and {}", 
                    left_operand.type_name(), 
                    right_operand.type_name()))
        }
    }

    /// Regex matching operator.
    pub fn not_matches(left_operand: &Self, right_operand: &Self) -> Self {
        ShyValue::not(&ShyValue::matches(left_operand, right_operand))
    }
}

// Conversions from basic types to ShyValue

impl From<f64> for ShyValue { fn from(x: f64) -> Self { ShyValue::Scalar(ShyScalar::Rational(x)) } }
impl From<&f64> for ShyValue { fn from(x: &f64) -> Self { ShyValue::Scalar(ShyScalar::Rational(*x)) } }
impl From<i64> for ShyValue { fn from(x: i64) -> Self { ShyValue::Scalar(ShyScalar::Integer(x)) } }
impl From<&i64> for ShyValue { fn from(x: &i64) -> Self { ShyValue::Scalar(ShyScalar::Integer(*x)) } }
impl From<i32> for ShyValue { fn from(x: i32) -> Self { ShyValue::Scalar(ShyScalar::Integer(x as i64)) } }
impl From<&i32> for ShyValue { fn from(x: &i32) -> Self { ShyValue::Scalar(ShyScalar::Integer(*x as i64)) } }
impl From<bool> for ShyValue { fn from(x: bool) -> Self { ShyValue::Scalar(ShyScalar::Boolean(x)) } }
impl From<&bool> for ShyValue { fn from(x: &bool) -> Self { ShyValue::Scalar(ShyScalar::Boolean(*x)) } }
impl From<String> for ShyValue { fn from(s: String) -> Self { ShyValue::Scalar(ShyScalar::String(s.clone())) } }
impl From<&str> for ShyValue { fn from(s: &str) -> Self { ShyValue::Scalar(ShyScalar::String(s.to_string())) } }


//..................................................................

/// ShyToken represents the tokens on the Output stack of the Shunting Yard Algorithm.
///   - The Value and Operator variants will appear on the Output stack. 
///   - The None value is for error processing.
///   - The OperatorWithValue (used for Functions and Power) will be split into 
///     a Value token (the Function name) and an Operator token (the function invocation).
#[derive(Clone, PartialEq, Debug)]
pub enum ShyToken{
    Value(ShyValue),
    Operator(ShyOperator),
    OperatorWithValue(ShyOperator, ShyValue),
    Error,
    None
}

impl ShyToken{
    pub fn is_error(&self) -> bool {
        discriminant(&ShyToken::Error) == discriminant(self)
    }
}

/// Convert a ParserToken into a ShyToken.
impl<'a> From<ParserToken> for ShyToken{
    fn from(parser_token: ParserToken) -> Self {
        let op: ShyOperator = parser_token.clone().into();
        match op {
            ShyOperator::Operand => {
                let val: ShyValue = parser_token.into();
                ShyToken::Value(val)
            },
            // Function calls require that we put the function name on the value stack
            // and the FunctionCall operator on the operator stack.
            ShyOperator::FunctionCall => {
                let val: ShyValue = parser_token.into();
                ShyToken::OperatorWithValue(ShyOperator::FunctionCall, val)
            },
            // A Power will become two tokens, Exponentiation and the numeric value of the exponent
            ShyOperator::Power => {
                let val: ShyValue = parser_token.into();
                ShyToken::OperatorWithValue(ShyOperator::Exponentiation, val)
            }

            ShyOperator::Error => ShyToken::Error,
            _ => ShyToken::Operator(op)
        }
    }
}

impl From<ShyOperator> for ShyToken {
    fn from(op: ShyOperator) -> Self {
        ShyToken::Operator(op)
    }
}

//..................................................................

#[cfg(test)]
/// Tests of the ShyOperator, ShyToken, and ShyValue.
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    #[test]
    /// Verify that the correct operator precedence is returned.
    fn operator_precedence() {
        assert_that!(ShyOperator::Or.precedence()).is_equal_to(4);
    }

    #[test]
    /// Verify that the correct operator name is returned.
    fn operator_name() {
        assert_that!(ShyOperator::Multiply.to_string()).is_equal_to("Multiply".to_string());
    }

    #[test]
    /// Verify that converting from a ParserToken to a ShyOperator works.
    fn from_parser_token_to_shy_operator() {
        let pt_multiply = ParserToken::MultiplicativeOp("*".to_string());
        let so_multiply : ShyOperator = pt_multiply.into(); 
        assert_that!(so_multiply).is_equal_to(ShyOperator::Multiply);
    }

    #[test]
    /// Verify that a Boolean ShyScalar is parsed from a ParserToken.
    fn parse_boolean() {
        let shy_token: ShyToken = ParserToken::Identifier("true".to_string()).into();
        match shy_token {
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Boolean(true))) => assert!(true),
            _ => assert!(false)
        };
    }

    #[test]
    /// Verify that an Integer ShyScalar is parsed from a ParserToken.
    fn parse_integer() {
        let shy_token: ShyToken = ParserToken::Integer("987654321".to_string()).into();
        match shy_token {
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(987654321))) => assert!(true),
            _ => assert!(false)
        };
    }

    #[test]
    /// Verify that a Rational ShyScalar is parsed from a ParserToken.
    fn parse_rational() {
        let shy_token: ShyToken = ParserToken::Rational("1.962315e+3".to_string()).into();
        match shy_token {
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Rational(ref value))) => assert_that(value).is_close_to(1962.315, 0.000001),
            _ => assert!(false)
        };
    }

    #[test]
    /// Verify that a ShyOperator::Multiply is parsed from a ParserToken.
    fn parse_operator_multiply() {
        let shy_token: ShyToken = ParserToken::MultiplicativeOp("*".to_string()).into();
        match shy_token {
            ShyToken::Operator(ShyOperator::Multiply) => assert!(true),
            _ => assert!(false)
        };
    }

    #[test]
    /// Verify that a ShyOperator::FunctionCall and a ShyValue::FunctionName is parsed from a ParserToken.
    fn parse_operator_function() {
        let shy_token: ShyToken = ParserToken::Function("sin".to_string()).into();
        match shy_token {
            ShyToken::OperatorWithValue(ShyOperator::FunctionCall, ShyValue::FunctionName(ref func_name)) if *func_name == "sin" => assert!(true),
            _ => assert!(false)
        };
    }

    #[test]
    /// Verify that a ShyOperator::NotEquals is parsed from a ParserToken.
    fn parse_not_equals() {
        let mut shy_token: ShyToken = ParserToken::EqualityOp("!=".to_string()).into();
        match shy_token {
            ShyToken::Operator(ShyOperator::NotEquals) => assert!(true),
            _ => assert!(false)
        };

        shy_token = ParserToken::EqualityOp("≠".to_string()).into();
        match shy_token {
            ShyToken::Operator(ShyOperator::NotEquals) => assert!(true),
            _ => assert!(false)
        };
    }

    #[test]
    /// Adding ShyValues.
    fn shyvalue_add() {
        binary_operator_test(&2.5.into(), &3.5.into(), &6.0.into(), &ShyValue::add);
        binary_operator_test(&10.into(),  &2.into(),   &12.into(),  &ShyValue::add);
        binary_operator_test(&10.into(), &2.5.into(), &12.5.into(), &ShyValue::add);
        binary_operator_test(&"Hello ".into(), &"World".into(), &"Hello World".into(), &ShyValue::add);
        assert!( &ShyValue::add(&true.into(), &3.5.into()).is_error());
    }

    #[test]
    /// Subtracting ShyValues.
    fn shyvalue_subtract() {
        binary_operator_test(&2.5.into(), &3.5.into(), &(-1.0).into(), &ShyValue::subtract);
        binary_operator_test(&10.into(),  &2.into(),   &8.into(),  &ShyValue::subtract);
        binary_operator_test(&10.into(), &2.5.into(), &7.5.into(), &ShyValue::subtract);
        assert!( &ShyValue::subtract(&true.into(), &3.5.into()).is_error());
    }

    #[test]
    /// Multiplying ShyValues.
    fn shyvalue_multiply() {
        binary_operator_test(&2.5.into(), &3.5.into(), &8.75.into(), &ShyValue::multiply);
        binary_operator_test(&10.into(),  &2.into(),   &20.into(),  &ShyValue::multiply);
        binary_operator_test(&10.into(), &2.5.into(), &25.0.into(), &ShyValue::multiply);
        binary_operator_test(&"la".into(), &3.into(), &"lalala".into(), &ShyValue::multiply);
        assert!( &ShyValue::multiply(&true.into(), &3.5.into()).is_error());
    }

    #[test]
    /// Dividing ShyValues.
    fn shyvalue_divide() {
        binary_operator_test(&7.0.into(), &2.0.into(), &3.5.into(), &ShyValue::divide);
        binary_operator_test(&10.into(),  &2.into(),   &5.0.into(),  &ShyValue::divide);
        binary_operator_test(&12.0.into(), &3.into(), &4.0.into(), &ShyValue::divide);
        binary_operator_test(&1.into(), &0.into(), &f64::INFINITY.into(), &ShyValue::divide); // Divide by zero check
        assert!( &ShyValue::divide(&true.into(), &3.5.into()).is_error());
    }

    #[test]
    /// Modulo with ShyValues.
    fn shyvalue_modulo() {
        binary_operator_test(&7.0.into(), &2.0.into(), &1.0.into(), &ShyValue::modulo);
        binary_operator_test(&17.into(),  &5.into(),   &2.into(),  &ShyValue::modulo);
        binary_operator_test(&33.into(), &2.5.into(), &0.5.into(), &ShyValue::modulo);
        assert!( &ShyValue::modulo(&true.into(), &3.5.into()).is_error());
    }

    #[test]
    /// Exponentiating ShyValues (raise to a power).
    fn shyvalue_power() {
        binary_operator_test(&2.0.into(), &3.0.into(), &8.0.into(), &ShyValue::power);
        binary_operator_test(&10.into(),  &2.into(),   &100.into(),  &ShyValue::power);
        binary_operator_test(&16.into(),  &0.5.into(),   &4.0.into(),  &ShyValue::power);
        binary_operator_test(&10.0.into(), &(-2).into(), &0.01.into(), &ShyValue::power);
        assert!( &ShyValue::power(&true.into(), &3.5.into()).is_error());
    }

    #[test]
    /// Square root of a ShyValue.
    fn shyvalue_sqrt() {
        unary_operator_test(&4.into(), &2.0.into(), &ShyValue::sqrt);
        unary_operator_test(&16.0.into(),  &4.0.into(), &ShyValue::sqrt);

        // Since NaN does not equal NaN, a different test for square root of a negative number:
        assert!(&ShyValue::sqrt(&(-16).into()).is_nan());

        assert!(&ShyValue::sqrt(&true.into()).is_error());
    }

    #[test]
    /// Factorial of a ShyValue.
    fn shyvalue_factorial() {
        unary_operator_test(&1.into(), &1.into(), &ShyValue::factorial);
        unary_operator_test(&4.into(), &24.into(), &ShyValue::factorial);
        unary_operator_test(&5.0.into(), &120.into(), &ShyValue::factorial);
        assert!(&ShyValue::factorial(&21.0.into()).is_error());
    }

    #[test]
    /// Test is_truthy
    fn is_truthy_tests() {
        assert!(is_truthy("y"));
        assert!(is_truthy("yes"));
        assert!(is_truthy("Y"));
        assert!(is_truthy("YES"));
        assert!(is_truthy("Yes"));
        assert!(is_truthy("t"));
        assert!(is_truthy("T"));
        assert!(is_truthy("true"));
        assert!(is_truthy("True"));
        assert!(is_truthy("TRUE"));
        assert!(is_truthy("1"));
        assert!(!is_truthy("false"));
        assert!(!is_truthy("NO"));
    }

    #[test]
    /// Test is_falsey
    fn is_falsey_tests() {
        assert!(is_falsey("n"));
        assert!(is_falsey("no"));
        assert!(is_falsey("N"));
        assert!(is_falsey("NO"));
        assert!(is_falsey("f"));
        assert!(is_falsey("F"));
        assert!(is_falsey("false"));
        assert!(is_falsey("False"));
        assert!(is_falsey("FALSE"));
        assert!(is_falsey("0"));
        assert!(!is_falsey("T"));
    }

    #[test]
    /// Test logical and operator.
    fn shyvalue_and() {
        binary_operator_test(&true.into(), &true.into(), &true.into(), &ShyValue::and);
        binary_operator_test(&true.into(), &false.into(), &false.into(), &ShyValue::and);
        binary_operator_test(&false.into(), &true.into(), &false.into(), &ShyValue::and);
        binary_operator_test(&false.into(), &false.into(), &false.into(), &ShyValue::and);
        binary_operator_test(&1.into(), &2.into(), &true.into(), &ShyValue::and);
        binary_operator_test(&0.into(), &1.into(), &false.into(), &ShyValue::and);
        binary_operator_test(&"".into(), &1.into(), &false.into(), &ShyValue::and);

        // Despite an error argument, still able to conclude result is false.
        binary_operator_test(&ShyValue::error("An error".to_string()), &false.into(), &false.into(), &ShyValue::and);

        // Because of error argument, unable to determine truth or falsity.
        assert!(ShyValue::and(&ShyValue::error("An error".to_string()), &true.into()).is_error());
    }

    #[test]
    /// Test logical or operator.
    fn shyvalue_or() {
        binary_operator_test(&true.into(), &true.into(), &true.into(), &ShyValue::or);
        binary_operator_test(&true.into(), &false.into(), &true.into(), &ShyValue::or);
        binary_operator_test(&false.into(), &true.into(), &true.into(), &ShyValue::or);
        binary_operator_test(&false.into(), &false.into(), &false.into(), &ShyValue::or);
        binary_operator_test(&1.into(), &2.into(), &true.into(), &ShyValue::or);
        binary_operator_test(&0.into(), &1.into(), &true.into(), &ShyValue::or);
        binary_operator_test(&"".into(), &1.into(), &true.into(), &ShyValue::or);

        // Despite an error argument, still able to conclude result is false.
        binary_operator_test(&ShyValue::error("An error".to_string()), &true.into(), &true.into(), &ShyValue::or);

        // Because of error argument, unable to determine truth or falsity.
        assert!(ShyValue::or(&ShyValue::error("An error".to_string()), &false.into()).is_error());
    }

    #[test]
    /// Test logical not operator.
    fn shyvalue_not() {
        unary_operator_test(&false.into(), &true.into(), &ShyValue::not);
        unary_operator_test(&true.into(), &false.into(), &ShyValue::not);
        unary_operator_test(&1.into(), &false.into(), &ShyValue::not);
        unary_operator_test(&"".into(), &true.into(), &ShyValue::not);
        assert!(&ShyValue::not(&ShyValue::error("An error".to_string())).is_error());
    }

    #[test]
    /// Test less than operator.
    fn shyvalue_less_than() {
        binary_operator_test(&1.into(), &2.into(), &true.into(), &ShyValue::less_than);
        binary_operator_test(&4.5.into(), &4.into(), &false.into(), &ShyValue::less_than);
        binary_operator_test(&7.14.into(), &7.15.into(), &true.into(), &ShyValue::less_than);
    }

    #[test]
    /// Test greater than operator.
    fn shyvalue_greater_than() {
        binary_operator_test(&1.into(), &2.into(), &false.into(), &ShyValue::greater_than);
        binary_operator_test(&4.5.into(), &4.into(), &true.into(), &ShyValue::greater_than);
        binary_operator_test(&7.14.into(), &7.15.into(), &false.into(), &ShyValue::greater_than);
        binary_operator_test(&"Apple".into(), &"Adam".into(), &true.into(), &ShyValue::greater_than);
    }

    #[test]
    /// Test assign operator. 
    fn shyvalue_assign() {
        assignment_operator_test(
            &ShyValue::Variable("x".to_string()), 
            &10.into(), 
            &mut ExecutionContext::default(), 
            &10.into(), 
            &ShyValue::assign);
    }

    #[test]
    /// Test plus_assign operator. 
    fn shyvalue_plus_assign() {
        let mut ctx = ExecutionContext::default();
        let x = "x".to_string();
        ctx.store(&x, 10.into());
        assignment_operator_test(
            &ShyValue::Variable(x), 
            &1.into(), 
            &mut ctx, 
            &11.into(), 
            &ShyValue::plus_assign);
    }

    #[test]
    /// Test minus_assign operator. 
    fn shyvalue_minus_assign() {
        let mut ctx = ExecutionContext::default();
        let x = "x".to_string();
        ctx.store(&x, 10.5.into());
        assignment_operator_test(
            &ShyValue::Variable(x), 
            &1.into(), 
            &mut ctx, 
            &9.5.into(), 
            &ShyValue::minus_assign);
    }

    #[test]
    /// Test multiply_assign operator. 
    fn shyvalue_multiply_assign() {
        let mut ctx = ExecutionContext::default();
        let x = "x".to_string();
        ctx.store(&x, 6.into());
        assignment_operator_test(
            &ShyValue::Variable(x), 
            &7.into(), 
            &mut ctx, 
            &42.into(), 
            &ShyValue::multiply_assign);
    }

    #[test]
    /// Test divide_assign operator. 
    fn shyvalue_divide_assign() {
        let mut ctx = ExecutionContext::default();
        let x = "x".to_string();
        ctx.store(&x, 6.into());
        assignment_operator_test(
            &ShyValue::Variable(x), 
            &24.into(), 
            &mut ctx, 
            &0.25.into(), 
            &ShyValue::divide_assign);
    }

    #[test]
    /// Test modulo_assign operator. 
    fn shyvalue_modulo_assign() {
        let mut ctx = ExecutionContext::default();
        let x = "x".to_string();
        ctx.store(&x, 21.into());
        assignment_operator_test(
            &ShyValue::Variable(x), 
            &6.into(), 
            &mut ctx, 
            &3.into(), 
            &ShyValue::modulo_assign);
    }

    #[test]
    /// Test and_assign operator. 
    fn shyvalue_and_assign() {
        let mut ctx = ExecutionContext::default();
        let x = "x".to_string();
        let x_var = &ShyValue::Variable(x.clone());
        ctx.store(&x, true.into());
        assignment_operator_test(x_var, &true.into(), &mut ctx, &true.into(), &ShyValue::and_assign);
        assignment_operator_test(x_var, &false.into(), &mut ctx, &false.into(), &ShyValue::and_assign);
        assignment_operator_test(x_var, &true.into(), &mut ctx, &false.into(), &ShyValue::and_assign);
    }

    #[test]
    /// Test or_assign operator. 
    fn shyvalue_or_assign() {
        let mut ctx = ExecutionContext::default();
        let x = "x".to_string();
        let x_var = &ShyValue::Variable(x.clone());
        ctx.store(&x, false.into());
        assignment_operator_test(x_var, &false.into(), &mut ctx, &false.into(), &ShyValue::or_assign);
        assignment_operator_test(x_var, &true.into(), &mut ctx, &true.into(), &ShyValue::or_assign);
        assignment_operator_test(x_var, &false.into(), &mut ctx, &true.into(), &ShyValue::or_assign);
    }

    #[test]
    /// Test post_increment operator. 
    fn shyvalue_post_increment() {
        let mut ctx = ExecutionContext::default();
        let x = "x".to_string();
        let x_var = &ShyValue::Variable(x.clone());
        post_operator_test(x_var, &mut ctx, &0.into(), &1.into(), &ShyValue::post_increment);
        post_operator_test(x_var, &mut ctx, &1.into(), &2.into(), &ShyValue::post_increment);
        post_operator_test(x_var, &mut ctx, &2.into(), &3.into(), &ShyValue::post_increment);
    }

    #[test]
    /// Test post_decrement operator. 
    fn shyvalue_post_decrement() {
        let mut ctx = ExecutionContext::default();
        let x = "x".to_string();
        let x_var = &ShyValue::Variable(x.clone());
        ctx.store(&x, 3.into());
        post_operator_test(x_var, &mut ctx, &3.into(), &2.into(), &ShyValue::post_decrement);
        post_operator_test(x_var, &mut ctx, &2.into(), &1.into(), &ShyValue::post_decrement);
        post_operator_test(x_var, &mut ctx, &1.into(), &0.into(), &ShyValue::post_decrement);
    }

    #[test]
    /// Test comma operator.
    fn shyvalue_comma() {
        let a: ShyValue = 5.into();
        let b: ShyValue = 2.75.into();
        let ab = ShyValue::comma(&a, &b);
        match ab {
            ShyValue::Vector(v) => {
                if let &[ShyScalar::Integer(aa), ShyScalar::Rational(bb)] = &*v {
                    asserting("First operand").that(&aa).is_equal_to(5_i64);
                    asserting("Second operand").that(&bb).is_equal_to(2.75_f64);
                } else {
                    assert!(false, "Not a Vec with an integer and a rational element");
                }
            },
            _ => assert!(false, "Not a Vec")
        }
    }

    #[test]
    /// Test prefix minus operator.
    fn shyvalue_prefix_minus() {
        unary_operator_test(&1.into(), &(-1).into(), &ShyValue::prefix_minus);
        unary_operator_test(&true.into(), &false.into(), &ShyValue::prefix_minus);
        unary_operator_test(&(-11.11).into(), &11.11.into(), &ShyValue::prefix_minus);
        assert!(&ShyValue::prefix_minus(&ShyValue::error("An error".to_string())).is_error());
    }

    #[test]
    /// Test matches operator.
    fn shyvalue_matches() {
        binary_operator_test(&"Hello World".into(), &"el+o".into(), &true.into(), &ShyValue::matches);
        binary_operator_test(&"Hello World".into(), &"^e".into(), &false.into(), &ShyValue::matches);
    }   

    #[test]
    /// Create a nested ShyValue(Object) and verify that we can navigate to the bottom object.
    fn shyvalue_set_and_get() {
        let top_obj = ShyValue::empty();
        if let Result::Err(err) = top_obj.set("name", "Bob".into()) {
            assert!(false, format!("Cannot set name: {:?}", err));
        }
        if let Result::Err(err) = top_obj.set("address", ShyValue::empty()) {
            assert!(false, format!("Cannot set address: {:?}", err));
        }
        let expected_value: String = String::from("02180");
        if let Result::Err(err) = top_obj.get_safe("address").set("ZIP", expected_value.clone().into()) {
            assert!(false, format!("Cannot set ZIP: {:?}", err));
        }
        match top_obj.get_safe("address").get("ZIP") {
            Ok(ShyValue::Scalar(ShyScalar::String(ref actual_value))) => asserting("Retrieved ZIP code matches").that(actual_value).is_equal_to(&expected_value),
            _ => assert!(false, format!("Did not retrieve correct ZIP code from the address"))
        }
    }

    // ...................................................................................

    // Test helpers

    /// Test a binary operator (excluding assignment)
    fn binary_operator_test<'a>(left: &ShyValue, right: &ShyValue, expected: &ShyValue, op: &dyn Fn(&ShyValue, &ShyValue) -> ShyValue) {
        let actual = op(left, right);
        asserting(&format!("Operation on {:?} and {:?} should yield {:?}", left, right, expected)).that(&actual).is_equal_to(expected);
    }

    /// Test a unary operator
    fn unary_operator_test<'a>(left: &ShyValue, expected: &ShyValue, op: &dyn Fn(&ShyValue) -> ShyValue) {
        let actual = op(left);
        asserting(&format!("Operation on {:?} should yield {:?}", left, expected)).that(&actual).is_equal_to(expected);
    }

    fn assignment_operator_test<'a>(left: &ShyValue, right: &ShyValue, 
            ctx: &mut ExecutionContext, expected: &ShyValue, 
            op: &dyn Fn(&ShyValue, &ShyValue, &mut ExecutionContext) -> ShyValue) {
        let result = op(left, right, ctx);
        asserting("Return value matches").that(&result).is_equal_to(expected);
        match left {
            ShyValue::Variable(var_name) => {
                asserting("Stored value matches").that(&ctx.load(var_name).unwrap()).is_equal_to(expected);
            },
            _ => assert!(false, "left operand is not a variable")
        }
    }

    fn post_operator_test<'a>(left: &ShyValue, 
            ctx: &mut ExecutionContext, expected_result: &ShyValue, expected_store: &ShyValue,
            op: &dyn Fn(&ShyValue, &mut ExecutionContext) -> ShyValue) {
        let result = op(left, ctx);
        asserting("Return value matches").that(&result).is_equal_to(expected_result);
        match left {
            ShyValue::Variable(var_name) => {
                asserting("Stored value matches").that(&ctx.load(var_name).unwrap()).is_equal_to(expected_store);
            },
            _ => assert!(false, "left operand is not a variable")
        }
    }
}
