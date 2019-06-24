#![allow(dead_code)]

#[allow(unused_imports)]

use crate::lexer::ParserToken;
use std::mem::discriminant;
use std::f64;
use std::convert::TryFrom;
use std::collections::HashSet;
use std::cmp::Ordering;

/*
    Data used in the ShuntingYard parser:

        - Associativity (used by ShyOperator)
        - ShyOperator (used by ShyToken)
        - ShyScalar (used by ShyValue)
        - ShyValue (used by ShyToken)
        - ShyToken (parsed from ParserToken)

    1. The Lexer reads a string and yields ParserTokens.
    2. ShuntingYard converts ParserTokens into ShyTokens, whether Value variants (ShyValue) or Operator variants (ShyOperator). 
    3. ShuntingYard then resequences the ShyTokens, changing them from infix to postfix order. 

*/

//..................................................................

/// Operator Associativity
custom_derive! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, EnumDisplay, EnumFromStr, IterVariants(AssociativityVariants), IterVariantNames(AssociativityVariantNames))]
    pub enum Associativity {
        Left,
        Right,
        None
    }
}

//..................................................................

/// Checking a string to see if it is truthy or falsy.

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

/// Factorial lookup for integers.

lazy_static! {
    static ref FACTORIAL_FIXED: [i64; 21] = {
        let mut factorial_values : [i64; 21] = [
            1,
            1,
            1 * 2,
            1 * 2 * 3,
            1 * 2 * 3 * 4,
            1 * 2 * 3 * 4 * 5,
            1 * 2 * 3 * 4 * 5 * 6,
            1 * 2 * 3 * 4 * 5 * 6 * 7,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14 * 15,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14 * 15 * 16,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14 * 15 * 16 * 17,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14 * 15 * 16 * 17 * 18,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14 * 15 * 16 * 17 * 18 * 19,
            1 * 2 * 3 * 4 * 5 * 6 * 7 * 8 * 9 * 10 * 11 * 12 * 13 * 14 * 15 * 16 * 17 * 18 * 19 * 20
        ];
        factorial_values
    };
}

/// Compute the factorial of an integer between 0 and 20 (inclusive).
/// If the number os out of range, return None.
fn factorial(n : i64) -> Option<i64> {
    match usize::try_from(n) {
        Ok(i) if i <= 20 => Some(FACTORIAL_FIXED[i]),
        _ => None
    }
}

//..................................................................

/// Approximate Factorial lookup for numbers up to and including 170!.

lazy_static! {
    static ref FACTORIAL_FLOAT: [f64; 171] = {
        let mut factorial_values : [f64; 171] = [1.0; 171];
        for n in 2..=170 {
            factorial_values[n] = factorial_values[n-1] * (n as f64);
        }
        factorial_values
    };
}

/// Compute the factorial of an integer between 0 and 170 (inclusive), where the larger values are approximate.
/// If the number is out of range, return None.
fn factorial_approx(n : i64) -> Option<f64> {
    match usize::try_from(n) {
        Ok(i) if i <= 170 => Some(FACTORIAL_FLOAT[i]),
        _ => None
    }
}

//..................................................................

/// A ShyOperator represents a specific operator that may be applied to operands (ShyValues).
/// Each ShyOperator has an operator precedence. 
/// All operators are left associative, except the assignment operators, which are right associative.
custom_derive! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, EnumDisplay, EnumFromStr, IterVariants(ShyOperatorVariants), IterVariantNames(ShyOperatorVariantNames))]
    pub enum ShyOperator {
        /// Load a value from a variable in the execution context passed in by the caller. 
        /// The variable name will be in a corresponding ShyToken.
        Load,

        /// Store a value resulting from a computation into a variable in the execution context passed by the caller.
        /// The variable name will be in a corresponding ShyToken.
        Store,

        Semicolon,

        /// Indicates that a function will be called, but not which. The function name is stored in a ShyValue.
        FunctionCall,
        OpenParenthesis,
        CloseParenthesis,
        Comma,
        OpenBracket,
        CloseBracket,
        Member,
        PrefixPlusSign,
        PrefixMinusSign,
        PostIncrement,
        PostDecrement,
        Factorial,
        SquareRoot,
        LogicalNot,
        Power,
        Exponentiation,
        Match,
        NotMatch,
        Multiply,
        Divide,
        Mod,
        Add,
        Subtract,
        LessThan,
        LessThanOrEqualTo,
        GreaterThan,
        GreaterThanOrEqualTo,
        Equals,
        NotEquals,
        And, 
        Or, 
        Ternary,
        Assign,
        PlusAssign,
        MinusAssign,
        MultiplyAssign,
        DivideAssign,
        ModAssign,
        AndAssign,
        OrAssign,
        
        /// Operands are not operators - this is how the Operator parser tells the Operand parser (ShyValue) to kick in.
        Operand,
        Error
    }
}

