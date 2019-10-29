
use std::fmt::{Display,Formatter,Result};
use serde::{Serialize, Deserialize};
use super::associativity::Associativity;
use crate::lexer::parser_token::ParserToken;

//..................................................................

//custom_derive! {
    //#[derive(Copy, Clone, Debug, PartialEq, Eq, EnumDisplay, EnumFromStr, IterVariants(ShyOperatorVariants), IterVariantNames(ShyOperatorVariantNames))]
    #[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
    /// A ShyOperator represents a specific operator that may be applied to operands (ShyValues).
    /// Each ShyOperator has an operator precedence. 
    /// All operators are left associative, except the assignment operators, which are right associative.
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
        /// The QuitIfFalse operator is also called the applicability operator. 
        /// If while executing a `Rule` we evaluate this operator and it finds a false value and quits the evaluation,
        /// that means that the `Rule` is not applicable. 
        /// If part of a `RuleSet`, such an `Expression` will not contribute to the final decision 
        /// as to whether the `RuleSet` passed or failed.
        QuitIfFalse,
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
//}

impl Display for ShyOperator {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}", self)
    }
}

impl ShyOperator {
    /// Return the precedence of an operator, where a higher number means that the operator has a higher precedence. 
    pub fn precedence(&self) -> u8  {
        match self {
            ShyOperator::Semicolon => 18, // Semicolon does not follow normal rules of precedence.
            ShyOperator::Load => 17,
            ShyOperator::Store => 17,
            ShyOperator::FunctionCall => 16,
            ShyOperator::OpenParenthesis => 15,
            ShyOperator::CloseParenthesis => 15,
            ShyOperator::OpenBracket => 15,
            ShyOperator::CloseBracket => 15,
            ShyOperator::Member => 15,
            ShyOperator::Power => 14,
            ShyOperator::Exponentiation => 14,
            ShyOperator::PrefixPlusSign => 13,
            ShyOperator::PrefixMinusSign => 13,
            ShyOperator::PostIncrement => 13,
            ShyOperator::PostDecrement => 13,
            ShyOperator::SquareRoot => 13,
            ShyOperator::LogicalNot => 13,
            ShyOperator::Factorial => 12,
            ShyOperator::Match => 11,
            ShyOperator::NotMatch => 11,
            ShyOperator::Multiply => 10,
            ShyOperator::Divide => 10,
            ShyOperator::Mod => 10,
            ShyOperator::Add => 9,
            ShyOperator::Subtract => 9,
            ShyOperator::LessThan => 8,
            ShyOperator::LessThanOrEqualTo => 8,
            ShyOperator::GreaterThan => 8,
            ShyOperator::GreaterThanOrEqualTo => 8,
            ShyOperator::Equals => 7,
            ShyOperator::NotEquals => 7,
            ShyOperator::And => 6, 
            ShyOperator::Or => 5, 
            ShyOperator::Ternary => 4,
            ShyOperator::QuitIfFalse => 4,
            ShyOperator::Comma => 3,
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

    /// Determines if the given operator stores a result in its first argument.
    pub fn is_assignment(&self) -> bool  {
        match self {
            ShyOperator::Assign => true,
            ShyOperator::PlusAssign => true,
            ShyOperator::MinusAssign => true,
            ShyOperator::MultiplyAssign => true,
            ShyOperator::DivideAssign => true,
            ShyOperator::ModAssign => true,
            ShyOperator::AndAssign => true,
            ShyOperator::OrAssign => true,
            ShyOperator::PostIncrement => true,
            ShyOperator::PostDecrement => true,
            _ => false
        }
    }

    /// Number of arguments that each operator takes.
    pub fn arguments(&self) -> usize {
        match self {
            ShyOperator::Load => 1,
            ShyOperator::Store => 1,
            ShyOperator::Semicolon => 0,
            ShyOperator::QuitIfFalse => 1,
            // FunctionCall is variable, but the first argument is the function name while the rest of the arguments are packed into a single Vec by the comma operators.
            ShyOperator::FunctionCall => 2,
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

            // Will not support ternary operator. Using ? for another purpose. 
            // ParserToken::QuestionMark => ShyOperator::Ternary,
            // ParserToken::Colon => ShyOperator::Ternary,
            ParserToken::QuestionMark => ShyOperator::QuitIfFalse,

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
            ParserToken::PropertyChain(_) => ShyOperator::Operand,
            ParserToken::Function(_) => ShyOperator::FunctionCall,
            ParserToken::Error(_) => ShyOperator::Error, 
            _ => ShyOperator::Error
        }
    }
}
