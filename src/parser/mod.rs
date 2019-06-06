#![allow(dead_code)]

#[allow(unused_imports)]
use std::fmt::Result;
use crate::lexer::ParserToken;
use crate::lexer::Lexer;

pub mod shy_token;
#[allow(unused_imports)]
use shy_token::Associativity;
use shy_token::ShyToken;
use shy_token::ShyOperator;
use shy_token::ShyValue;
use shy_token::ShyScalar;

//..................................................................

#[derive(Debug)]
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

impl<'a> From<&str> for ShuntingYard<'a> {
    fn from(expression: &str) -> Self {
        ShuntingYard {
            expression_source: expression.to_string(),
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
        for ptoken in parser_tokens.iter() {
            if let ParserToken::Error(_) = ptoken {
                return Err(format!("Lexical Analyzer found this error: {}", ptoken.to_string()));
            }
        }

        // Transform ParserTokens into ShyTokens.
        self.infix_order.extend(parser_tokens.iter().map(
          |ptoken: &ParserToken| {
              let stoken: ShyToken<'a> = ptoken.clone().into();
              if stoken.is_error() {
                  println!("Parser unable to translate ParserToken {} '{}' into a ShyToken", ptoken.name(), ptoken.to_string());
              }
              stoken
          }
        ));

        // Check the ShyTokens for errors.
        if self.infix_order.iter().any(|token| token.is_error()) { 
            return Err(format!("Parser found errors")); 
        }

        // Time for Shunting Yard!
        let shunt_status = self.shunt();

        shunt_status
    }

    /// Perform the Shunting yard algorithm.
    fn shunt(&mut self) -> std::result::Result<usize,String> {
        for stoken in self.infix_order.iter() {
            match stoken {
                // Value Rule: Values are immediately copied to the postfix-ordered output stack.
                ShyToken::Value(_) => self.postfix_order.push(stoken.clone()),

                // Left Parenthesis Rule: Push all Left Parentheses onto the Operator Stack
                ShyToken::Operator(ShyOperator::OpenParenthesis) =>
                    self.operator_stack.push(ShyOperator::OpenParenthesis),

                // Right Parenthesis Rule: Pop all operators off the Operator Stack 
                //                         and push them onto the postfix-ordered output stack 
                //                         until we find matching Left Parenthesis.
                ShyToken::Operator(ShyOperator::CloseParenthesis) => {
                    loop {
                        match self.operator_stack.pop() {
                            Some(ShyOperator::OpenParenthesis) => break,
                            Some(op) => self.postfix_order.push(ShyToken::Operator(op)),
                            None => { 
                                println!("Unbalanced closing parenthesis:\n{:?}", self);
                                return Err("Unbalanced closing parenthesis".to_string())
                            }
                        }
                    }
                },

                // TODO: Handle Unary operators.

                // TODO: Handle Load / Store of variables.

                // Operator handling depends upon:
                //    - precedence
                //    - associativity
                //    - unary operators
                ShyToken::Operator(op) => {
                    loop {
                        match self.operator_stack.last() {
                            Some(ShyOperator::OpenParenthesis) | Some(ShyOperator::CloseParenthesis) => break,

                            // Higher Precedence Rule: Operator on operator stack has higher precedence than current operator, 
                            //                         so pop operator stack and push that operator onto the postfix-ordered output stack
                            //                         before pushing the current operator onto the operator stack.
                            Some(higher_precedence_op) if higher_precedence_op.precedence() > op.precedence()  => {
                                self.postfix_order.push(ShyToken::Operator(higher_precedence_op.clone()));
                                self.operator_stack.pop();
                                ()
                            },
                            // Lower Precedence Rule:  Operator on operator stack has lower precedence than current operator, 
                            //                         so stop popping off operators.  
                            Some(lower_precedence_op) if lower_precedence_op.precedence() < op.precedence()  => {
                                break;
                            },
                            // Left Associative Rule:  Operators have same precedence, and operator on stack has left associativity,
                            //                         so pop operator stack and push it onto postfix-ordered output stack.
                            Some(equal_precedence_op) if equal_precedence_op.precedence() == op.precedence() 
                                                         && equal_precedence_op.associativity() == Associativity::Left  => {
                                self.postfix_order.push(ShyToken::Operator(*equal_precedence_op));
                                self.operator_stack.pop();
                                ()
                            },
                            // Right Associative Rule: Operators have same precedence, and operator on stack has right associativity,
                            //                         so stop popping off operators.
                            _ => break
                         }
                    }
                    self.operator_stack.push(*op)
                },
                
                // Function Rule: Functions call for an operator to be pushed on the operator stack and a value (the function name)
                //                to be pushed on the postfix-ordered output stack.
                //                Assume that the operator is a ShyOperator::FunctionCall and the value is a ShyValue::FunctionName.
                ShyToken::OperatorWithValue(op, value) => {
                    self.postfix_order.push(ShyToken::Value(value.clone()));
                    self.operator_stack.push(*op)
                },
                // This is an error case that should not occur currently.
                // Once when we support shortcut operators for and (&&) and or (||),
                // an error in a branch not taken should be overlooked, so defer to the evaluation of the expression.
                _ => self.postfix_order.push(stoken.clone())
            }
        }
        // End of Input Rule: Once there are no more operators expected, transfer all remaining operators 
        //                    from the operator stack to the postfix-ordered output stack.
        //                    This reverses the token's original order, by popping from one stack and pushing onto the other.
        loop {
            match self.operator_stack.pop() {
                Some(ShyOperator::OpenParenthesis) => return Err("Unbalanced opening parenthesis".to_string()),
                Some(op) => self.postfix_order.push(ShyToken::Operator(op)),
                None => break
            }
        }
        Ok(self.postfix_order.len())
    }

    pub fn compile(&mut self) -> std::result::Result<usize,String> {
        match self.parse() {
            Ok(token_count) => {
                Ok(token_count)
            },
            Err(s) => Err(s)
        }
    }

    // TODO: Implement execute method.
}

//..................................................................

#[cfg(test)]
/// Tests of the ShuntingYard.
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    #[test]
    /// Verify that the tokens for "2 + 2" are correctly rearranged into infix order.
    fn compile_2_plus_2() {
        compile_test_case(
            "2 + 2", 
            vec![
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(2))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(2))),
            ShyToken::Operator(ShyOperator::Add),
        ]);
    }

    #[test]
    /// Verify that operator precedence rules are followed.
    fn operator_precedence() {
        compile_test_case(
            "2 + 3 * 4 - 5", 
            vec![
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(2))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(3))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(4))),
            ShyToken::Operator(ShyOperator::Multiply),
            ShyToken::Operator(ShyOperator::Add),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(5))),
            ShyToken::Operator(ShyOperator::Subtract),
        ]);
    }

    #[test]
    /// Verify that parentheses rules are followed.
    fn parentheses() {
        compile_test_case(
            "(2 + 3) * (4 - 5)", 
            vec![
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(2))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(3))),
            ShyToken::Operator(ShyOperator::Add),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(4))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(5))),
            ShyToken::Operator(ShyOperator::Subtract),
            ShyToken::Operator(ShyOperator::Multiply),
        ]);
    }

    #[test]
    /// Verify that an error with too many closing parentheses is generated.
    fn unbalanced_closing_parentheses() {
        let mut shy: ShuntingYard = "(2 + 3) * (4 - 5))".into();
        match shy.compile() {
            Err(msg) => assert_that(&msg).contains("Unbalanced"),
            _ => assert!(false, "Did not return error")
        }
    }

    #[test]
    /// Verify that an error with too many opening parentheses is generated.
    fn unbalanced_opening_parentheses() {
        let mut shy: ShuntingYard = "((2 + 3) * (4 - 5)".into();
        match shy.compile() {
            Err(msg) => assert_that(&msg).contains("Unbalanced"),
            _ => assert!(false, "Did not return error")
        }
    }

    #[test]
    /// Verify that a factorial function is properly handled, both for literal integers and for a variable.
    fn factorial() {
        compile_test_case(
            "(10 + 3! - n! /2) != 7", 
            vec![
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(10))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(3))),
            ShyToken::Operator(ShyOperator::Factorial),
            ShyToken::Operator(ShyOperator::Add),
            ShyToken::Value(ShyValue::Variable("n".to_string())),
            ShyToken::Operator(ShyOperator::Factorial),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(2))),
            ShyToken::Operator(ShyOperator::Divide),
            ShyToken::Operator(ShyOperator::Subtract),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(7))),
            ShyToken::Operator(ShyOperator::NotEquals),
        ]);
    }

    fn compile_test_case(expression: &str, expected_tokens: Vec<ShyToken>) {
        let mut shy: ShuntingYard = expression.into();
        match shy.compile() {
            Ok(token_count) => assert_that!(token_count).is_equal_to(expected_tokens.len()),
            Err(msg) => {
               println!("Shy:\n{:?}", shy);
               assert!(false, format!("Error compiling: {}", msg))
            }
        }
        assert!(expected_tokens.iter().eq(shy.postfix_order.iter()), );
    }


}
