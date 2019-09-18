#![allow(dead_code)]

use std::mem;

pub mod lexer_event;
use lexer_event::LexerEvent;
use lexer_event::LexerEventIterator;

pub mod lexer_state;
use lexer_state::LexerState;

pub mod parser_token;
use parser_token::{ParserToken, LexerError};


// Tokenizer classes:
//    - LexerError: Stored inside ParserToken::Error.
//    - ParserToken: Categorizes part of the expression string as a token useable by a parser 
//    - LexerState: Describes which state the Lexer state machine is in (see lexer_state.rs)
//    - Lexer: Uses a state machine to break a string into ParserTokens


//..................................................................

/// Lexer is a lexical analyzer for breaking an input string of characters 
/// into a series of lexical tokens of type ParserToken, which can be consumed by the Parser.
/// The Lexer reports errors by returning a ParserToken::Error, which describes the error and its position in the expression.
pub struct Lexer<'e> {
    /// Current state of the lexical analyzer.
    current_state: LexerState,

    /// Iterates over the formula text string, and categorizes each character, and returns one LexerEven for each.
    events: LexerEventIterator<'e>,

    /// Construction site for the next token to be yielded by the iterator, built up one character at a time.
    next_token: String,

    /// Buffer holding the next token in cases when we need to yield two tokens at once
    token_buffer: Option<ParserToken>,

    /// Character position within the string being tokenized where the first error occurred, or -1 if no error yet.
    position_with_error: i32,

    /// Turn on/off logging
    pub enable_logging: bool,

    /// Holds the log of transitions from state to state if enable_logging is true.
    transition_log: String
}

impl<'e> Lexer<'e> {

