#![allow(dead_code)]

#[allow(unused_imports)]
use std::fmt::Result;
use std::marker::PhantomData;
use crate::lexer::parser_token::ParserToken;
use crate::lexer::Lexer;

pub mod indent;
// use indent::*;


pub mod shy_token;
#[allow(unused_imports)]
use shy_token::ShyToken;
use shy_token::ShyValue;

pub mod factorial;
pub mod associativity;
pub mod voting_rule;
use associativity::Associativity;

pub mod execution_context;
use execution_context::ExecutionContext;

pub mod shy_scalar;
use shy_scalar::ShyScalar;

pub mod shy_operator;
use shy_operator::ShyOperator;

pub mod shy_association;
pub mod shy_object;


//..................................................................

/// Implements the Shunting Yard algorithm for converting a series of tokens in infix order 
/// into a stack of values and operators in postfix order.
/// Once reordered, the result of the expression may be efficiently computed from the postfix stack of tokens.
#[derive(Debug)]
pub struct ShuntingYard<'a> {
    /// Weird Rust idiom to define the desired variance/covariance since you are not allowed to have an unbounded lifetime.
    /// See http://web.mit.edu/rust-lang_v1.25/arch/amd64_ubuntu1404/share/doc/rust/html/nomicon/phantom-data.html
    marker: PhantomData<&'a i64>,

    /// The input expression prior to parsing.
    pub expression_source: String,

    /// Tokenized form of the input expression, still in infix order.
    infix_order: Vec<ShyToken>,

    /// Tokens rearranged into postfix order as a result of shunting yard.
    /// This form may have additional tokens added that were not present in the infix_order list of tokens.
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
        let mut op_count_since_value = 0;
        for stoken in infix_order_copy.iter() {
            // Variable Rule: Check for rvalues on postfix-ordered output stack.
            //                If we find an rvalue, push a Load operator onto the postfix-ordered output stack.
            //                Variable values must be loaded from context before the other operators can act upon them. 
            //                Only the assignment, post-increment and post-decrement operators perform their own 
            //                loading from and saving to the context.
            if self.is_rvalue_on_stack(&stoken) && op_count_since_value == 0 {
                self.postfix_order.push(ShyToken::Operator(ShyOperator::Load));
            }
            op_count_since_value += 1;
            match stoken {
                // Value Rule, Part 1: Values are immediately copied to the postfix-ordered output stack.
                //                     This includes ShyValue::Variable, which may need to be followed by 
                //                     a Load token at the top of the next loop.
                ShyToken::Value(_) => { 
                    op_count_since_value = 0;
                    self.postfix_order.push(stoken.clone())
                },

                // Semicolon Rule: Force the moving of all operators on the operator_stack to the postfix_order stack,
                //                 followed by the semicolon itself.
                ShyToken::Operator(ShyOperator::Semicolon) => {
                    self.reduce_all()
                },

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
        // Variable Rule, Part 2: If the last token of the expression is a variable, 
        //                        we cannot look ahead to see what the next operator is.
        //                        Automatically add a Load operator. 
        //                        This must be done before copying the remaining operators from the operator stack!
        if self.is_last_token_variable() {
            self.postfix_order.push(ShyToken::Operator(ShyOperator::Load));
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

    /// Decide if the top of the postfix_order stack is a Variable or PropertyChain AND it should be considered 
    /// an rvalue whose value should be loaded from context,
    /// not an lvalue into which a result should be stored.
    /// If an rvalue, we should push a Load operator token onto postfix_order. (Pushing Load is not done here.)
    /// It is an rvalue if the top of the stack holds a ShyToken::Value(ShyValue::Variable) or a ShyToken::Value(ShyValue::PropertyChain)
    /// and the given stoken is NOT an assignment operator or post-increment or post-decrement operator.
    fn is_rvalue_on_stack(&self, stoken: &ShyToken) -> bool {
        let is_assignment_operator = match stoken {
            ShyToken::Operator(op) => op.is_assignment(), 
            _ => false 
        };
        match &self.postfix_order.last() {
            Some(ShyToken::Value(ShyValue::Variable(_))) => !is_assignment_operator,
            Some(ShyToken::Value(ShyValue::PropertyChain(_))) => !is_assignment_operator,
            _ => false // Top of stack is NOT a Variable
        }
    }

    /// True if the last token in postfix order is a variable or a property chain.
    fn is_last_token_variable(&self) -> bool {
        match &self.postfix_order.last() {
            Some(ShyToken::Value(ShyValue::Variable(_))) => true,
            Some(ShyToken::Value(ShyValue::PropertyChain(_))) => true,
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

    /// Move all tokens from operator_stack to postfix_order stack, in LIFO order.
    fn reduce_all(&mut self) {
        loop {
            match self.operator_stack.last() {
                Some(op) => {
                    self.postfix_order.push(ShyToken::Operator(op.clone()));
                    self.operator_stack.pop();
                    ()
                },
                // No more tokens
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
                    postfix_order: self.postfix_order.clone(),
                    trace_on: false
                })
            },
            Err(s) => Err(format!("{}\n{:?}", s, self))
        }
    }

}

//..................................................................

/// Compiled Expression that can be executed.
#[derive(Debug, Clone)]
pub struct Expression<'a> {
    marker: PhantomData<&'a i64>,

    /// Infix Expression as a string before it was compiled
    pub expression_source: String,

    /// The constants, variable references and operators parsed from the expression_source and rearranged into postfix order.
    /// This list of tokens was generated by the shunting yard algorithm.
    pub postfix_order: Vec<ShyToken>,

    /// If true, a trace of the execution of the expression is printed as a diagnostic.
    trace_on: bool
}

impl<'a> Expression<'a> {

    /// Create and compile a new Expression from a String or &str slice.
    /// If compilation fails, return an Expression with a single Error token 
    /// for which had_compile_error() will return true.
    pub fn new<S>(expr_source: S) -> Expression<'a> where S: Into<String> {
        let expr_string: String = expr_source.into();
        let shy : ShuntingYard = expr_string.clone().into(); 
        match shy.compile() {
            Ok(expr) => expr,
            _ => Expression {
                marker: PhantomData,
                expression_source: expr_string.clone(),
                postfix_order: vec![ShyToken::Error],
                trace_on: false
            }
        }
    }

    pub fn had_compile_error(&self) -> bool {
        self.postfix_order.len() == 0 || self.postfix_order.iter().any(|token| token.is_error() )
    }

    /// Execute an already compiled expression against the given ExecutionContext.  
    pub fn exec(&self, context: &mut ExecutionContext<'a>) -> std::result::Result<ShyValue,String> {
        let mut output_stack : Vec<ShyValue> = vec![];
        if self.trace_on {
            println!("Tracing: {}", self.expression_source);
            Self::dump_postfix(&self.postfix_order);
        }
        for token in self.postfix_order.iter().cloned() {
            if self.trace_on {
                Self::dump_stack(&output_stack);
                println!("  Token: {:?}", token);
            }
            match token {
                ShyToken::Value(value) => output_stack.push(value),
                ShyToken::Operator(ShyOperator::QuitIfFalse) => {
                    let test_result = Self::operate(&mut output_stack, ShyOperator::QuitIfFalse, context);
                    if test_result.is_falsey() {
                        break;
                    }
                },
                ShyToken::Operator(op) => { 
                    Self::operate(&mut output_stack, op, context);
                    ()
                },
                _ => output_stack.push(ShyValue::error("Invalid token in expression".to_string()))
            }
        }
        if self.trace_on {
            Self::dump_stack(&output_stack);
        }
        // The final result of the expression is on top of the stack; pop it off and return it. 
        match output_stack.pop() {
            Some(value) => Ok(value),
            None => Err("Expression stack is empty".to_string())
        }
    }

    fn dump_stack(output_stack: &Vec<ShyValue>) {
        println!("Output Stack:");
        let mut i = 0;
        for token in output_stack.iter().cloned() {
            i = i + 1;
            println!("  {}. {:?}", i, token);
        }
    }

    fn dump_postfix(postfix_order: &Vec<ShyToken>) {
        println!("Postfix order:");
        let mut i = 0;
        for token in postfix_order.iter().cloned() {
            i = i + 1;
            println!("  {}. {:?}", i, token);
        }
    }

    /// Execute the expression with trace turned on, to print diagnostics to the console.
    /// This may have side effects upon the context!
    pub fn trace(&mut self, context: &mut ExecutionContext<'a>) -> std::result::Result<ShyValue, String> {
        self.trace_on = true;
        let exec_result = self.exec(context);
        match &exec_result {
            Ok(_) => { println!("Success"); },
            Err(msg) => { println!("Failure: {}", msg); }
        }
        println!("After execution, {:?}", context);
        self.trace_on = false;
        exec_result
    }

    /// Check if the stack has enough items to satisfy the needs of the operator
    fn is_stack_size_sufficient(output_stack: &mut Vec<ShyValue>, op: ShyOperator) -> bool {
        op.arguments() <= output_stack.len() 
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
            let stack_empty = ShyValue::error(format!("Too few values on stack for operation {:?}. Size = {}", op, output_stack.len()));
            output_stack.clear();
            output_stack.push(stack_empty.clone());
            return stack_empty;
        }
        // If a unary operator, arg1 is the sole argument. 
        // If a binary operator, arg1 is the left operand.
        let mut arg1: ShyValue = 0.into();

        // If a unary operator, arg2 is unused.
        // If a binary operator, arg2 is the right operand.
        let mut arg2: ShyValue = 0.into();
        let mut _arg3: ShyValue = 0.into();

        match op.arguments() {
            1 => {
                arg1 = output_stack.pop().unwrap();
            },
            2 => {
                arg2 = output_stack.pop().unwrap();
                arg1 = output_stack.pop().unwrap();
            },
            3 => {
                _arg3 = output_stack.pop().unwrap();
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
            ShyOperator::Exponentiation => ShyValue::power(&arg1, &arg2),
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
            ShyOperator::QuitIfFalse => {
                if arg1.is_falsey() {
                    output_stack.push(false.into());
                    false.into()
                }
                else {
                    output_stack.push(true.into());
                    true.into()
                }
            },
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
    use std::time::Instant;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    use super::shy_object::ShyObject;

    use crate::cache::{ApproximateLRUCache, Cache};



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

    /// Compile formula testing associativity: "a = b + c * d"
    #[test]
    fn compile_associativity() {
        compile_test_case(
            "a = b + c * d", 
            vec![
            ShyToken::Value(ShyValue::Variable("a".into())),
            ShyToken::Value(ShyValue::Variable("b".into())),
            ShyOperator::Load.into(),
            ShyToken::Value(ShyValue::Variable("c".into())),
            ShyOperator::Load.into(),
            ShyToken::Value(ShyValue::Variable("d".into())),
            ShyOperator::Load.into(),
            ShyOperator::Multiply.into(),
            ShyOperator::Add.into(),
            ShyOperator::Assign.into(),  
        ]);
    }    

    /// Compile formula testing post-increment of a path: "wedding_gifts.count ++"
    #[test]
    fn compile_postincrement_path() {
        // TODO: This test passes because it assumes the wrong series of tokens. 
        //       Need 
        compile_test_case(
            "wedding_gifts.count ++", 
            vec![
            ShyToken::Value(ShyValue::PropertyChain(vec!["wedding_gifts".to_string(), "count".to_string()].into())),
            ShyOperator::Load.into(),
            ShyOperator::PostIncrement.into()
        ]);
    }   

    /// Execute a simple formula: "x = 1"
    #[test]
    fn exec_simple_assignment() {
        let mut ctx = ExecutionContext::default();
        execute_test_case("x = 1", &mut ctx, &1.into(), false); 
        asserting("result written to context").that(&ctx.load(&"x".to_string()).unwrap()).is_equal_to(&1.into());
    }

    /// Execute formula testing associativity: "a = b + c * d"
    #[test]
    fn exec_associativity() {
        let mut ctx = ExecutionContext::default();
        ctx.store(&"b".into(), 2);
        ctx.store(&"c".into(), 3);
        ctx.store(&"d".into(), 4);
        let expected: ShyValue = 14.into();
        execute_test_case("a = b + c * d", &mut ctx, &expected, false); 
        asserting("result written to context").that(&ctx.load(&"a").unwrap()).is_equal_to(&expected);
    }

    #[test]
    fn exec_fancy() {
        let mut ctx = ExecutionContext::default();
        ctx.store(&"b".into(), 2);
        ctx.store(&"c".into(), 3);
        ctx.store(&"d".into(), 25);
        let expected: ShyValue = 5.0.into();
        execute_test_case("a = ((b^3 + c) * √d - 10)/9", &mut ctx, &expected, true); 
        asserting("result written to context").that(&ctx.load(&"a").unwrap()).is_equal_to(&expected);
    }

    #[test]
    fn exec_regex() {
        let mut ctx = ExecutionContext::default();
        ctx.store(&"a".into(), "A9123");
        let expected: ShyValue = true.into();
        execute_test_case("a ~ /9[0-9]+3/", &mut ctx, &expected, true); 
    }

    /// Verify that we can execute multiple assignments separated by semicolons and all are performed.
    #[test]
    fn exec_semicolon() {
        let mut ctx = ExecutionContext::default();
        ctx.store(&"a".into(), 10);
        let expected: ShyValue = 80.into();
        execute_test_case("x = 2 * a; y = a^2; z = y - x", &mut ctx, &expected, true); 
        asserting("first result written to context").that(&ctx.load(&"x").unwrap()).is_equal_to(&20.into());
        asserting("second result written to context").that(&ctx.load(&"y").unwrap()).is_equal_to(&100.into());
        asserting("third result written to context").that(&ctx.load(&"z").unwrap()).is_equal_to(&80.into());
    }

    /// Verify that a value stored in a ShyObject can be retrieved by its property path and used in an expression.
    #[test]
    fn exec_path_load() {
        let mut ctx = ExecutionContext::default();
        let car = ShyObject::empty();
        car.as_deref_mut().set("speed", 75.0.into());
        ctx.store(&"vehicle".into(), ShyValue::Object(car));
        let expected: ShyValue = true.into();
        execute_test_case("speeding = vehicle.speed > 65.0", &mut ctx, &expected, true); 
        asserting("speeding value written to context").that(&ctx.load(&"speeding").unwrap()).is_equal_to(&expected);
    }

    /// Verify that an existent path can be incremented from an actual value of zero to a value of one.
    #[test]
    #[ignore]
    fn exec_increment_existing_path() {
        let mut ctx = ExecutionContext::default();
        let gifts = ShyObject::empty();
        gifts.as_deref_mut().set("count", 0.into());
        ctx.store(&"wedding_gifts".into(), ShyValue::Object(gifts));

        let expected: ShyValue = 1.into();
        execute_test_case("wedding_gifts.count ++", &mut ctx, &expected, true); 
        asserting("incremented count of known path works")
            .that(&ctx.load_str_chain("wedding_gifts.count").unwrap())
            .is_equal_to(&expected);
    }

    /// Verify that a nonexistent path can be autovivified and incremented from an inferred value of zero to a value of one.
    #[test]
    #[ignore]
    fn exec_increment_missing_path() {
        let mut ctx = ExecutionContext::default();
        let expected: ShyValue = 1.into();
        execute_test_case("wedding_gifts.count ++", &mut ctx, &expected, true); 
        asserting("incremented count of unknown path works")
            .that(&ctx.load_str_chain("wedding_gifts.count").unwrap())
            .is_equal_to(&expected);
    }

    /// Verify that an existent path can be loaded then updated.
    #[test]
    fn exec_load_and_store_existing_path() {
        let mut ctx = ExecutionContext::default();
        let gifts = ShyObject::empty();
        gifts.as_deref_mut().set("count", 4.into());
        ctx.store(&"wedding_gifts".into(), ShyValue::Object(gifts));

        let expected: ShyValue = 5.into();
        execute_test_case("wedding_gifts.count = wedding_gifts.count + 1", &mut ctx, &expected, true); 
        asserting("load and store of known path works")
            .that(&ctx.load_str_chain("wedding_gifts.count").unwrap())
            .is_equal_to(&expected);
    }

    #[test]
    fn exec_existing_path_with_plus_assign() {
        let mut ctx = ExecutionContext::default();
        let gifts = ShyObject::empty();
        gifts.as_deref_mut().set("count", 4.into());
        ctx.store(&"wedding_gifts".into(), ShyValue::Object(gifts));

        let expected: ShyValue = 5.into();
        execute_test_case("wedding_gifts.count += 1", &mut ctx, &expected, true); 
        asserting("existing path with plus assign works")
            .that(&ctx.load_str_chain("wedding_gifts.count").unwrap())
            .is_equal_to(&expected);
    }

    /// Verify that an existent path can be used in an or-equals.
    #[test]
    fn exec_path_or_equals() {
        let mut ctx = ExecutionContext::default();
        let circumstances = ShyObject::empty();
        circumstances.as_deref_mut().set("graduated_high_school", false.into());
        circumstances.as_deref_mut().set("graduated_college", true.into());
        circumstances.as_deref_mut().set("criminal_record", false.into());
        circumstances.as_deref_mut().set("credit_score", 650.into());
        ctx.store(&"circumstances".into(), ShyValue::Object(circumstances));

        let expected: ShyValue = true.into();
        let expr = "
        circumstances.interview = circumstances.graduated_high_school || circumstances.graduated_college;
        circumstances.interview ||= !circumstances.criminal_record;
        circumstances.interview ||= circumstances.credit_score >= 600
        ";
        execute_test_case(expr, &mut ctx, &expected, true); 
        asserting("load and store of path with or-equals operator")
            .that(&ctx.load_str_chain("circumstances.interview").unwrap())
            .is_equal_to(&expected);
    }

    #[test]
    /// Verify that the "if" function works.
    fn exec_if() {
        let mut ctx = ExecutionContext::default();

        let expected: ShyValue = 42.into();
        let expr = "smart = true; answer = if(smart, 42, 0)";
        execute_test_case(expr, &mut ctx, &expected, true); 
        asserting("if function")
            .that(&ctx.load(&"answer".to_string()).unwrap())
            .is_equal_to(&expected);
    }

    #[test]
    /// Verify that the `majority` function works.
    fn exec_majority() {
        let mut ctx = ExecutionContext::default();

        let expected: ShyValue = true.into();
        let expr = "tall = false; dark = true; handsome = true; answer = majority(tall, dark, handsome)";
        execute_test_case(expr, &mut ctx, &expected, true); 
        asserting("majority function")
            .that(&ctx.load(&"answer".to_string()).unwrap())
            .is_equal_to(&expected);
    }

    #[test]
    /// Verify that the `?` operator works.
    fn exec_quit_if_false() {
        let mut ctx = ExecutionContext::default();

        let expected_result: ShyValue = false.into();
        let expected_y: ShyValue = 1.into();
        let expr = "x = 10; x > 5 ?; y = 1; x > 20? ; y = 2";
        execute_test_case(expr, &mut ctx, &expected_result, true); 
        asserting("? operator")
            .that(&ctx.load(&"y".to_string()).unwrap())
            .is_equal_to(&expected_y);
    }    

    #[test]
    /// Verify that if expressions are cached, they still execute properly and it takes less time to execute them.
    /// On a Windows Tablet, for a typical formula: 
    /// 
    ///    - 4.4 evals per ms without cache 
    ///    - 42.6 evals per ms with a cache
    /// 
    /// This is running unoptimized.
    fn expression_cache_performance() {
        let mut cache : ApproximateLRUCache<String, Expression> = ApproximateLRUCache::new(10000);
        let mut ctx = ExecutionContext::default();
        ctx.store(&"x".to_string(), 5);
        ctx.store(&"y".to_string(), 10);
        let mut expressions : Vec<ExpressionCacheTest> = Vec::new();
        for i in 0..100 {
            for j in 0..100 {
                let expression_text = format!("y * (4 * {} - 2 * {})^2 / x", i, j);
                let test = ExpressionCacheTest::new(&expression_text, &mut ctx, &mut cache);
                expressions.push(test);
            }
        }

        let timer_with_cache = Instant::now();
        for expr in expressions.iter_mut() {
            expr.execute_with_cache(&mut ctx, &mut cache);
        }
        let elapsed_millis_with_cache  : i64 = timer_with_cache.elapsed().as_millis() as i64;

        let timer_without_cache = Instant::now();
        for expr in expressions.iter_mut() {
            expr.execute_without_cache(&mut ctx);
        }
        let elapsed_millis_without_cache : i64 = timer_without_cache.elapsed().as_millis() as i64;
        let message = format!("With cache: {}ms    Without cache: {}ms", elapsed_millis_with_cache, elapsed_millis_without_cache);
        asserting(&message).that(&elapsed_millis_without_cache).is_greater_than(&(5*elapsed_millis_with_cache));
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

    fn execute_test_case(expression: &str, ctx: &mut ExecutionContext, expected: &ShyValue, turn_on_trace: bool) {
        let shy: ShuntingYard = expression.into();
        match shy.compile() {
            Ok(mut expr) => {
                let exec_result = 
                    if turn_on_trace { expr.trace(ctx) }
                    else { expr.exec(ctx) };
                match exec_result {
                    Ok(actual) => asserting(&format!("exec of {}", expression.to_string())).that(&actual).is_equal_to(expected),
                    Err(msg) => {
                        println!("{:?}", ctx);
                        assert!(false, format!("Error executing {}: {}", expression, msg))
                    }
                }
            },
            Err(msg) => { assert!(false, format!("Error compiling {}: {}", expression, msg)) }
        }
    }

    struct ExpressionCacheTest<'a> {
        text_expression : String,
        compiled_expression : Expression<'a>,
        expected_result : ShyValue
    }

    impl<'a> ExpressionCacheTest<'a> {
        pub fn new<C>(expression : &str, ctx : &mut ExecutionContext, cache : &mut C) -> Self 
        where C : Cache<String, Expression<'a>>
        {
            let shy : ShuntingYard = expression.into();
            let compiled = shy.compile().unwrap();
            let result = compiled.exec(ctx).unwrap();
            cache.get_or_add(&expression.to_string(), & |_| Some(compiled.clone()) );
            ExpressionCacheTest {
                text_expression : expression.to_string(),
                compiled_expression : compiled,
                expected_result : result
            }
        }

        pub fn execute_without_cache(&mut self, ctx : &mut ExecutionContext<'a>) {
            let shy : ShuntingYard = self.text_expression.clone().into();
            let actual_result = shy.compile().unwrap().exec(ctx).unwrap();
            if actual_result != self.expected_result {
                panic!("Actual result {:?} does not match expected {:?}", actual_result, self.expected_result);
            }
        }

        pub fn execute_with_cache<C>(&mut self, ctx : &mut ExecutionContext<'a>, cache : &mut C) 
        where C : Cache<String, Expression<'a>> {
            match cache.get(&self.text_expression) {
                Some((compiled_expression_from_cache, _)) => {
                    let actual_result = compiled_expression_from_cache.exec(ctx).unwrap();
                    if actual_result != self.expected_result {
                        panic!("Actual result {:?} does not match expected {:?}", actual_result, self.expected_result);
                    }
                },
                None => panic!("{} not in cache", self.text_expression)
            }

        }
    }
}
