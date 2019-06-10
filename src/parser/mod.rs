#![allow(dead_code)]

#[allow(unused_imports)]
use std::fmt::Result;
use std::collections::HashMap;
use std::f64;
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
        // Need to clone infix_order to placate the borrow-checker, otherwise I cannot call the reduce method.
        let infix_order_copy = self.infix_order.clone();
        for stoken in infix_order_copy.iter() {
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

    /// Compile the expression into a postfix ordered series of tokens.
    pub fn compile(mut self) -> std::result::Result<Expression<'a>,String> {
        match self.parse() {
            Ok(_) => {
                // TODO: Optimizations like constant folding, And/Or operator short-cutting, branching.
                Ok(Expression { 
                    expression_source: self.expression_source.clone(),
                    postfix_order: self.postfix_order.clone()
                })
            },
            Err(s) => Err(format!("{}\n{:?}", s, self))
        }
    }


}

//..................................................................


/// ExecutionContext holds variables and functions needed when executing expressions.
///   - Some variables are loaded for use in the formulas.
///   - Some variables are used to store the results of formulas after execution. 
///   - The functions may be called in the expressions.
pub struct ExecutionContext<'a> {
    pub variables: HashMap<String, ShyValue<'a>>,

    functions: HashMap<String, ShyFunction<'a>>
}

type ShyFunction<'a> = Box<(Fn(ShyValue<'a>) -> ShyValue<'a> + 'a)>;

type Ctx<'a> = ExecutionContext<'a>;

impl<'a> ExecutionContext<'a> {

    pub fn shy_func<F>(f: F) -> ShyFunction<'a>
        where F: Fn(ShyValue<'a>) -> ShyValue<'a> + 'a {
            Box::new(f) as ShyFunction
    }

    /// Define a context function that assumes the argument is a float or integer or a vector
    /// that holds a single float or integer and returns a double.
    pub fn shy_double_func<G>(g: G) -> ShyFunction<'a>
        where G: Fn(f64) -> f64 + 'a {
            Ctx::shy_func(move |v| {
                match v {
                    ShyValue::Scalar(ShyScalar::Rational(x)) => g(x).into(),
                    ShyValue::Scalar(ShyScalar::Integer(i)) => g(i as f64).into(),
                    ShyValue::Vector(vect) if vect.len() == 1 => match vect[0] {
                        ShyScalar::Rational(x) => g(x).into(),
                        ShyScalar::Integer(i) => g(i as f64).into(),
                        _ => f64::NAN.into()
                    },
                    _ => f64::NAN.into()
                }
            })
    }

    fn standard_functions() -> HashMap<String, ShyFunction<'a>> {
        let mut map = HashMap::new();
        map.insert("abs".to_string(), Ctx::shy_double_func(|x| x.abs()));
        map.insert("acos".to_string(), Ctx::shy_double_func(|x| x.acos()));
        map.insert("asin".to_string(), Ctx::shy_double_func(|x| x.asin()));
        map.insert("atan".to_string(), Ctx::shy_double_func(|x| x.atan()));
        map.insert("cos".to_string(), Ctx::shy_double_func(|x| x.cos()));
        map.insert("exp".to_string(), Ctx::shy_double_func(|x| x.exp()));
        map.insert("ln".to_string(), Ctx::shy_double_func(|x| x.ln()));
        map.insert("sin".to_string(), Ctx::shy_double_func(|x| x.sin()));
        map.insert("sqrt".to_string(), Ctx::shy_double_func(|x| x.sqrt()));
        map.insert("tan".to_string(), Ctx::shy_double_func(|x| x.tan()));
        map
    }

    fn standard_variables() ->  HashMap<String, ShyValue<'a>> {
        let mut map = HashMap::new();
        map.insert("PI".to_string(), f64::consts::PI.into());
        map.insert("π".to_string(), f64::consts::PI.into());
        map.insert("e".to_string(), f64::consts::E.into());
        map.insert("φ".to_string(), ( (1.0 + 5_f64.sqrt())/2.0).into());
        map.insert("PHI".to_string(), ( (1.0 + 5_f64.sqrt())/2.0).into());
        map
    }

    pub fn new(mut vars: HashMap<String, ShyValue<'a>>, mut funcs: HashMap<String, ShyFunction<'a>>) -> Self {
        vars.extend(ExecutionContext::standard_variables());
        funcs.extend(ExecutionContext::standard_functions());
        ExecutionContext {
            variables: vars,
            functions: funcs
        }
    }

    /// Create a default context that only defines math functions and constants.
    pub fn default() -> Self {
        ExecutionContext {
            variables: ExecutionContext::standard_variables(),
            functions: ExecutionContext::standard_functions()
        }
    }    

    /// Store a new value for the variable in the context.
    pub fn store(&mut self, name: String, val: ShyValue<'a>) {
        self.variables.insert(name.clone(), val);
    }

    /// Retrieve the current value of the variable from the context, or an Error.
    pub fn load(&self, name: String) -> ShyValue<'a> { 
        match self.variables.get(&name) {
            Some(val) => val.clone(),
            None => ShyValue::error(format!("Name {} not found in context", name))
        }
    }

    /// Call a function that is stored in the context.
    pub fn call(&self, function_name: String, args: ShyValue<'a>) -> ShyValue<'a> {
        match self.functions.get(&function_name) {
            Some(func) => func(args),
            None => ShyValue::error(format!("No function named {} in context", function_name))
        }
    }

}