    /// Create a new Lexer over the given string and start it in the Start state.
    pub fn new(s: &'e String) -> Lexer<'e> {
        Lexer {
            current_state: LexerState::Start,
            events: LexerEventIterator::new(&s),
            next_token: String::new(),
            token_buffer: None,
            position_with_error: -1,
            enable_logging: false,
            transition_log: String::new()
        } 
    }

    //..................................................................

    // Status related


    pub fn has_reached_goal(&mut self) -> bool { self.current_state == LexerState::Goal }

    pub fn has_error(&mut self) -> bool { self.current_state == LexerState::Error }

    //..................................................................

    // Stack manipulation

    /// Push the character that the LexerEvent represents onto the end of the next_token.
    fn push(&mut self, e: LexerEvent) { self.next_token.push_str(&e.to_string()) }

    fn push_char(&mut self, c: char) { self.next_token.push_str(&c.to_string()) }

    /// Return the current value of next_token, simultaneously storing a new empty string as next_token.
    fn yield_string(&mut self) -> String { mem::replace(&mut self.next_token, String::new()) }

    //..................................................................

    // Lookahead processing

    /// Peek at the next event, convert it to s string and compare it to the given match_string.
    /// Return true if they match, false otherwise, including if there are no more events at which to peek.
    fn does_next_token_match_string(&mut self, match_string: String) -> bool {
        match self.events.peek() {
            Some(token) => token.to_string() == match_string,
            None => false
        }
    }

    fn does_next_token_match_filter<F>(&mut self, match_filter: F) -> bool
    where F : Fn(LexerEvent) -> bool
    {
        match self.events.peek() {
            Some(token) => match_filter(token),
            None => false
        }
    }

    
    //..................................................................

    // Methods that perform transitions from one state to another, with various side-effects.

    /// Transition to a new state without yielding a token, discarding the event.
    fn transition_without_yield(&mut self, new_state: LexerState) -> Option<ParserToken> {
        self.current_state = new_state;
        None
    }

    /// Transition to a new state and yield the given token (not the text on the stack).
    fn transition_with_yield(&mut self, new_state: LexerState, token_to_yield: ParserToken ) -> Option<ParserToken> {
        self.current_state = new_state;
        Some(token_to_yield)
    }

    /// Transition to a new state and yield a token built from the stack, while placing a second token in the buffer so it can be yielded in the next iteration of the lexer loop.
    fn transition_with_double_yield<TokenMaker>(&mut self, new_state: LexerState, token_maker: TokenMaker, token_to_buffer: ParserToken  ) -> Option<ParserToken>
    where TokenMaker : Fn(String) -> Option<ParserToken>
    {
        self.current_state = new_state;
        if let Some(_) = &self.token_buffer {
            panic!("Lexer buffer already full");
        }
        self.token_buffer = Some(token_to_buffer);
        token_maker(self.yield_string())
    }

    /// Transition to a new state and push the string form of the event onto the stack, without yielding a token.
    fn transition_with_push(&mut self, new_state: LexerState, event_to_push: LexerEvent) -> Option<ParserToken> {
        self.current_state = new_state;
        self.push(event_to_push);
        None
    }

    /// Transition to a new state and push the given character onto the stack, without yielding a token.
    fn transition_with_push_char(&mut self, new_state: LexerState, char_to_push: char) -> Option<ParserToken> {
        self.current_state = new_state;
        self.push_char(char_to_push);
        None
    }

    /// Transition to a new state, pop a string from the stack, create a ParserToken from that string and return the token.
    /// The new event is discarded.
    fn transition_with_pop<TokenMaker>(&mut self, new_state: LexerState, token_maker: TokenMaker) -> Option<ParserToken> 
    where TokenMaker : Fn(String) -> Option<ParserToken>
    {
        self.current_state = new_state;
        token_maker(self.yield_string())
    }

    /// Transition to a new state, create a ParserToken from the string from the stack and the current event, and return the token.
    /// The string form of the new event is appended to the popped string.
    /// If the generated token Option is None, transition to the Error state.
    fn transition_with_pop_plus_event<TokenMaker>(&mut self, new_state: LexerState, token_maker: TokenMaker, current_event: LexerEvent) -> Option<ParserToken> 
    where TokenMaker : Fn(String) -> Option<ParserToken>
    {
        match token_maker(self.yield_string() + &current_event.to_string()) {
            None => self.transition_to_error(current_event),
            Some(parser_token) => self.transition_with_yield(new_state, parser_token)
        }
    }

    /// Transition to a new state and put the event back on the events iterator, without yielding a token.
    fn transition_with_put_back(&mut self, new_state: LexerState, event_to_put_back: LexerEvent) -> Option<ParserToken> 
    {
        self.current_state = new_state;
        self.events.put_back(event_to_put_back);
        None // not an error
    }

    /// Transition to a new state and put the event back on the events iterator, yielding a token popped from the stack.
    /// The pop for the yielded token occurs before the push of the event.
    fn transition_with_pop_and_put_back<TokenMaker>(&mut self, new_state: LexerState, token_maker: TokenMaker, event_to_put_back: LexerEvent) -> Option<ParserToken>
    where TokenMaker : Fn(String) -> Option<ParserToken>
    {
        let token_to_yield = self.transition_with_pop(new_state, token_maker);
        self.events.put_back(event_to_put_back);
        token_to_yield
    }

    /// Reenter the same state and yield the given token.
    fn reenter_with_yield(&mut self, token_to_yield: ParserToken) -> Option<ParserToken> {
        Some(token_to_yield)
    }

    /// Reenter the same state, discard the character read from the expression, and do not yield a ParserToken.
    fn reenter_without_yield(&mut self) -> Option<ParserToken> { None } // not an error

    /// Reenter the same state, push the string form of the given event onto the stack, and do not yield a ParserToken. 
    fn reenter_with_push(&mut self, event_to_push: LexerEvent) -> Option<ParserToken>  {
        self.push(event_to_push);
        None // not an error
    }

    /// Transition to the Error state; do not yield a token.
    fn transition_to_error(&mut self, e: LexerEvent) -> Option<ParserToken> {
        // Solution to Chicken-and-egg problem:
        //
        // Q: Logging is done in two places, here and in Lexer.next.
        // We must only do it once, or we double the log size.
        // We log each token in log_append, but the ParserToken::Error must have the 
        // full log written to it so the caller can see what went wrong. 
        // How can the Error token be created if its construction
        // requires a complete log message that includes itself? 
        //
        // A: To break the chicken-and-egg impasse, we do this: 
        //    1. Create two ParserToken::Error objects, 
        //    2. Log the first ParserToken::Error,
        //    3. Copy the complete log from the Lexer
        //    4. Include that complete log when creating the second ParserToken::Error.
        //    5. Return the second ParserToken::Error to the caller. 

        let previous_state = self.current_state.to_string();
        self.current_state = LexerState::Error;
        
        if self.position_with_error < 0 {
            self.position_with_error = self.events.current_position();
        }
        let temp_error_token = Some(ParserToken::Error(LexerError { 
            error_position: self.position_with_error, 
            error_line: self.events.current_line(),
            log: format!("Error receiving '{}'", e.to_string())
         }));
        self.log_append(self.position_with_error, e.to_string(), previous_state, &temp_error_token);
        Some(
            ParserToken::Error(
                LexerError { error_position: self.position_with_error, error_line: self.events.current_line(), log: self.get_log() }
            )
        )
    }
    
    //..................................................................

    // Methods for the states:
    //   - All state methods return an Option<ParserToken> which holds either
    //       - the next token to be yielded by the iterator
    //       - or None if we need to process more characters before we can complete the next token. 
    //   - The state method may cause the current_state to change or not.
    //   - Many state methods push a character onto the end of the next_token. 
    //   - Some state methods lookahead or put a character back.
    //   - Any LexerEvent not accepted by a given state will cause the lexer to enter the Error state.

    /// start state transitions.
    /// This state is the beginning state for the lexer's push-down automata.
    fn start(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            LexerEvent::BOS => self.transition_without_yield(LexerState::Empty),
            _ => self.transition_to_error(e)
        }
    }

