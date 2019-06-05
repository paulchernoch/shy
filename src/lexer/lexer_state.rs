#![allow(dead_code)]

//use std::fmt::Display;
//use std::fmt::Formatter;
//use std::fmt::Result;

//..................................................................

/// LexerState names the State of the Lexer's stack automata.
custom_derive! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, EnumDisplay, EnumFromStr, IterVariants(LexerStateVariants), IterVariantNames(LexerStateVariantNames))]
    pub enum LexerState {
        Start,
        Goal,
        Error,
        Empty,
        String,
        StringEscape,
        Identifier,
        FunctionName,
        ContinuableOperator,
        LogicalOperator,
        ExpectRegex,
        Regex,
        RegexEscape,
        IntegerDigits,
        FractionalDigits,
        ExponentSign,
        ExponentDigits,
        Power,
        Exclamation
    }
}

impl LexerState {
    pub fn value(&self) -> i32 {
        match *self {
            LexerState::Start => 0,
            LexerState::Goal => 1,
            LexerState::Error => 2,
            LexerState::Empty => 3,
            LexerState::String => 4,
            LexerState::StringEscape => 5,
            LexerState::Identifier => 6,
            LexerState::FunctionName => 7,
            LexerState::ContinuableOperator =>8,
            LexerState::LogicalOperator => 9,
            LexerState::ExpectRegex => 10,
            LexerState::Regex => 11,
            LexerState::RegexEscape => 12,
            LexerState::IntegerDigits => 13,
            LexerState::FractionalDigits => 14,
            LexerState::ExponentSign => 15,
            LexerState::ExponentDigits => 16,
            LexerState::Power => 17,
            LexerState::Exclamation => 18
        }
    }
    pub fn size() -> i32 {
        19
    }
}

