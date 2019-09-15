use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

//..................................................................

/// Error details for the Lexer
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LexerError {
    pub error_position: i32,
    pub error_line: i32,
    pub log: String
}

//..................................................................

#[derive(Clone, PartialEq, Eq, Debug)]
/// Tokens returned by the Lexer.
/// 
/// Note: MemberOp is currently not produced. 
///       Instead, a series of identifiers separated by periods with no intervening spaces is rendered as a
///       PropertyChain. This is easier than modifying Shunting Yard to deal with member operators.
///       Figuring out whether a chain will be used as a lvalue or an rvalue requires too much lookahead
///       if it is represented as multiple Identifiers interspersed with MemberOps. 
pub enum ParserToken {
    StringLiteral(String),
    Identifier(String),
    PropertyChain(Vec<String>),
    Function(String),
    LogicalNotOp,
    FactorialOp,
    Integer(String),
    Rational(String),
    Regex(String),
    OpenParenthesis,
    CloseParenthesis,
    Comma,
    QuestionMark,
    Colon,
    Semicolon,
    OpenBracket,
    CloseBracket,
    ExponentiationOp,
    PowerOp(String),
    MemberOp,
    MatchOp(String), // ~ !~
    AssignmentOp(String), // = += -= *= %= /= &&= ||=
    MultiplicativeOp(String), // * / %
    SignOp(String), // + -
    AdditiveOp(String), // + -  (note the conflict with SignOp! Parser may have to change one into the other based on context.)
    IncrementDecrementOp(String), // ++ --
    RelationalOp(String), // < <= ≤ > >= ≥ != ≠
    EqualityOp(String), // ==
    LogicalOp(String), // && ||
    SquareRootOp, // √
    Error(LexerError)
}

impl ParserToken {
    /// The name of the enum variant.
    pub fn name(&self) -> &'static str  {
        match self {
            ParserToken::StringLiteral(_) => "StringLiteral",
            ParserToken::Identifier(_) => "Identifier",
            ParserToken::PropertyChain(_) => "PropertyChain",
            ParserToken::Function(_) => "Function",
            ParserToken::LogicalNotOp => "LogicalNotOp",
            ParserToken::FactorialOp => "FactorialOp",
            ParserToken::Integer(_) => "Integer",
            ParserToken::Rational(_) => "Rational",
            ParserToken::Regex(_) => "Regex",
            ParserToken::OpenParenthesis => "OpenParenthesis",
            ParserToken::CloseParenthesis => "CloseParenthesis",
            ParserToken::Comma => "Comma",
            ParserToken::QuestionMark => "QuestionMark",
            ParserToken::Colon => "Colon",
            ParserToken::Semicolon => "Semicolon",
            ParserToken::OpenBracket => "OpenBracket",
            ParserToken::CloseBracket => "CloseBracket",
            ParserToken::ExponentiationOp => "ExponentiationOp",
            ParserToken::PowerOp(_) => "PowerOp",
            ParserToken::MemberOp => "MemberOp",
            ParserToken::MatchOp(_) => "MatchOp",
            ParserToken::AssignmentOp(_) => "AssignmentOp",
            ParserToken::MultiplicativeOp(_) => "MultiplicativeOp",
            ParserToken::AdditiveOp(_) => "AdditiveOp",
            ParserToken::SignOp(_) => "SignOp",
            ParserToken::IncrementDecrementOp(_) => "IncrementDecrementOp",
            ParserToken::RelationalOp(_) => "RelationalOp",
            ParserToken::EqualityOp(_) => "EqualityOp",
            ParserToken::LogicalOp(_) => "LogicalOp", 
            ParserToken::SquareRootOp => "SquareRootOp", 
            ParserToken::Error(_) => "Error", 
        }
    }

    /// The value of the character or string stored in the token.
    pub fn val(&self) -> String {
        let error_message : String;
        let mut temp_string = String::new();
        let return_val: &str = match self {
            ParserToken::StringLiteral(s) => s,
            ParserToken::Identifier(s) => s,
            ParserToken::PropertyChain(vec) => {
                 temp_string.push_str(&vec.join("."));
                 &temp_string
            },
            ParserToken::Function(s) => s,
            ParserToken::LogicalNotOp => "!",
            ParserToken::FactorialOp => "!",
            ParserToken::Integer(s) => s,
            ParserToken::Rational(s) => s,
            ParserToken::Regex(s) => s,
            ParserToken::OpenParenthesis => "(",
            ParserToken::CloseParenthesis => ")",
            ParserToken::Comma => ",",
            ParserToken::QuestionMark => "?",
            ParserToken::Colon => ":",
            ParserToken::Semicolon => ";",
            ParserToken::OpenBracket => "[",
            ParserToken::CloseBracket => "]",
            ParserToken::ExponentiationOp => "^",
            ParserToken::PowerOp(s) => s,
            ParserToken::MemberOp => ".",
            ParserToken::MatchOp(s) => s,
            ParserToken::AssignmentOp(s) => s,
            ParserToken::MultiplicativeOp(s) => s,
            ParserToken::SignOp(s) => s,
            ParserToken::AdditiveOp(s) => s,
            ParserToken::IncrementDecrementOp(s) => s,
            ParserToken::RelationalOp(s) => s,
            ParserToken::EqualityOp(s) => s,
            ParserToken::LogicalOp(s) => s,
            ParserToken::SquareRootOp => "√",
            ParserToken::Error(err) => {
                error_message = format!("Error!\nLine {}, position {}, Log:\n{}", err.error_line, err.error_position, err.log);
                &error_message
            }

        };
        return_val.to_string()
    }

    pub fn new_property_chain(chain_as_string : &String) -> Self {
        ParserToken::PropertyChain(chain_as_string.split(".").map(|s| s.to_string()).collect())
    }

    pub fn to_property_chain(&self) -> Self {
        match self {
            ParserToken::Identifier(ref name) if name.find(".") != None => {
                ParserToken::new_property_chain(name)
            },
            _ => self.clone()
        }
    }
}

impl Display for ParserToken {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut _write = |s: String| write!(f, "{}", s);
        _write(self.val())
    }
}
