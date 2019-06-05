#![allow(dead_code)]

extern crate itertools;

use itertools::put_back;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;
use std::convert::From;



/// Divides characters into categories such that all characters in the same category serve similar
/// functions in the lexical analyzer. For example, both a dollar sign and an underscore may appear in identifiers,
/// and at present have no other uses (except as part of a string literal or regex).
/// 
/// Some of these events correspond to whole parser tokens (like operators) but others will need to be combined
/// with a series of events to form a parser token.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LexerEvent {
    BOS,
    EOS,
    Space,
    Newline,
    Letter(char),
    Digit(char),
    Superscript(char), // ¹ ² ³ ⁴ ⁵ ⁶ ⁷ ⁸ ⁹ ⁰
    DollarUnderscore(char), // $ _
    Backslash, // \
    Slash,  // /
    DoubleQuote, // ""
    Equals,  // =
    ExclamationPoint,
    ExpressionStarter(char), // ( [ 
    ExpressionEnder(char), // ) , ? : ; ]
    Caret, // ^
    Period, // .
    Sign(char), // + -
    Multiplicative(char), // * / % ·
    Comparison(char), // ≤ ≥ ≠
    AngleBracket(char), // < >
    AmpersandBar(char), // & |
    Tilde,
    Other(char)
}

impl LexerEvent {
    pub fn new(c: char) -> LexerEvent {
        match c {
            '«' => LexerEvent::BOS,
            '»' => LexerEvent::EOS,
            ' ' | '\t' => LexerEvent::Space,
            '\n' => LexerEvent::Newline,
            'a'...'z' | 'A'...'Z' | 'α'...'ω' | 'Α'...'Ω' => LexerEvent::Letter(c),
            '0'...'9' => LexerEvent::Digit(c),
            '¹' | '²' | '³' | '⁴' | '⁵' | '⁶' | '⁷' | '⁸' | '⁹' | '⁰' => LexerEvent::Superscript(c),
            '$' | '_' => LexerEvent::DollarUnderscore(c),
            '\\'  => LexerEvent::Backslash,
            '/'  => LexerEvent::Slash,
            '"'  => LexerEvent::DoubleQuote,
            '='  => LexerEvent::Equals,
            '!'  => LexerEvent::ExclamationPoint,
            '(' | '[' => LexerEvent::ExpressionStarter(c),
            ')' | ',' | '?' | ':' | ';' | ']' => LexerEvent::ExpressionEnder(c),
            '^'  => LexerEvent::Caret,
            '.'  => LexerEvent::Period,
            '+' | '-'  => LexerEvent::Sign(c),
            '*' | '%' | '·' => LexerEvent::Multiplicative(c),
            '<' | '>'  => LexerEvent::AngleBracket(c),
            '≤' | '≥' | '≠' => LexerEvent::Comparison(c),   
            '&' | '|'  => LexerEvent::AmpersandBar(c),
            '~'  => LexerEvent::Tilde,
            _ => LexerEvent::Other(c)
        }
    }
}

impl Display for LexerEvent {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut _ctos = |c: char| write!(f, "{}", c.to_string());
        match self {
            LexerEvent::BOS => _ctos('«'),
            LexerEvent::EOS => _ctos('»'),
            LexerEvent::Space => _ctos(' '),
            LexerEvent::Newline => _ctos('\n'),
            LexerEvent::Letter(letter) => _ctos(*letter),
            LexerEvent::Digit(digit) => _ctos(*digit),
            LexerEvent::Superscript(digit) => _ctos(*digit),
            LexerEvent::DollarUnderscore(du) => _ctos(*du),
            LexerEvent::Backslash => _ctos('\\'),
            LexerEvent::Slash => _ctos('/'),
            LexerEvent::DoubleQuote => _ctos('"'),
            LexerEvent::Equals => _ctos('='),
            LexerEvent::ExclamationPoint => _ctos('!'),
            LexerEvent::ExpressionStarter(starter) => _ctos(*starter),
            LexerEvent::ExpressionEnder(ender) => _ctos(*ender),
            LexerEvent::Caret => _ctos('^'),
            LexerEvent::Period => _ctos('.'),
            LexerEvent::Sign(sign) => _ctos(*sign),
            LexerEvent::Multiplicative(mult) => _ctos(*mult),
            LexerEvent::AngleBracket(angle) => _ctos(*angle),
            LexerEvent::Comparison(cmp) => _ctos(*cmp),
            LexerEvent::AmpersandBar(ab) => _ctos(*ab),
            LexerEvent::Tilde => _ctos('~'),
            LexerEvent::Other(o) => _ctos(*o)
        }
    }
}

