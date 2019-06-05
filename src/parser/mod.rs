#![allow(dead_code)]

#[allow(unused_imports)]
use std::fmt::Result;
use crate::lexer::ParserToken;
use crate::lexer::Lexer;
use crate::lexer;

pub mod shy_token;
#[allow(unused_imports)]
use shy_token::ShyToken;
use shy_token::ShyOperator;

//..................................................................

pub struct ShuntingYard<'a> {
    pub expression_source: String,

    infix_order: Vec<ShyToken<'a>>,

    postfix_order: Vec<ShyToken<'a>>,

    operator_stack: Vec<ShyOperator>,
}

impl<'a> From<String> for ShuntingYard<'a> {
    fn from(expression: String) -> Self {
        ShuntingYard {
            expression_source: expression,
            infix_order: vec![],
            postfix_order: vec![],
            operator_stack: vec![]
        }
    }
}

impl<'a> ShuntingYard<'a> {
    /// Parse the expression into tokens and apply the shunting yard algorithm to rearrange the tokens into postfix order.
    /// Return the number of tokens parsed, or an error.
    fn parse(&mut self) -> std::result::Result<usize,String> {
        let lexer = Lexer::new(&self.expression_source);
        let mut parser_tokens = Vec::new();

        // Read expression and parse it into ParserTokens.
        parser_tokens.extend(lexer);

        self.infix_order.extend(parser_tokens.iter().map(
          |ptoken: &ParserToken| {
              let stoken: ShyToken<'a> = ptoken.clone().into();
              stoken
          }
        ));

        // Transform the ParserTokens into ShyTokens.
        let error_count = self.infix_order.iter().filter(
          |token| match token { 
              ShyToken::Error => true,
              _ => false
          }
          ).count();
        match error_count {
            0 => Ok(parser_tokens.len()),
            _ => return Err(format!("Lexical analyzer found {} errors", error_count))
        }
    }

    pub fn compile(&mut self) -> std::result::Result<usize,String> {
        match self.parse() {
            Ok(_) => {
                Ok(0)
            },
            Err(s) => Err(s)
        }
    }
}

//..................................................................

#[cfg(test)]
/// Tests of the ShuntingYard.
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;


}
