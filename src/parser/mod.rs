#![allow(dead_code)]

#[allow(unused_imports)]
use std::fmt::Result;
use std::marker::PhantomData;
use crate::lexer::ParserToken;
use crate::lexer::Lexer;

pub mod shy_token;
#[allow(unused_imports)]
use shy_token::ShyToken;
use shy_token::ShyValue;
use shy_token::ShyScalar;

pub mod factorial;
pub mod associativity;
use associativity::Associativity;

pub mod execution_context;
use execution_context::ExecutionContext;

pub mod shy_operator;
use shy_operator::ShyOperator;

//..................................................................

/// Implements the Shunting Yard algorithm for converting a series of tokens in infix order 
/// into a stack of values and operators in postfix order.
/// Once reordered, the result of the expression may be efficiently computed from the postfix stack of tokens.
#[derive(Debug)]
pub struct ShuntingYard<'a> {
    marker: PhantomData<&'a i64>,

    pub expression_source: String,

    infix_order: Vec<ShyToken>,

    postfix_order: Vec<ShyToken>,

    operator_stack: Vec<ShyOperator>,
}

impl<'a> From<String> for ShuntingYard<'a> {
    fn from(expression: String) -> Self {
        ShuntingYard {
            marker: PhantomData,
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
            marker: PhantomData,
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
              let stoken: ShyToken = ptoken.clone().into();
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
        // Need to clone infix_order to placate the borrow-checker, otherwise I cannot call the reduce method.
        let infix_order_copy = self.infix_order.clone();
        for stoken in infix_order_copy.iter() {
            // Variable Rule: Check for rvalues on postfix-ordered output stack.
            //                If we find an rvalue, push a Load operator onto the postfix-ordered output stack.
            //                Variable values must be loaded from context before the other operators can act upon them. 
            //                Only the assignment, post-increment and post-decrement operators perform their own 
            //                loading from and saving to the context.
            if self.is_rvalue_on_stack(&stoken) {
                self.postfix_order.push(ShyToken::Operator(ShyOperator::Load));
            }
            match stoken {
                // Value Rule: Values are immediately copied to the postfix-ordered output stack.
                //             This includes ShyValue::Variable, which may need to be followed by 
                //             a Load token at the top of the next loop.
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

                // Precedence & Associativity Rules:
                ShyToken::Operator(op) => {
                    self.reduce(op.clone());
                    self.operator_stack.push(*op)
                },
                
                // Function Rule: Functions call for an operator to be pushed on the operator stack and a value (the function name)
                //                to be pushed on the postfix-ordered output stack.
                //                Assume that the value is a ShyValue::FunctionName.
                ShyToken::OperatorWithValue(ShyOperator::FunctionCall, value) => {
                    self.postfix_order.push(ShyToken::Value(value.clone()));
                    self.operator_stack.push(ShyOperator::FunctionCall)
                },

                // Power Rule: Power operations call for an exponentiation operator to be pushed on the operator stack and a value (the exponent)
                //             to be pushed on the postfix-ordered output stack.
                //             Assume that the value is a ShyValue::Integer.
                ShyToken::OperatorWithValue(ShyOperator::Exponentiation, value) => {
                    self.reduce(ShyOperator::Exponentiation);
                    self.postfix_order.push(ShyToken::Value(value.clone()));
                    self.operator_stack.push(ShyOperator::Exponentiation)
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

    /// Decide if the top of the postfix_order stack is a Variable AND it should be considered 
    /// an rvalue whose value should be loaded from context,
    /// not an lvalue into which a result should be stored.
    /// If an rvalue, we should push a Load operator token onto postfix_order. (Pushing Load is not done here.)
    /// It is an rvalue if the top of the stack holds a ShyToken::Value(ShyValue::Variable) 
    /// and the given stoken is NOT an assignment operator or post-increment or post-decrement operator.
    fn is_rvalue_on_stack(&self, stoken: &ShyToken) -> bool {
        match &self.postfix_order.last() {
            Some(ShyToken::Value(ShyValue::Variable(_))) => {
                match stoken {
                    ShyToken::Operator(op) if !op.is_assignment() => { true }, // Top of stack is a Variable used as an rvalue
                    _ => false // Token is not an operator (unlikely) or is an assignment operator (making the variable an lvalue) 
                }
            },
            _ => false // Top of stack is NOT a Variable
        }
    }

    /// Apply the rules for precedence and associativity to reduce the operator_stack
    /// by moving some operators to the postfix_order stack.
    fn reduce(&mut self, op: ShyOperator) {
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
    }

    /// Compile the expression into a postfix ordered series of tokens and return the Expression.
    pub fn compile(mut self) -> std::result::Result<Expression<'a>,String> {
        match self.parse() {
            Ok(_) => {
                // TODO: Optimizations like constant folding, And/Or operator short-cutting, branching.
                Ok(Expression { 
                    marker: PhantomData,
                    expression_source: self.expression_source.clone(),
                    postfix_order: self.postfix_order.clone()
                })
            },
            Err(s) => Err(format!("{}\n{:?}", s, self))
        }
    }

}

//..................................................................

/// Compiled Expression that can be executed.
#[derive(Debug)]
pub struct Expression<'a> {
    marker: PhantomData<&'a i64>,

    /// Infix Expression as a string before it was compiled
    pub expression_source: String,

    /// The constants, variable references and operators parsed from the expression_source and rearranged into postfix order.
    pub postfix_order: Vec<ShyToken>
}

impl<'a> Expression<'a> {
    /// Execute an already compiled expression against the given ExecutionContext.  
    pub fn exec(&self, context: &mut ExecutionContext<'a>) -> std::result::Result<ShyValue,String> {
        let mut output_stack : Vec<ShyValue> = vec![];
        for token in self.postfix_order.iter().cloned() {
            match token {
                ShyToken::Value(value) => output_stack.push(value),
                ShyToken::Operator(op) => { 
                    Self::operate(&mut output_stack, op, context);
                    ()
                },
                _ => output_stack.push(ShyValue::error("Invalid token in expression".to_string()))
            }
        }
        // The final result of the expression is on top of the stack; pop it off and return it. 
        match output_stack.pop() {
            Some(value) => Ok(value),
            None => Err("Expression stack is empty".to_string())
        }
    }

    /// Check if the stack has enough items to satisfy the needs of the operator
    fn is_stack_size_sufficient(output_stack: &mut Vec<ShyValue>, op: ShyOperator) -> bool {
        op.arguments() >= output_stack.len() 
    }

    /// Check if the stack is topped by an error value
    fn does_stack_have_error(output_stack: &mut Vec<ShyValue>) -> bool {
        match output_stack.last() {
            Some(ShyValue::Scalar(ShyScalar::Error(_))) => true,
            _ => false
        }
    }

    /// Apply an operator, removing tokens from the stack, computing a result, and pushing the result back on the stack.
    fn operate(output_stack: &mut Vec<ShyValue>, op: ShyOperator, context: &mut ExecutionContext<'a>) -> ShyValue {
        if Self::does_stack_have_error(output_stack) { return output_stack.last().unwrap().clone(); }
        if !Self::is_stack_size_sufficient(output_stack, op)   {
            output_stack.clear();
            let stack_empty = ShyValue::error(format!("Too few values on stack for operation {:?}", op));
            output_stack.push(stack_empty.clone());
            return stack_empty;
        }
        // If a unary operator, arg1 is the sole argument. 
        // If a binary operator, arg1 is the left operand.
        let mut arg1: ShyValue = 0.into();

        // If a unary operator, arg2 is unused.
        // If a binary operator, arg2 is the right operand.
        let mut arg2: ShyValue = 0.into();
        let mut arg3: ShyValue = 0.into();

        match op.arguments() {
            1 => {
                arg1 = output_stack.pop().unwrap();
            },
            2 => {
                arg2 = output_stack.pop().unwrap();
                arg1 = output_stack.pop().unwrap();
            },
            3 => {
                arg3 = output_stack.pop().unwrap();
                arg2 = output_stack.pop().unwrap();
                arg1 = output_stack.pop().unwrap();
            },
            _ => ()
        }
        let unimplemented = ShyValue::error(format!("Operation {} unimplemented", op.to_string()));
        let result = match op {
            ShyOperator::Load => ShyValue::load(&arg1, context),
            ShyOperator::Store => unimplemented,
            ShyOperator::Semicolon => {
                // Semicolons separate individual statements.
                // When we encounter one, wipe the stack clear to prepare for the next statement. 
                // Return the result of the previous statement. 
                // If the previous statement left the stack empty, return a NAN wrapped as a ShyValue. 
                if output_stack.len() == 0 {
                    return std::f64::NAN.into();
                }
                let intermediate_result = output_stack.pop().unwrap();
                output_stack.clear();
                return intermediate_result;
            },
            ShyOperator::FunctionCall => ShyValue::call(&arg1, &arg2, context),
            ShyOperator::OpenParenthesis => unimplemented,
            ShyOperator::CloseParenthesis => unimplemented,
            ShyOperator::Comma => ShyValue::comma(&arg1, &arg2),
            ShyOperator::OpenBracket => unimplemented,
            ShyOperator::CloseBracket => unimplemented,
            ShyOperator::Member => unimplemented,
            ShyOperator::Power => ShyValue::power(&arg1, &arg2),
            ShyOperator::Exponentiation => context.call("exp".to_string(), arg1),
            ShyOperator::PrefixPlusSign => ShyValue::prefix_plus(&arg1),
            ShyOperator::PrefixMinusSign => ShyValue::prefix_minus(&arg1),
            ShyOperator::PostIncrement => ShyValue::post_increment(&arg1, context),
            ShyOperator::PostDecrement => ShyValue::post_decrement(&arg1, context),
            ShyOperator::SquareRoot => ShyValue::sqrt(&arg1),
            ShyOperator::LogicalNot => ShyValue::not(&arg1),
            ShyOperator::Factorial => ShyValue::factorial(&arg1),
            ShyOperator::Match => ShyValue::matches(&arg1, &arg2),
            ShyOperator::NotMatch => ShyValue::not_matches(&arg1, &arg2),
            ShyOperator::Multiply => ShyValue::multiply(&arg1, &arg2),
            ShyOperator::Divide => ShyValue::divide(&arg1, &arg2),
            ShyOperator::Mod => ShyValue::modulo(&arg1, &arg2),
            ShyOperator::Add => ShyValue::add(&arg1, &arg2),
            ShyOperator::Subtract => ShyValue::subtract(&arg1, &arg2),
            ShyOperator::LessThan => ShyValue::less_than(&arg1, &arg2),
            ShyOperator::LessThanOrEqualTo => ShyValue::less_than_or_equal_to(&arg1, &arg2),
            ShyOperator::GreaterThan => ShyValue::greater_than(&arg1, &arg2),
            ShyOperator::GreaterThanOrEqualTo => ShyValue::greater_than_or_equal_to(&arg1, &arg2),
            ShyOperator::Equals => ShyValue::equals(&arg1, &arg2),
            ShyOperator::NotEquals => ShyValue::not_equals(&arg1, &arg2),
            ShyOperator::And => ShyValue::and(&arg1, &arg2), 
            ShyOperator::Or => ShyValue::or(&arg1, &arg2), 
            ShyOperator::Ternary => unimplemented,
            ShyOperator::Assign => ShyValue::assign(&arg1, &arg2, context),
            ShyOperator::PlusAssign => ShyValue::plus_assign(&arg1, &arg2, context),
            ShyOperator::MinusAssign => ShyValue::minus_assign(&arg1, &arg2, context),
            ShyOperator::MultiplyAssign => ShyValue::multiply_assign(&arg1, &arg2, context),
            ShyOperator::DivideAssign => ShyValue::divide_assign(&arg1, &arg2, context),
            ShyOperator::ModAssign => ShyValue::modulo_assign(&arg1, &arg2, context),
            ShyOperator::AndAssign => ShyValue::and_assign(&arg1, &arg2, context),
            ShyOperator::OrAssign => ShyValue::or_assign(&arg1, &arg2, context),
            _ => {
                output_stack.clear();
                let unsupported = ShyValue::error(format!("Invalid operator {:?}", op));
                output_stack.push(unsupported.clone());
                unsupported
            }
        };
        output_stack.push(result.clone());
        result
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

    /// Verify that the tokens for "2 + 2" are correctly rearranged into infix order.
    #[test]
    fn compile_2_plus_2() {
        compile_test_case(
            "2 + 2", 
            vec![
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(2))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(2))),
            ShyToken::Operator(ShyOperator::Add),
        ]);
    }

    /// Verify that operator precedence rules are followed.
    #[test]
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

    /// Verify that parentheses rules are followed.
    #[test]
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

    /// Verify that an error with too many closing parentheses is generated.
    #[test]
    fn unbalanced_closing_parentheses() {
        let shy: ShuntingYard = "(2 + 3) * (4 - 5))".into();
        match shy.compile() {
            Err(msg) => assert_that(&msg).contains("Unbalanced"),
            _ => assert!(false, "Did not return error")
        }
    }

    /// Verify that an error with too many opening parentheses is generated.
    #[test]
    fn unbalanced_opening_parentheses() {
        let shy: ShuntingYard = "((2 + 3) * (4 - 5)".into();
        match shy.compile() {
            Err(msg) => assert_that(&msg).contains("Unbalanced"),
            _ => assert!(false, "Did not return error")
        }
    }

    /// Verify that a factorial function is properly handled, both for literal integers and for a variable.
    #[test]
    fn factorial() {
        compile_test_case(
            "(10 + 3! - n! /2) != 7", 
            vec![
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(10))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(3))),
            ShyToken::Operator(ShyOperator::Factorial),
            ShyToken::Operator(ShyOperator::Add),
            ShyToken::Value(ShyValue::Variable("n".to_string())),
            ShyToken::Operator(ShyOperator::Load),
            ShyToken::Operator(ShyOperator::Factorial),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(2))),
            ShyToken::Operator(ShyOperator::Divide),
            ShyToken::Operator(ShyOperator::Subtract),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(7))),
            ShyToken::Operator(ShyOperator::NotEquals),
        ]);
    }

    /// Verify that logical not (a prefix operator) is handled properly when parenthesized.
    #[test]
    fn logical_not_parenthesized() {
        compile_test_case(
            "!(a || b)", 
            vec![
            ShyToken::Value(ShyValue::Variable("a".to_string())),
            ShyOperator::Load.into(),
            ShyToken::Value(ShyValue::Variable("b".to_string())),
            ShyOperator::Load.into(),
            ShyOperator::Or.into(),
            ShyOperator::LogicalNot.into(),
        ]);
    }

    #[test]
    fn string_match() {
        compile_test_case(
            r#"name ~ /^Paul/ && color == "blue""#, 
            vec![
            ShyToken::Value(ShyValue::Variable("name".to_string())),
            ShyOperator::Load.into(),
            // TODO: Implement ShyScalar::Regex
            ShyToken::Value(ShyValue::Scalar(ShyScalar::String("^Paul".to_string()))),
            ShyOperator::Match.into(),
            ShyToken::Value(ShyValue::Variable("color".to_string())),
            ShyOperator::Load.into(),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::String("blue".to_string()))),
            ShyOperator::Equals.into(),
            ShyOperator::And.into(),
        ]);
    }

    /// Verify that logical not (a prefix operator) is handled properly when parenthesized.
    #[test]
    fn function_call() {
        compile_test_case(
            "0.5 + sin(π/6)", 
            vec![
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Rational(0.5))),
            ShyToken::Value(ShyValue::FunctionName("sin".to_string())),
            ShyToken::Value(ShyValue::Variable("π".to_string())),
            ShyOperator::Load.into(),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(6))),
            ShyOperator::Divide.into(),
            ShyOperator::FunctionCall.into(),
            ShyOperator::Add.into(),
        ]);
    }

    /// Verify that raising a value to a power has the correct precedence and associativity.
    #[test]
    fn power() {
        compile_test_case(
            "3*2¹⁰/5", 
            vec![
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(3))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(2))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(10))),
            ShyOperator::Exponentiation.into(),
            ShyOperator::Multiply.into(),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(5))),
            ShyOperator::Divide.into(),
        ]);
    }

    /// Verify that raising a value to a power has the correct precedence and associativity.
    #[test]
    fn square_root() {
        compile_test_case(
            "3*√2/5", 
            vec![
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(3))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(2))),
            ShyOperator::SquareRoot.into(),
            ShyOperator::Multiply.into(),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(5))),
            ShyOperator::Divide.into(),
        ]);
    }

    #[test]
    fn context_call() {
        let ctx = ExecutionContext::default();
        let actual = ctx.call("exp".to_string(), 0_f64.into());
        match actual {
            ShyValue::Scalar(ShyScalar::Rational(x)) => assert_that(&x).is_close_to(1_f64, 0.000001),
            _ => assert!(false, format!("Wrong type of value returned from call {:?}", actual))
        }
    }

    /// Compile a simple formula: "x = 1".
    #[test]
    fn compile_simple_assignment() {
        compile_test_case(
            "x = 1", 
            vec![
            ShyToken::Value(ShyValue::Variable("x".to_string())),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(1))),
            ShyOperator::Assign.into(),
        ]);
    }

    /// Execute a simple formula: "x = 1"
    #[test]
    fn exec_simple_assignment() {
        let mut ctx = ExecutionContext::default();
        execute_test_case("x = 1", &mut ctx, &1.into()); 
    }

