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
use parser::execution_context::ExecutionContext;

#[allow(unused_imports)]
use parser::ShuntingYard;

fn repl() {
    use std::io::{stdin,stdout,Write};
    let mut ctx = ExecutionContext::default();
    let mut trace_on = false;
    loop {
        let mut input=String::new();
        print!("> ");
        let _=stdout().flush();
        stdin().read_line(&mut input).expect("I/O Error entering expression");
        let command = input.trim();
        if command == "quit" || command == "exit" { break; }
        if command == "trace on" {
            trace_on = true; 
            continue; 
        }
        if command == "trace off" {
            trace_on = false; 
            continue; 
        }
        let shy: ShuntingYard = command.into();
        match shy.compile() {
            Ok(mut expr) => {
                if trace_on { expr.trace(&mut ctx); }
                match expr.exec(&mut ctx) {
                    Ok(actual) => println!("{:?}", actual),
                    Err(msg) => println!("Error executing {}: {}", command, msg)
                }
            },
            Err(msg) => { println!("Error compiling {}: {}", command, msg) }
        }
    }

}


fn main() {
    repl();
}