impl<'a> From<&HashMap<String,f64>> for ExecutionContext<'a> {
    /// Create an ExecutionContext from a simple map of string-float pairs.
    fn from(initial_values: &HashMap<String,f64>) -> Self {
        let mut context = ExecutionContext::default();
        for (key, value) in &*initial_values {
            let wrapped_value: ShyValue<'a> = (*value).into();
            context.variables.insert(key.clone(), wrapped_value);
        }
        context
    }
}



//..................................................................

/// Compiled Expression that can be executed.
#[derive(Debug)]
pub struct Expression<'a> {
    pub expression_source: String,

    pub postfix_order: Vec<ShyToken<'a>>
}

impl<'a> Expression<'a> {
    pub fn exec(&self, context: &mut ExecutionContext<'a>) -> std::result::Result<ShyValue<'a>,String> {
        let mut output_stack : Vec<ShyValue<'a>> = vec![];
        for token in self.postfix_order.iter().cloned() {
            match token {
                ShyToken::Value(value) => output_stack.push(value),
                ShyToken::Operator(op) => Self::operate(&mut output_stack, op, context),
                _ => output_stack.push(ShyValue::error("Invalid token in expression".to_string()))
            }
        }
        match output_stack.pop() {
            Some(value) => Ok(value),
            None => Err("Expression stack is empty".to_string())
        }
    }

    /// Apply an operator, removing tokens from the stack, computing a result, and pushing the result back on the stack.
    fn operate(output_stack: &mut Vec<ShyValue<'a>>, op: ShyOperator, context: &mut ExecutionContext<'a>) {
        match op {
            ShyOperator::Load => (),
            ShyOperator::Store => (),
            ShyOperator::Semicolon => (),
            ShyOperator::FunctionCall => (),
            ShyOperator::OpenParenthesis => (),
            ShyOperator::CloseParenthesis => (),
            ShyOperator::Comma => (),
            ShyOperator::OpenBracket => (),
            ShyOperator::CloseBracket => (),
            ShyOperator::Member => (),
            ShyOperator::Power => (),
            ShyOperator::Exponentiation => (),
            ShyOperator::PrefixPlusSign => (),
            ShyOperator::PrefixMinusSign => (),
            ShyOperator::PostIncrement => (),
            ShyOperator::PostDecrement => (),
            ShyOperator::SquareRoot => (),
            ShyOperator::LogicalNot => (),
            ShyOperator::Factorial => (),
            ShyOperator::Match => (),
            ShyOperator::NotMatch => (),
            ShyOperator::Multiply => (),
            ShyOperator::Divide => (),
            ShyOperator::Mod => (),
            ShyOperator::Add => (),
            ShyOperator::Subtract => (),
            ShyOperator::LessThan => (),
            ShyOperator::LessThanOrEqualTo => (),
            ShyOperator::GreaterThan => (),
            ShyOperator::GreaterThanOrEqualTo => (),
            ShyOperator::Equals => (),
            ShyOperator::NotEquals => (),
            ShyOperator::And => (), 
            ShyOperator::Or => (), 
            ShyOperator::Ternary => (),
            ShyOperator::Assign => (),
            ShyOperator::PlusAssign => (),
            ShyOperator::MinusAssign => (),
            ShyOperator::MultiplyAssign => (),
            ShyOperator::DivideAssign => (),
            ShyOperator::ModAssign => (),
            ShyOperator::AndAssign => (),
            ShyOperator::OrAssign => (),
            _ => panic!("Invalid operator {:?}", op),
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
        let shy: ShuntingYard = "(2 + 3) * (4 - 5))".into();
        match shy.compile() {
            Err(msg) => assert_that(&msg).contains("Unbalanced"),
            _ => assert!(false, "Did not return error")
        }
    }

    #[test]
    /// Verify that an error with too many opening parentheses is generated.
    fn unbalanced_opening_parentheses() {
        let shy: ShuntingYard = "((2 + 3) * (4 - 5)".into();
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

    #[test]
    /// Verify that logical not (a prefix operator) is handled properly when parenthesized.
    fn logical_not_parenthesized() {
        compile_test_case(
            "!(a || b)", 
            vec![
            ShyToken::Value(ShyValue::Variable("a".to_string())),
            ShyToken::Value(ShyValue::Variable("b".to_string())),
            ShyToken::Operator(ShyOperator::Or),
            ShyToken::Operator(ShyOperator::LogicalNot),
        ]);
    }

    #[test]
    fn string_match() {
        compile_test_case(
            r#"name ~ /^Paul/ && color == "blue""#, 
            vec![
            ShyToken::Value(ShyValue::Variable("name".to_string())),
            // TODO: Implement ShyScalar::Regex
            ShyToken::Value(ShyValue::Scalar(ShyScalar::String("^Paul".to_string()))),
            ShyToken::Operator(ShyOperator::Match),
            ShyToken::Value(ShyValue::Variable("color".to_string())),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::String("blue".to_string()))),
            ShyToken::Operator(ShyOperator::Equals),
            ShyToken::Operator(ShyOperator::And),
        ]);
    }

    #[test]
    /// Verify that logical not (a prefix operator) is handled properly when parenthesized.
    fn function_call() {
        compile_test_case(
            "0.5 + sin(π/6)", 
            vec![
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Rational(0.5))),
            ShyToken::Value(ShyValue::FunctionName("sin".to_string())),
            ShyToken::Value(ShyValue::Variable("π".to_string())),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(6))),
            ShyToken::Operator(ShyOperator::Divide),
            ShyToken::Operator(ShyOperator::FunctionCall),
            ShyToken::Operator(ShyOperator::Add),
        ]);
    }

    #[test]
    /// Verify that raising a value to a power has the correct precedence and associativity.
    fn power() {
        compile_test_case(
            "3*2¹⁰/5", 
            vec![
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(3))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(2))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(10))),
            ShyToken::Operator(ShyOperator::Exponentiation),
            ShyToken::Operator(ShyOperator::Multiply),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(5))),
            ShyToken::Operator(ShyOperator::Divide),
        ]);
    }

    #[test]
    /// Verify that raising a value to a power has the correct precedence and associativity.
    fn square_root() {
        compile_test_case(
            "3*√2/5", 
            vec![
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(3))),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(2))),
            ShyToken::Operator(ShyOperator::SquareRoot),
            ShyToken::Operator(ShyOperator::Multiply),
            ShyToken::Value(ShyValue::Scalar(ShyScalar::Integer(5))),
            ShyToken::Operator(ShyOperator::Divide),
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


}