    /// goal state transitions. 
    /// This is the final state, only reached after we receive the EOS (end-of-string) event.
    fn goal(&mut self, e: LexerEvent) -> Option<ParserToken> { 
        match e {
            LexerEvent::EOS => None,
            _ => self.transition_to_error(e)
        }
    }

    /// empty state transitions. 
    /// This state means that we have not accumulated any characters for the next token.
    fn empty(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            // Whitespace outside of a string or Regex will be skipped. 
            LexerEvent::Space | LexerEvent::Newline => self.reenter_without_yield(),

            // Start of a string found; go to the string state to await the matching closing quote 
            // (and handle escaped characters)
            LexerEvent::DoubleQuote => self.transition_without_yield(LexerState::String),

            // Unambiguously single character operators or grouping symbols
            LexerEvent::ExpressionStarter('(') => self.reenter_with_yield(ParserToken::OpenParenthesis),
            LexerEvent::ExpressionStarter('[') => self.reenter_with_yield(ParserToken::OpenBracket),
            LexerEvent::ExpressionEnder(')') => self.reenter_with_yield(ParserToken::CloseParenthesis),
            LexerEvent::ExpressionEnder(']') => self.reenter_with_yield(ParserToken::CloseBracket),
            LexerEvent::ExpressionEnder(',') => self.reenter_with_yield(ParserToken::Comma),
            LexerEvent::ExpressionEnder(';') => self.reenter_with_yield(ParserToken::Semicolon),
            LexerEvent::ExpressionEnder('?') => self.reenter_with_yield(ParserToken::QuestionMark),
            LexerEvent::ExpressionEnder(':') => self.reenter_with_yield(ParserToken::Colon),
            LexerEvent::Caret => self.reenter_with_yield(ParserToken::ExponentiationOp),
            LexerEvent::Comparison(relop) => self.reenter_with_yield(ParserToken::RelationalOp(relop.to_string())),
            LexerEvent::SquareRoot => self.reenter_with_yield(ParserToken::SquareRootOp),

            // A period can dereference a property (e.g. person.name) or begin a degenerate number (e.g. .5)
            LexerEvent::Period => if self.does_next_token_match_filter(
                |evt| {
                    let c: char = evt.into();
                    c.is_ascii_digit()
                }
            )
                 { self.transition_with_push(LexerState::FractionalDigits, e) }
            else { self.reenter_with_yield(ParserToken::MemberOp) },

            // Start of an identifier found; go to the identifier state and wait until we reach 
            // a character not permitted in an identifier.
            LexerEvent::Letter(_) => self.transition_with_push(LexerState::Identifier, e),
            LexerEvent::DollarUnderscore(_) => self.transition_with_push(LexerState::Identifier, e),

            // First character in an operator that may have one or two characters, like ==, >=, <=, %=, +=, /=, ++, --
            LexerEvent::Multiplicative(_) => self.transition_with_push(LexerState::ContinuableOperator, e),
            LexerEvent::Slash => self.transition_with_push(LexerState::ContinuableOperator, e),
            LexerEvent::Equals => self.transition_with_push(LexerState::ContinuableOperator, e),
            LexerEvent::AngleBracket(_) => self.transition_with_push(LexerState::ContinuableOperator, e),

            // Plus or minus sign may be part or all of an operator (+ - ++ -- += -=) or part of a number literal (+3.5, -7).
            LexerEvent::Sign(_) => if self.does_next_token_match_filter(
                |evt| {
                    let c: char = evt.into();
                    c.is_ascii_digit() || c == '.'
                }
            )
                 { self.transition_with_push(LexerState::IntegerDigits, e) }
            else { self.transition_with_push(LexerState::ContinuableOperator, e) },

            // First character in a logical operator that may have two or three characters, like &&, ||, &&=, ||=
            LexerEvent::AmpersandBar(_) => self.transition_with_push(LexerState::LogicalOperator, e),

            // Digits begin a number with no leading sign
            LexerEvent::Digit(_) => self.transition_with_push(LexerState::IntegerDigits, e),

            // Superscripts begin a number with no leading sign, but cause an exponentiation operator to be inserted
            LexerEvent::Superscript(c) => self.transition_with_push(LexerState::Power, LexerEvent::Digit(Lexer::superscript_to_digit(c))),

            // Exclamation point may be prefix (logical not or not match) or suffix (factorial)
            LexerEvent::ExclamationPoint => self.transition_with_push(LexerState::Exclamation, e),

            // Tilde signifies that we are about to match a regex pattern
            LexerEvent::Tilde => self.transition_with_yield(LexerState::ExpectRegex, ParserToken::MatchOp("~".to_owned())),

            LexerEvent::EOS => self.transition_without_yield(LexerState::Goal),

            // All other events cause an Error
            _ => self.transition_to_error(e)
        }
    }

    /// Convert a superscripted digit to a normal digit, but leave unchanged all other characters.
    fn superscript_to_digit(c: char) -> char {
        match c {
            '¹' => '1',
            '²' => '2',
            '³' => '3',
            '⁴' => '4', 
            '⁵' => '5',
            '⁶' => '6',
            '⁷' => '7',
            '⁸' => '8',
            '⁹' => '9',
            '⁰' => '0',
            _ => c
        }
    }

    /// String state transitions, part of building a string literal.
    fn string(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            LexerEvent::DoubleQuote => self.transition_with_pop(LexerState::Empty, |s| Some(ParserToken::StringLiteral(s))),
            LexerEvent::Backslash => self.transition_without_yield(LexerState::StringEscape),
            _ => self.reenter_with_push(e)
        }
    }
    /// StringEscape state transitions, for escaping special characters while building a string literal.
    /// The supported escape sequences are: \n \r \t \\
    fn string_escape(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            LexerEvent::Letter('n') => self.transition_with_push_char(LexerState::String, '\n'),
            LexerEvent::Letter('r') => self.transition_with_push_char(LexerState::String, '\r'),
            LexerEvent::Letter('t') => self.transition_with_push_char(LexerState::String, '\t'),
            _ => self.transition_with_push(LexerState::String, e)
        }
    }
    /// Identifier state transitions, part of building an identifier or a function name.
    fn identifier(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            LexerEvent::Letter(_) | LexerEvent::Digit(_)
            | LexerEvent::DollarUnderscore(_) | LexerEvent::Period => self.reenter_with_push(e),
            LexerEvent::ExclamationPoint => if self.does_next_token_match_string("=".to_owned()) {
                // The exclamation point is part of a not equals operator (!=). Put it back for reuse.
                // We call to_property_chain in case there are periods in the name, indicating a series of property references.
                // If so, we make it into a PropertyChain.
                self.transition_with_pop_and_put_back(LexerState::Empty, |s| Some(ParserToken::Identifier(s).to_property_chain()), e)
            }
            else {
                // The exclamation point is the factorial operator. Yield two tokens, an identifier followed by a factorial.
                self.transition_with_double_yield(LexerState::Empty, |s| Some(ParserToken::Identifier(s).to_property_chain()), ParserToken::FactorialOp)
            },
            // If an identifier is followed by an open parenthesis, it is a function name.
            // Do not attempt to make it into a PropertyChain.
            LexerEvent::ExpressionStarter('(') => self.transition_with_pop_and_put_back(LexerState::Empty, |s| Some(ParserToken::Function(s)), e),
            LexerEvent::Space => self.transition_without_yield(LexerState::FunctionName),
            _ => self.transition_with_pop_and_put_back(LexerState::Empty, |s| Some(ParserToken::Identifier(s).to_property_chain()), e)
        }
    }

    /// FunctionName state transitions. 
    /// An identifier has been found; now see if it is followed by an open parenthesis.
    /// If is it, the identifier is a function name, otherwise an identifier.
    fn function_name(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            LexerEvent::Space => self.reenter_without_yield(),
            LexerEvent::ExpressionStarter('(') => self.transition_with_pop_and_put_back(LexerState::Empty, |s| Some(ParserToken::Function(s)), e),
            _ => self.transition_with_pop_and_put_back(LexerState::Empty, |s| Some(ParserToken::Identifier(s).to_property_chain()), e)
        }
    }

    /// ContinuableOperator state transitions, which assumes that the stack already has one of these symbols: % * + - / < = >.
    fn continuable_operator(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            LexerEvent::Equals => self.transition_with_pop_plus_event(
                LexerState::Empty, 
                |s| match s.as_str() {
                    "+=" | "-=" | "*=" | "/=" | "%=" => Some(ParserToken::AssignmentOp(s)),
                    "==" => Some(ParserToken::EqualityOp(s)),
                    "<=" | ">=" => Some(ParserToken::RelationalOp(s)),
                    _ => None // will enter Error state
                }, 
                e),
            LexerEvent::Sign(_) => self.transition_with_pop_plus_event(
                LexerState::Empty, 
                |s| match s.as_str() {
                    "++" | "--" => Some(ParserToken::IncrementDecrementOp(s)),
                    _ => None // will enter Error state
                }, 
                e),
            _ => self.transition_with_pop_and_put_back(
                LexerState::Empty, 
                |s| match s.as_str() {
                    "*" | "/" | "%" => Some(ParserToken::MultiplicativeOp(s)),
                    "+" | "-" => Some(ParserToken::AdditiveOp(s)),
                    "=" => Some(ParserToken::AssignmentOp(s)),
                    "<" | ">" => Some(ParserToken::RelationalOp(s)),
                    _ => None // Impossible case, because of transitions that could lead here
                }, 
                e)
        }
    }
    /// LogicalOperator state transitions, which may yield a logical operator like && or ||, 
    /// or an assignment operator like &&= or ||=.
    fn logical_operator(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            LexerEvent::AmpersandBar(_) => self.reenter_with_push(e),
            LexerEvent::Equals => self.transition_with_pop_plus_event(
                LexerState::Empty, 
                |s| match s.as_str() {
                    "&&=" | "||=" => Some(ParserToken::AssignmentOp(s)),
                    _ => None // will enter Error state
                }, 
                e),
            _ => self.transition_with_pop_and_put_back(
                LexerState::Empty, 
                |s| Some(ParserToken::LogicalOp(s)), 
                e)
        }
    }
    /// ExpectRegex state transitions.
    /// After seeing a match operator, we expect to see a Regex beginning delimiter (slash), but might have to toss some whitespace first.
    fn expect_regex(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            LexerEvent::Space | LexerEvent::Newline => self.reenter_without_yield(),
            LexerEvent::Slash => self.transition_without_yield(LexerState::Regex),
            _ => self.transition_to_error(e)
        }
    }

    /// Regex state transitions. Build the Regex, one character at a time.
    /// The slash delimiters are discarded.
    fn regex(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            // Found slash delimiter that ends the Regex 
            LexerEvent::Slash => self.transition_with_pop(LexerState::Empty, |s| Some(ParserToken::Regex(s))),

            // Start an escape sequence
            LexerEvent::Backslash => self.transition_with_push(LexerState::RegexEscape, e),

            // All other characters are added to the Regex without modification
            _ => self.reenter_with_push(e)
        }
    }
    fn regex_escape(&mut self, e: LexerEvent) -> Option<ParserToken> {
        self.transition_with_push(LexerState::Regex, e)
    }
    /// IntegerDigits state transitions. 
    /// Add digits to the integer part until the number terminates or we find a decimal point.
    fn integer_digits(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            // Continue building the whole number part before the decimal
            LexerEvent::Digit(_) => self.reenter_with_push(e),

            // Found decimal point, so advance to thee fractional part.
            LexerEvent::Period => self.transition_with_push(LexerState::FractionalDigits, e),

            // Found an exclamation point. Interpret it as a postfix Factorial operator, not a prefix negation operator.
            LexerEvent::ExclamationPoint => self.transition_with_double_yield(LexerState::Empty, |s| Some(ParserToken::Integer(s)), ParserToken::FactorialOp),

            // Went too far - make an Integer with no fractional part and put the new character back. It is either whitespace or part of the next token.
            _ => self.transition_with_pop_and_put_back(LexerState::Empty, |s| Some(ParserToken::Integer(s)), e)
        }
    }
    fn fractional_digits(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            // Continue building the fractional number part after the decimal and before any exponent.
            LexerEvent::Digit(_) => self.reenter_with_push(e),

            // Found the 'e' or 'E' that begins the optional exponent.
            LexerEvent::Letter('e') | LexerEvent::Letter('E') => self.transition_with_push(LexerState::ExponentSign, e),

            // Went too far - make a rational number with a fractional part and no exponent and put the new character back. 
            // It is either whitespace or part of the next token.
            _ => self.transition_with_pop_and_put_back(LexerState::Empty, |s| Some(ParserToken::Rational(s)), e)
        }
    }
    /// ExponentSign state transitions.
    /// Looks for an optional sign for the exponent, or the first digit of the exponent.
    fn exponent_sign(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            // Start building the number part of the exponent.
            LexerEvent::Digit(_) => self.transition_with_push(LexerState::ExponentDigits, e),

            // Found an optional sign for the exponent.
            LexerEvent::Sign(_) => self.transition_with_push(LexerState::ExponentDigits, e),

            _ => self.transition_to_error(e)
        }
    }
    fn exponent_digits(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            // Continue building the number part of the exponent.
            LexerEvent::Digit(_) => self.reenter_with_push(e),

            // Went too far - make a rational number with a fractional part and exponent and put the new character back.
            // It is either whitespace or part of the next token. 
            _ => self.transition_with_pop_and_put_back(LexerState::Empty, |s| Some(ParserToken::Rational(s)), e)
        }
    }

    /// Assemble a power op - a superscript number that performs an exponentiation without the ^ operator.
    fn power(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            // Continue building the PowerOp.
            LexerEvent::Superscript(c) => self.reenter_with_push(LexerEvent::Digit(Lexer::superscript_to_digit(c))),

            // Went too far - finish up and put the non-superscript character back. 
            _ => self.transition_with_pop_and_put_back(LexerState::Empty, |s| Some(ParserToken::PowerOp(s)), e)
        }
    }

    /// Exclamation state transitions.
    /// Attempt to disambiguate the exclamation point's use as a prefix logical not operator or negative match operator
    /// from its use as a postfix factorial operator.
    fn exclamation(&mut self, e: LexerEvent) -> Option<ParserToken> {
        match e {
            // The "!="" equality operator.
            LexerEvent::Equals => self.transition_with_pop_plus_event(LexerState::Empty, |s| Some(ParserToken::EqualityOp(s)), e),

            // Expression ending characters leave no doubt that the exclamation point is a postfix factorial operator, not a prefix logical not.
            LexerEvent::ExpressionEnder(_) => self.transition_with_pop_and_put_back(LexerState::Empty, |_s| Some(ParserToken::FactorialOp), e),

            // The "!~" matching operator (that does not match a regex).
            LexerEvent::Tilde => self.transition_with_pop_plus_event(LexerState::ExpectRegex, |s| Some(ParserToken::MatchOp(s)), e),

            // All other characters suggest that this is the logical not operator.
            // There are ambiguous cases not yet handled properly.
            _ => self.transition_with_pop_and_put_back(LexerState::Empty, |_s| Some(ParserToken::LogicalNotOp), e),
        }
    }
    /// In the Error state, remain in this state regardless of the incoming LexerEvent.
    fn error(&mut self, _e: LexerEvent) -> Option<ParserToken> {
        None
    }

    //..................................................................

    // Logging

    /// Append a message to Lexer.transition_log if Lexer.enable_logging is true.
    /// The message identifies:
    ///   - the position of the character read in the larger expression,
    ///   - which character was read from the expression,
    ///   - the source state
    ///   - the target state
    ///   - which ParserToken (if any) was yielded
    pub fn log_append(&mut self, position: i32, e: String, start_state: String, parser_token_opt: &Option<ParserToken>)  {
        if self.enable_logging {
            let log_entry = match parser_token_opt {
                Some(parser_token) => {
                    let token_name = parser_token.name();
                    if token_name != "Error" {
                        format!("{}. char '{}': jumps {} -> {} yields {} '{}'\n", 
                            position, e, start_state, self.current_state.to_string(), token_name, parser_token.val())
                    }
                    else {
                        format!("{}. char '{}': jumps {} -> {} yields {} '{}'\n", 
                            position, e, start_state, self.current_state.to_string(), token_name, e.to_string())
                    }
                },
                None => format!("{}. char '{}': jumps {} -> {}\n", self.events.current_position(), e, start_state, self.current_state.to_string()),
            };
            self.transition_log.push_str(&log_entry);
        }
    }

    pub fn get_log(&mut self) -> String {
        self.transition_log.to_string()
    }

}