impl ShyOperator {

    pub fn precedence(&self) -> u8  {
        match self {
            ShyOperator::Load => 17,
            ShyOperator::Store => 17,
            ShyOperator::Semicolon => 16,
            ShyOperator::FunctionCall => 15,
            ShyOperator::OpenParenthesis => 14,
            ShyOperator::CloseParenthesis => 14,
            ShyOperator::Comma => 14,
            ShyOperator::OpenBracket => 14,
            ShyOperator::CloseBracket => 14,
            ShyOperator::Member => 14,
            ShyOperator::Power => 13,
            ShyOperator::Exponentiation => 13,
            ShyOperator::PrefixPlusSign => 12,
            ShyOperator::PrefixMinusSign => 12,
            ShyOperator::PostIncrement => 12,
            ShyOperator::PostDecrement => 12,
            ShyOperator::SquareRoot => 12,
            ShyOperator::LogicalNot => 12,
            ShyOperator::Factorial => 11,
            ShyOperator::Match => 10,
            ShyOperator::NotMatch => 10,
            ShyOperator::Multiply => 9,
            ShyOperator::Divide => 9,
            ShyOperator::Mod => 9,
            ShyOperator::Add => 8,
            ShyOperator::Subtract => 8,
            ShyOperator::LessThan => 1,
            ShyOperator::LessThanOrEqualTo => 7,
            ShyOperator::GreaterThan => 7,
            ShyOperator::GreaterThanOrEqualTo => 7,
            ShyOperator::Equals => 6,
            ShyOperator::NotEquals => 6,
            ShyOperator::And => 5, 
            ShyOperator::Or => 4, 
            ShyOperator::Ternary => 3,
            ShyOperator::Assign => 2,
            ShyOperator::PlusAssign => 2,
            ShyOperator::MinusAssign => 2,
            ShyOperator::MultiplyAssign => 2,
            ShyOperator::DivideAssign => 2,
            ShyOperator::ModAssign => 2,
            ShyOperator::AndAssign => 2,
            ShyOperator::OrAssign => 2,
            ShyOperator::Operand => 1,
            ShyOperator::Error => 0
        }
    }

    pub fn associativity(&self) -> Associativity  {
        match self {
            ShyOperator::Assign => Associativity::Right,
            ShyOperator::PlusAssign => Associativity::Right,
            ShyOperator::MinusAssign => Associativity::Right,
            ShyOperator::MultiplyAssign => Associativity::Right,
            ShyOperator::DivideAssign => Associativity::Right,
            ShyOperator::ModAssign => Associativity::Right,
            ShyOperator::AndAssign => Associativity::Right,
            ShyOperator::OrAssign => Associativity::Right,
            ShyOperator::Exponentiation => Associativity::Right,
            ShyOperator::Power => Associativity::Right,
            _ => Associativity::Left
        }
    }

    /// Number of arguments that each operator takes.
    pub fn arguments(self) -> usize {
        match self {
            ShyOperator::Load => 1,
            ShyOperator::Store => 1,
            ShyOperator::Semicolon => 0,
            // FunctionCall is variable, but the arguments are packed into a single Vec by the comma operators.
            ShyOperator::FunctionCall => 1,
            ShyOperator::OpenParenthesis => 0,
            ShyOperator::CloseParenthesis => 0,
            ShyOperator::Comma => 2,
            ShyOperator::OpenBracket => 0,
            ShyOperator::CloseBracket => 1,
            ShyOperator::Member => 2,
            ShyOperator::Power => 2,
            ShyOperator::Exponentiation => 2,
            ShyOperator::PrefixPlusSign => 1,
            ShyOperator::PrefixMinusSign => 1,
            ShyOperator::PostIncrement => 1,
            ShyOperator::PostDecrement => 1,
            ShyOperator::SquareRoot => 1,
            ShyOperator::LogicalNot => 1,
            ShyOperator::Factorial => 1,
            ShyOperator::Match => 2,
            ShyOperator::NotMatch => 2,
            ShyOperator::Multiply => 2,
            ShyOperator::Divide => 2,
            ShyOperator::Mod => 2,
            ShyOperator::Add => 2,
            ShyOperator::Subtract => 2,
            ShyOperator::LessThan => 2,
            ShyOperator::LessThanOrEqualTo => 2,
            ShyOperator::GreaterThan => 2,
            ShyOperator::GreaterThanOrEqualTo => 2,
            ShyOperator::Equals => 2,
            ShyOperator::NotEquals => 2,
            ShyOperator::And => 2, 
            ShyOperator::Or => 2, 
            ShyOperator::Ternary => 3,
            ShyOperator::Assign => 2,
            ShyOperator::PlusAssign => 2,
            ShyOperator::MinusAssign => 2,
            ShyOperator::MultiplyAssign => 2,
            ShyOperator::DivideAssign => 2,
            ShyOperator::ModAssign => 2,
            ShyOperator::AndAssign => 2,
            ShyOperator::OrAssign => 2,
            _ => 0
        }
    }

}

