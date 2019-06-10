#![allow(dead_code)]

#[allow(unused_imports)]
use crate::lexer::ParserToken;
use std::mem::discriminant;

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
}
