#![allow(dead_code)]
#![recursion_limit="128"]

//! # Shy: Shunting Yard Expression Parser and Evaluator
//!
//! `shy` is a rules engine that can compile infix expressions into postfix expressions, then execute them.
//! 
//! This application has three main modules (so far): 
//! 
//!    1. `lexer` is the lexical analyzer that tokenizes mathematical expressions given as strings.
//!    2. `parser` executes the **Shunting Yard** algorithm that compiles the tokens into an 
//!        expression and executes the expression.
//!    3. `cache` implements an approximate LRU (least recently used) cache. 
//!       Used together with the expression compiler to reuse the formulas that have already been compiled,
//!       this speeds the execution of the expressions up tenfold. 
//!    

extern crate itertools;
#[macro_use] extern crate custom_derive;
#[macro_use] extern crate enum_derive;
extern crate regex;

use std::convert::TryInto;
use std::env;

#[macro_use]
extern crate lazy_static;

#[allow(unused_macros)]
#[cfg(test)]
extern crate spectral;
extern crate rand;

mod cache;
mod parser;
mod lexer;
mod rule;
mod graph;
mod service;

use parser::execution_context::ExecutionContext;
// use parser::shy_scalar::ShyScalar;
use parser::shy_token::ShyValue;
use service::shy_service;

#[allow(unused_imports)]
use parser::ShuntingYard;

/// Read-execute-print-loop - an terminal-based interactive formula executor.
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
                if trace_on { let _ = expr.trace(&mut ctx); }
                match expr.exec(&mut ctx) {
                     
                    Ok(ShyValue::Scalar(actual)) => {
                        let s_maybe : Result<String, &'static str> = actual.try_into();
                        match s_maybe {
                            Ok(s) => println!("{}", s),
                            Err(msg) => println!("Error executing {}: {}", command, msg)
                        }
                    },
                    Ok(actual_value) => println!("{:?}", actual_value),
                    Err(msg) => println!("Error executing {}: {}", command, msg)
                }
            },
            Err(msg) => { println!("Error compiling {}: {}", command, msg) }
        }
    }

}

/// Main entry point for application. 
/// 
/// If no command line arguments are supplied (besides the application executable name),
/// a REPL is started and the user can execute expressions
/// at the command line and have them compiled and evaluated. 
/// 
/// If arguments are supplied: 
/// 
///  - cargo run repl
///  - cargo run service <ip> <port>
/// 
/// The first syntax again runs the repl.
/// 
/// For the second syntax, a Web Service is started.
///   - If ip and port are omitted, run the server at this ip address: 127.0.0.1:8088
///   - If port is omitted, use port 8088. 
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 || args[1] == "repl" {
        repl();
    }
    else {
        let mut ip = "127.0.0.1";
        let mut port = "8088";
        if args.len() >= 4 {
            port = &args[3];
        }
        if args.len() >=3 {
            ip = &args[2];
        }
        shy_service(ip, port);
    }
}