impl From<ParserToken> for ShyOperator {
    fn from(e: ParserToken) -> Self {
        match e {
            ParserToken::Semicolon => ShyOperator::Semicolon,
            ParserToken::OpenParenthesis => ShyOperator::OpenParenthesis,
            ParserToken::CloseParenthesis => ShyOperator::CloseParenthesis,
            ParserToken::Comma => ShyOperator::Comma,
            ParserToken::OpenBracket => ShyOperator::OpenBracket,
            ParserToken::CloseBracket => ShyOperator::CloseBracket,
            ParserToken::MemberOp => ShyOperator::Member,

            ParserToken::SignOp(ref s) if *s == "+" => ShyOperator::PrefixPlusSign,
            ParserToken::SignOp(ref s) if *s == "-" => ShyOperator::PrefixMinusSign,

            ParserToken::IncrementDecrementOp(ref s) if *s == "++" => ShyOperator::PostIncrement,
            ParserToken::IncrementDecrementOp(ref s) if *s == "--" => ShyOperator::PostDecrement,

            ParserToken::FactorialOp => ShyOperator::Factorial,
            ParserToken::LogicalNotOp => ShyOperator::LogicalNot,
            ParserToken::SquareRootOp => ShyOperator::SquareRoot,
            ParserToken::PowerOp(_) => ShyOperator::Power, // Parse must translate into two tokens, an exponentiation and an operand
            ParserToken::ExponentiationOp => ShyOperator::Exponentiation,

            ParserToken::MatchOp(ref s) if *s == "~" => ShyOperator::Match,
            ParserToken::MatchOp(ref s) if *s == "!~" => ShyOperator::NotMatch,

            ParserToken::MultiplicativeOp(ref s) if *s == "*" || *s == "·" => ShyOperator::Multiply,
            ParserToken::MultiplicativeOp(ref s) if *s == "/" => ShyOperator::Divide,
            ParserToken::MultiplicativeOp(ref s) if *s == "%" => ShyOperator::Mod,

            ParserToken::AdditiveOp(ref s) if *s == "+" => ShyOperator::Add,
            ParserToken::AdditiveOp(ref s) if *s == "-" => ShyOperator::Subtract,

            ParserToken::RelationalOp(ref s) if *s == "<" => ShyOperator::LessThan,
            ParserToken::RelationalOp(ref s) if *s == "<=" || *s == "≤" => ShyOperator::LessThanOrEqualTo,
            ParserToken::RelationalOp(ref s) if *s == ">" => ShyOperator::GreaterThan,
            ParserToken::RelationalOp(ref s) if *s == ">="  || *s == "≥" => ShyOperator::GreaterThanOrEqualTo,

            ParserToken::EqualityOp(ref s) if *s == "==" => ShyOperator::Equals, 
            ParserToken::EqualityOp(ref s) if (*s == "!=" || *s == "≠")  => ShyOperator::NotEquals, 

            ParserToken::LogicalOp(ref s) if *s == "&&" => ShyOperator::And, 
            ParserToken::LogicalOp(ref s) if *s == "||"  => ShyOperator::Or, 

            ParserToken::QuestionMark => ShyOperator::Ternary,
            ParserToken::Colon => ShyOperator::Ternary,

            ParserToken::AssignmentOp(ref op) if *op == "=" => ShyOperator::Assign, 
            ParserToken::AssignmentOp(ref op) if *op == "+=" => ShyOperator::PlusAssign, 
            ParserToken::AssignmentOp(ref op) if *op == "-=" => ShyOperator::MinusAssign, 
            ParserToken::AssignmentOp(ref op) if *op == "*=" => ShyOperator::MultiplyAssign, 
            ParserToken::AssignmentOp(ref op) if *op == "/=" => ShyOperator::DivideAssign, 
            ParserToken::AssignmentOp(ref op) if *op == "%=" => ShyOperator::ModAssign, 
            ParserToken::AssignmentOp(ref op) if *op == "&&=" => ShyOperator::AndAssign, 
            ParserToken::AssignmentOp(ref op) if *op == "||=" => ShyOperator::OrAssign, 

            ParserToken::Integer(_) => ShyOperator::Operand,
            ParserToken::Rational(_) => ShyOperator::Operand,
            ParserToken::Regex(_) => ShyOperator::Operand,
            ParserToken::StringLiteral(_) => ShyOperator::Operand,
            ParserToken::Identifier(_) => ShyOperator::Operand,
            ParserToken::Function(_) => ShyOperator::FunctionCall,
            ParserToken::Error(_) => ShyOperator::Error, 
            _ => ShyOperator::Error
        }
    }
}

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