//..................................................................

/// Implement the Iterator trait for the Lexer.
/// This holds the main iteration loop for the Lexer.
impl<'e> Iterator for Lexer<'e> {
    type Item = ParserToken;
    fn next(&mut self) -> Option<ParserToken> {
        if let Some(_) = self.token_buffer {
            return mem::replace(&mut self.token_buffer, None)
        }
        if self.has_error() {
            return None;
        }
        #[allow(irrefutable_let_patterns)]
        while let event_option = self.events.next() {
            match event_option {
                Some(event) =>
                {  
                    let previous_state = self.current_state.to_string();
                    let event_string = event.to_string();
                    // Record the character position here for logging, before any push_back occurs which will decrement the position.
                    let char_position = self.events.current_position();
                    let possible_token = match self.current_state {
                        LexerState::Start               => self.start(event),
                        LexerState::Goal         => return self.goal(event), // Reached the goal! Must return.
                        LexerState::Empty               => self.empty(event),
                        LexerState::String              => self.string(event),
                        LexerState::StringEscape        => self.string_escape(event),
                        LexerState::Identifier          => self.identifier(event),
                        LexerState::FunctionName        => self.function_name(event),
                        LexerState::ContinuableOperator => self.continuable_operator(event),
                        LexerState::LogicalOperator     => self.logical_operator(event),
                        LexerState::ExpectRegex         => self.expect_regex(event),
                        LexerState::Regex               => self.regex(event),
                        LexerState::RegexEscape         => self.regex_escape(event),
                        LexerState::IntegerDigits       => self.integer_digits(event),
                        LexerState::FractionalDigits    => self.fractional_digits(event),
                        LexerState::ExponentSign        => self.exponent_sign(event),
                        LexerState::ExponentDigits      => self.exponent_digits(event),
                        LexerState::Power               => self.power(event),
                        LexerState::Exclamation         => self.exclamation(event),
                        LexerState::Error               => self.error(event)
                    };

                    // Logging is done in two places, here and in transition_to_error.
                    // Each event must only be logged once, so make sure that we are not logging a ParserToken::Error here.
                    // See transition_to_error for a discussion of why.
                    // (It has to do with solving a chicken-and-egg problem when creating ParserToken::Error objects.)
                    let must_log = match possible_token {
                        Some(ParserToken::Error(_)) => false,
                        _ => true
                    };
                    if must_log {
                        self.log_append(char_position, event_string, previous_state, &possible_token);
                    }
                    match possible_token {
                        Some(ParserToken::Error(err)) => return Some(ParserToken::Error(err)),
                        Some(token) => return Some(token),
                        None => continue
                    }
                },
                None => return None
            }
        }
        None
    }
}

