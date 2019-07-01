#![allow(dead_code)]
#![recursion_limit="128"]

extern crate itertools;
#[macro_use] extern crate custom_derive;
#[macro_use] extern crate enum_derive;
extern crate regex;

#[macro_use]
extern crate lazy_static;

#[allow(unused_macros)]
#[cfg(test)]
extern crate spectral;

mod parser;

mod lexer;

use lexer::Lexer;
//use lexer::lexer_state::LexerState;
#[allow(unused_imports)]
use parser::ShuntingYard;


fn main() {
    use std::io::{stdin,stdout,Write};
    let mut s=String::new();
    print!("Please enter an expression: ");
    let _=stdout().flush();
    stdin().read_line(&mut s).expect("Did not enter a correct string");
    if let Some('\n')=s.chars().next_back() {
        s.pop();
    }
    if let Some('\r')=s.chars().next_back() {
        s.pop();
    }
    println!("You entered: {}",s);

    let expression = s.to_string();
    let tokenizer = Lexer::new(&expression);
    for token in tokenizer {
        println!("Token: {} {}", token.name(), token.to_string());
    }
}