//..................................................................

// Test helper methods

    /// Compile an expression but do not execute it; compare the tokens generated to the expected sequence.
    fn compile_test_case(expression: &str, expected_tokens: Vec<ShyToken>) {
        let shy: ShuntingYard = expression.into();
        match shy.compile() {
            Ok(expr) => {
                if expr.postfix_order.len() != expected_tokens.len() {
                    println!("Expression:\n{:?}", expr);
                }
                assert_that!(expr.postfix_order.len()).is_equal_to(expected_tokens.len());
                assert!(expected_tokens.iter().eq(expr.postfix_order.iter()), )
            },
            Err(msg) => {
                assert!(false, format!("Error compiling: {}", msg))
            }
        }
    }

    fn execute_test_case(expression: &str, ctx: &mut ExecutionContext, expected: &ShyValue) {
        let shy: ShuntingYard = expression.into();
        match shy.compile() {
            Ok(expr) => {
                match expr.exec(ctx) {
                    Ok(actual) => asserting(&format!("exec of {}", expression.to_string())).that(&actual).is_equal_to(expected),
                    Err(msg) => assert!(false, format!("Error executing {}: {}", expression, msg))
                }
            },
            Err(msg) => { assert!(false, format!("Error compiling {}: {}", expression, msg)) }
        }

    }


}