//..................................................................

#[cfg(test)]
/// Tests of the Lexer.
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    #[test]
    /// Verify that the correct sequence of ParserTokens is generated when an expression is analyzed by the Lexer.
    fn lexer_iteration() {
        let expected_tokens = vec![
            ParserToken::Identifier("answer".to_string()), 
            ParserToken::AssignmentOp("&&=".to_string()), 
            ParserToken::Rational("12.5E+2".to_string()), 
            ParserToken::MultiplicativeOp("*".to_string()), 
            ParserToken::Function("exp".to_string()), 
            ParserToken::OpenParenthesis, 
            ParserToken::Integer("1".to_string()), 
            ParserToken::AdditiveOp("-".to_string()), 
            ParserToken::OpenParenthesis, 
            ParserToken::Identifier("x".to_string()), 
            ParserToken::ExponentiationOp, 
            ParserToken::Integer("2".to_string()), 
            ParserToken::MultiplicativeOp("/".to_string()), 
            ParserToken::Integer("2".to_string()), 
            ParserToken::FactorialOp, 
            ParserToken::CloseParenthesis, 
            ParserToken::AdditiveOp("+".to_string()), 
            ParserToken::OpenParenthesis, 
            ParserToken::Identifier("x".to_string()), 
            ParserToken::ExponentiationOp, 
            ParserToken::Integer("4".to_string()), 
            ParserToken::MultiplicativeOp("/".to_string()), 
            ParserToken::Integer("4".to_string()), 
            ParserToken::FactorialOp, 
            ParserToken::CloseParenthesis, 
            ParserToken::AdditiveOp("-".to_string()), 
            ParserToken::OpenParenthesis, 
            ParserToken::Identifier("x".to_string()), 
            ParserToken::ExponentiationOp, 
            ParserToken::Integer("6".to_string()), 
            ParserToken::MultiplicativeOp("/".to_string()), 
            ParserToken::Integer("6".to_string()), 
            ParserToken::FactorialOp, 
            ParserToken::CloseParenthesis, 
            ParserToken::CloseParenthesis, 
            ParserToken::MultiplicativeOp("*".to_string()), 
            ParserToken::Identifier("π".to_string()), 
            ParserToken::MultiplicativeOp("%".to_string()), 
            ParserToken::Integer("10".to_string()), 
            ParserToken::RelationalOp(">=".to_string()), 
            ParserToken::Integer("12".to_string()), 
            ParserToken::LogicalOp("||".to_string()), 
            ParserToken::Identifier("name".to_string()), 
            ParserToken::MatchOp("~".to_string()), 
            ParserToken::Regex("^Bob".to_string())
        ];
        lexer_test_helper(
            "answer &&= 12.5E+2 * exp(1 - (x^2 / 2!) + (x^4 / 4!) - (x^6/6!)) * π % 10 >= 12 || name ~ /^Bob/", 
            expected_tokens
        );
    }

    #[test]
    /// Verify the Lexer can parse identifiers with digits, underscores, Greek letters and dollar signs.
    fn identifiers() {
        lexer_test_helper(
            "  π3_14$ \t  ", 
            vec![ParserToken::Identifier("π3_14$".to_string())]
        );
    }

    #[test]
    /// Verify the Lexer can parse string literals.
    fn string_literal() {
        lexer_test_helper(
            "  \"A literal string\"  ", 
            vec![ParserToken::StringLiteral("A literal string".to_string())]
        );
    }

    #[test]
    /// Verify the Lexer can parse string literals.
    fn string_literal_with_escaped_quote() {
        lexer_test_helper(
            "  \"A \\\"literal\\\" string\"  ", 
            vec![ParserToken::StringLiteral("A \"literal\" string".to_string())]
        );
    }

    #[test]
    /// Verify the Lexer can parse an exclamation point as a factorial, a logical not or a not equals.
    fn exclamation_point() {
        lexer_test_helper(
            "  ! (3! != 6)  ", 
            vec![
                ParserToken::LogicalNotOp,
                ParserToken::OpenParenthesis,
                ParserToken::Integer("3".to_string()),
                ParserToken::FactorialOp,
                ParserToken::EqualityOp("!=".to_string()),
                ParserToken::Integer("6".to_string()),
                ParserToken::CloseParenthesis
            ]
        );
    }

    #[test]
    /// Verify the Lexer can parse regular expressions and match operators.
    fn regex_and_match_op() {
        lexer_test_helper(
            "  $x~/abcd/ && $x !~ /^ab/ ", 
            vec![
                ParserToken::Identifier("$x".to_string()),
                ParserToken::MatchOp("~".to_string()),
                ParserToken::Regex("abcd".to_string()),
                ParserToken::LogicalOp("&&".to_string()),
                ParserToken::Identifier("$x".to_string()),
                ParserToken::MatchOp("!~".to_string()),
                ParserToken::Regex("^ab".to_string())
            ]
        );
    }

    #[test]
    /// Verify the Lexer can parse integers, floating point and numbers using exponential notation.
    fn numbers() {
        lexer_test_helper(
            " 1 23 4.5 -6 +78. -99.999 1.02E+05 34.567e-0120 0. .5 -.5 ", 
            vec![
                ParserToken::Integer("1".to_string()),
                ParserToken::Integer("23".to_string()),
                ParserToken::Rational("4.5".to_string()),
                ParserToken::Integer("-6".to_string()),
                ParserToken::Rational("+78.".to_string()),
                ParserToken::Rational("-99.999".to_string()),
                ParserToken::Rational("1.02E+05".to_string()),
                ParserToken::Rational("34.567e-0120".to_string()),
                ParserToken::Rational("0.".to_string()),
                ParserToken::Rational(".5".to_string()),
                ParserToken::Rational("-.5".to_string())
            ]
        );
    }

    #[test]
    /// Verify the Lexer can parse superscripted numbers: ¹ ² ³ ⁴ ⁵ ⁶ ⁷ ⁸ ⁹ ⁰
    fn powerop() {
        lexer_test_helper(
            "15³ - 2¹⁰", 
            vec![
                ParserToken::Integer("15".to_string()),
                ParserToken::PowerOp("3".to_string()),
                ParserToken::AdditiveOp("-".to_string()),
                ParserToken::Integer("2".to_string()),
                ParserToken::PowerOp("10".to_string()),
            ]
        );
    }

    #[test]
    /// Verify the Lexer can parse property chains with periods
    fn property_chain() {
        lexer_test_helper(
            "person.address.zip", 
            vec![
                ParserToken::PropertyChain(vec!["person".into(), "address".into(), "zip".into()])
            ]
        );
    }

    #[test]
    /// Verify the Lexer can parse compound assignment operators
    fn compound_assignment() {
        lexer_test_helper(
            "||= &&= += -= *= /= %=", 
            vec![
                ParserToken::AssignmentOp("||=".into()),
                ParserToken::AssignmentOp("&&=".into()),
                ParserToken::AssignmentOp("+=".into()),
                ParserToken::AssignmentOp("-=".into()),
                ParserToken::AssignmentOp("*=".into()),
                ParserToken::AssignmentOp("/=".into()),
                ParserToken::AssignmentOp("%=".into())
            ]
        );
    }

    #[test]
    /// Verify that an illegal character does not panic, but returns an Error
    fn illegal_character() {
        let expression = "5 + #3".to_string();
        let tokenizer = Lexer::new(&expression);
        let mut actual_tokens = Vec::new();
        actual_tokens.extend(tokenizer);
        let expected_error_position = 5;
        match actual_tokens.last() {
            Some(ParserToken::Error(err)) 
                => assert_eq!(
                    expected_error_position, err.error_position, 
                    "Error position is {} not {}.", err.error_position, expected_error_position
                ),
            Some(token) => assert!(false, format!("Ended with wrong type of token: {}", token.name())),
            None => panic!("No tokens found by Lexer")
        }

    }

    #[test]
    /// Verify that an expression with nothing but whitespace does not trigger an error, but merely returns no tokens.
    fn empty_expression() {
        let expression = "  ".to_string();
        let tokenizer = Lexer::new(&expression);
        let mut actual_tokens = Vec::new();
        actual_tokens.extend(tokenizer);
        assert_that!(actual_tokens.len()).is_equal_to(0);
    }

    //..................................................................

    // Test helper methods

    /// Tokenize an expression string and compare the tokens yielded by the Lexer's iterator to the expected tokens.
    fn lexer_test_helper(expr: &str, expected_tokens: Vec<ParserToken>) {
        let expression = expr.to_string();
        let tokenizer = Lexer::new(&expression);
        let mut actual_tokens = Vec::new();
        actual_tokens.extend(tokenizer);
        assert_that!(&actual_tokens.iter()).contains_all_of(&expected_tokens.iter());
    }
}