impl From<char> for LexerEvent {
    fn from(item: char) -> Self {
        LexerEvent::new(item)
    }
}

impl From<&LexerEvent> for char {
    fn from(item: &LexerEvent) -> Self {
        match item.to_string().chars().next() {
            Some(c) => c,
            None => '�'
        }
    }
}

impl From<LexerEvent> for char {
    fn from(item: LexerEvent) -> Self {
        match item.to_string().chars().next() {
            Some(c) => c,
            None => '�'
        }
    }
}

///  LexerEventIterator iterates over a string and generates LexerEvents for each character.
/// You can also put_back characters/events, and peek ahead.
pub struct LexerEventIterator<'e> {
    /// Iterates over the characters in a string, with ability to put_back a character.
    char_iter: itertools::PutBack<std::str::Chars<'e>>,

    /// Has the beginning of string token (BOS) been issued yet?
    issued_bos: bool,

    /// Has the end of string token (EOS) been issued yet?
    issued_eos: bool,

    /// Number of characters that have passed through the lexical analyzer so far.
    /// Putting back characters decrements the position.
    position: i32,

    /// Line number in the input text where the iterator currently is.
    line: i32
}

impl<'e> LexerEventIterator<'e> {
    pub fn new(s: &'e std::string::String) -> LexerEventIterator<'e> {
        LexerEventIterator { 
            char_iter: put_back(s.chars()),
            issued_bos: false,
            issued_eos: false,
            position: 0,
            line: 1
        }
    }

    pub fn put_back(&mut self, e: LexerEvent) -> () {
        self.position -= 1;
        if let LexerEvent::Newline = e {
            self.line -= 1;
        }
        let c: char = e.into();
        self.char_iter.put_back(c);
    }

    /// Get the next element in the iterator if any, then if there was one, put it back.
    /// Returns a clone of the next item in the iterator in an Option, or None if the iterator reached the end.
    pub fn peek(&mut self) -> Option<LexerEvent> {
        let next_item_option = self.next();
        if let Some(next_event) = next_item_option {
            let copy_of_next = next_event.clone();
            self.put_back(next_event);
            Some(copy_of_next)
        }
        else {
            None
        }
    }

    pub fn current_position(&mut self) -> i32 {
        self.position
    }

    pub fn current_line(&mut self) -> i32 {
        self.line
    }
}

impl<'e> Iterator for LexerEventIterator<'e> {
    type Item = LexerEvent;
    fn next(&mut self) -> Option<LexerEvent> {
        if !self.issued_bos {
            self.issued_bos = true;
            return Some(LexerEvent::BOS);
        }
        match self.char_iter.next() {
            Some(c) => { 
                self.position += 1;
                if c == '\n' {
                    self.line += 1;
                }
                Some(LexerEvent::new(c))
            },
            None => { 
                if !self.issued_eos {
                    self.issued_eos = true;
                    return Some(LexerEvent::EOS);
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    #[test]
    fn lexer_event_new() {
        assert_eq!(LexerEvent::new('«'), LexerEvent::BOS);
        assert_eq!(LexerEvent::new('»'), LexerEvent::EOS);
        assert_eq!(LexerEvent::new('_'), LexerEvent::DollarUnderscore('_'));
        assert_eq!(LexerEvent::new('·'), LexerEvent::Multiplicative('·'));
        assert_eq!(LexerEvent::new('²'), LexerEvent::Superscript('²'));
    }

    #[test]
    fn lexer_event_to_string() {
        assert_eq!(LexerEvent::Tilde.to_string(), "~");
        assert_eq!(LexerEvent::Backslash.to_string(), "\\");
        assert_eq!(LexerEvent::Digit('4').to_string(), "4");
    }

    #[test]
    /// Verify that the correct sequence of LexerEvents is generated when an expression is analyzed by the LexerEventIterator.
    fn lexer_event_iteration() {
        let expression = "(2 + x) * 3!".to_string();
        let lex_iter = LexerEventIterator::new(&expression);
        let expected_events = vec![
            LexerEvent::BOS,
            LexerEvent::ExpressionStarter('('), 
            LexerEvent::Digit('2'), 
            LexerEvent::Space, 
            LexerEvent::Sign('+'), 
            LexerEvent::Space, 
            LexerEvent::Letter('x'), 
            LexerEvent::ExpressionEnder(')'), 
            LexerEvent::Space, 
            LexerEvent::Multiplicative('*'), 
            LexerEvent::Space, 
            LexerEvent::Digit('3'), 
            LexerEvent::ExclamationPoint,
            LexerEvent::EOS
        ];
        let mut actual_events = Vec::new();
        actual_events.extend(lex_iter);
        assert_that!(&actual_events.iter()).contains_all_of(&expected_events.iter());
    }

}