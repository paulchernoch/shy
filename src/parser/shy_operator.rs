
use super::associativity::Associativity;
use crate::lexer::ParserToken;

//..................................................................

custom_derive! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, EnumDisplay, EnumFromStr, IterVariants(ShyOperatorVariants), IterVariantNames(ShyOperatorVariantNames))]
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
            ShyOperator::Semicolon => 18, // Semicolon does not follow normal rules of precedence.
            ShyOperator::Load => 17,
            ShyOperator::Store => 17,
            ShyOperator::FunctionCall => 16,
            ShyOperator::OpenParenthesis => 15,
            ShyOperator::CloseParenthesis => 15,
            ShyOperator::Comma => 15,
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
            ShyOperator::Factorial => 11,
            ShyOperator::Match => 10,
            ShyOperator::NotMatch => 10,
            ShyOperator::Multiply => 9,
            ShyOperator::Divide => 9,
            ShyOperator::Mod => 9,
            ShyOperator::Add => 8,
            ShyOperator::Subtract => 8,
            ShyOperator::LessThan => 7,
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