//..................................................................

/// The Output stack of the Shunting Yard parser holds ShyValues wrapped inside a ShyToken.
///   - The final result of evaluating expressions is a single ShyValue, either a Scalar or a Vector.
///   - The Variable and FunctionName variants are intermediate tokens on the output stack that will be
///     removed in the process of evaluating functions, or loading and storing values in the evaluation context.
#[derive(Clone, PartialEq, Debug)]
pub enum ShyValue<'a> {
    /// A scalar value
    Scalar(ShyScalar),

    /// A vector value
    Vector(&'a[ShyScalar]),

    /// Name of a variable in the context to be read from or written to.
    Variable(String),

    /// Name of a function in the context to be called.
    FunctionName(String)
}
const TRUE_STRING: &str = "True";
const FALSE_STRING: &str = "False";
impl<'a> PartialOrd for ShyValue<'a> {

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

            _ => None
        }
    }
}

impl<'a> From<ParserToken> for ShyValue<'a> {
    fn from(parser_token: ParserToken) -> Self {
        match parser_token {
            ParserToken::Function(s) => ShyValue::FunctionName(s),
            ParserToken::Identifier(ref s) if *s == "true"  => ShyValue::Scalar(ShyScalar::Boolean(true)),
            ParserToken::Identifier(ref s) if *s == "false" => ShyValue::Scalar(ShyScalar::Boolean(false)),
            ParserToken::Identifier(s) => ShyValue::Variable(s),
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

impl<'a> ShyValue<'a> {
    pub fn error(message: String) -> Self {
        ShyValue::Scalar(ShyScalar::Error(message))
    }

    pub fn is_error(&self) -> bool {
        match self {
            ShyValue::Scalar(ShyScalar::Error(_)) => true,
            _ => false
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            ShyValue::FunctionName(_) => "FunctionName",
            ShyValue::Variable(_) => "Variable",
            ShyValue::Vector(_) => "Vector",
            ShyValue::Scalar(ShyScalar::Boolean(_)) => "Boolean",
            ShyValue::Scalar(ShyScalar::Integer(_)) => "Integer",
            ShyValue::Scalar(ShyScalar::Rational(_)) => "Rational",
            ShyValue::Scalar(ShyScalar::String(_)) => "String",
            ShyValue::Scalar(ShyScalar::Error(_)) => "Error",
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

    // Arithmetic Operators

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

    /// Exponentiation operator. 
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
            None => ShyValue::error("Incomparable types".to_string())
        }
    }

    /// Less than or equal to operator for ShyValues.
    pub fn less_than_or_equal_to(left_operand: &Self, right_operand: &Self) -> Self {
        match left_operand.partial_cmp(right_operand) {
            Some(Ordering::Less) => true.into(),
            Some(Ordering::Equal) => true.into(),
            Some(Ordering::Greater) => false.into(),
            None => ShyValue::error("Incomparable types".to_string())
        }
    }

    /// Greater than operator for ShyValues.
    pub fn greater_than(left_operand: &Self, right_operand: &Self) -> Self {
        match left_operand.partial_cmp(right_operand) {
            Some(Ordering::Less) => false.into(),
            Some(Ordering::Equal) => false.into(),
            Some(Ordering::Greater) => true.into(),
            None => ShyValue::error("Incomparable types".to_string())
        }
    }

    /// Greater than operator for ShyValues.
    pub fn greater_than_or_equal_to(left_operand: &Self, right_operand: &Self) -> Self {
        match left_operand.partial_cmp(right_operand) {
            Some(Ordering::Less) => false.into(),
            Some(Ordering::Equal) => true.into(),
            Some(Ordering::Greater) => true.into(),
            None => ShyValue::error("Incomparable types".to_string())
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

    // Assignment Operators

    /*
        Assign,
        PlusAssign,
        MinusAssign,
        MultiplyAssign,
        DivideAssign,
        ModAssign,
        AndAssign,
        OrAssign,
    */

    //..................................................................

    // Miscellaneous Operators

    /*
        Comma,
        OpenBracket,
        CloseBracket,
        Member,
        PrefixPlusSign,
        PrefixMinusSign,
        Match,
        NotMatch,
        Ternary,
        PostIncrement,
        PostDecrement,
    */
}

// Conversions from basic types to ShyValue

impl<'a> From<f64> for ShyValue<'a> { fn from(x: f64) -> Self { ShyValue::Scalar(ShyScalar::Rational(x)) } }
impl<'a> From<&f64> for ShyValue<'a> { fn from(x: &f64) -> Self { ShyValue::Scalar(ShyScalar::Rational(*x)) } }
impl<'a> From<i64> for ShyValue<'a> { fn from(x: i64) -> Self { ShyValue::Scalar(ShyScalar::Integer(x)) } }
impl<'a> From<&i64> for ShyValue<'a> { fn from(x: &i64) -> Self { ShyValue::Scalar(ShyScalar::Integer(*x)) } }
impl<'a> From<i32> for ShyValue<'a> { fn from(x: i32) -> Self { ShyValue::Scalar(ShyScalar::Integer(x as i64)) } }
impl<'a> From<&i32> for ShyValue<'a> { fn from(x: &i32) -> Self { ShyValue::Scalar(ShyScalar::Integer(*x as i64)) } }
impl<'a> From<bool> for ShyValue<'a> { fn from(x: bool) -> Self { ShyValue::Scalar(ShyScalar::Boolean(x)) } }
impl<'a> From<&bool> for ShyValue<'a> { fn from(x: &bool) -> Self { ShyValue::Scalar(ShyScalar::Boolean(*x)) } }
impl<'a> From<String> for ShyValue<'a> { fn from(s: String) -> Self { ShyValue::Scalar(ShyScalar::String(s.clone())) } }
impl<'a> From<&str> for ShyValue<'a> { fn from(s: &str) -> Self { ShyValue::Scalar(ShyScalar::String(s.to_string())) } }


//..................................................................

/// ShyToken represents the tokens on the Output stack of the Shunting Yard Algorithm.
///   - The Value and Operator variants will appear on the Output stack. 
///   - The None value is for error processing.
///   - The OperatorWithValue (used for Functions and Power) will be split into 
///     a Value token (the Function name) and an Operator token (the function invocation).
#[derive(Clone, PartialEq, Debug)]
pub enum ShyToken<'a> {
    Value(ShyValue<'a>),
    Operator(ShyOperator),
    OperatorWithValue(ShyOperator, ShyValue<'a>),
    Error,
    None
}

impl<'a> ShyToken<'a> {
    pub fn is_error(&self) -> bool {
        discriminant(&ShyToken::Error) == discriminant(self)
    }
}

/// Convert a ParserToken into a ShyToken.
impl<'a> From<ParserToken> for ShyToken<'a> {
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

    // ...................................................................................

    // Test helpers

    /// Test a binary operator
    fn binary_operator_test<'a>(left: &ShyValue<'a>, right: &ShyValue<'a>, expected: &ShyValue<'a>, op: &Fn(&ShyValue<'a>, &ShyValue<'a>) -> ShyValue<'a>) {
        let actual = op(left, right);
        asserting(&format!("Operation on {:?} and {:?} should yield {:?}", left, right, expected)).that(&actual).is_equal_to(expected);
    }

    /// Test a unary operator
    fn unary_operator_test<'a>(left: &ShyValue<'a>, expected: &ShyValue<'a>, op: &Fn(&ShyValue<'a>) -> ShyValue<'a>) {
        let actual = op(left);
        asserting(&format!("Operation on {:?} should yield {:?}", left, expected)).that(&actual).is_equal_to(expected);
    }
}
