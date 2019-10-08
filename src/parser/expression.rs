use std::marker::PhantomData;
use std::collections::HashSet;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::borrow::BorrowMut;

use super::shy_token::{ShyToken, ShyValue};
use super::ShuntingYard;
use super::execution_context::ExecutionContext;
use super::shy_operator::ShyOperator;
use super::shy_scalar::ShyScalar;
use crate::graph::Graph;

//..................................................................

pub enum ReferenceType {
    Definition,
    Dependency,
    ExternalDependency,
    Unknown
}

#[derive(Debug, Clone)]
/// Variables (or property chains) referenced by an expression, either as definitions or dependencies on other expressions (or the context).
pub struct References {
    /// Variables that are defined in an expression or set of expressions
    pub definitions : Vec<String>,

    /// Variables that are either external dependencies not defined in any expression, or are internal dependencies 
    /// that are used in one expression but defined in another. Until all expressions are processed,
    /// it is impossible to distinguish the two cases. 
    pub dependencies : Vec<String>,

    /// Variables that are not defined in any expressions and are referenced by at least one expression. 
    pub external_dependencies : Vec<String>
}

impl References {

    pub fn new() -> Self {
        References {
            definitions : Vec::new(),
            dependencies : Vec::new(),
            external_dependencies : Vec::new()
        }
    }

    pub fn get_reference_type(&self, variable : &String) -> ReferenceType {
        if self.definitions.contains(variable) { ReferenceType::Definition }
        else if self.dependencies.contains(variable) { ReferenceType::Dependency }
        else if self.external_dependencies.contains(variable) { ReferenceType::ExternalDependency }
        else { ReferenceType::Unknown }
    }

    /// Determine the cumulative effect of following a series of expressions that have collectively defined
    /// the variables in self by an expression that defines or depends on the expressions in additional_references.
    /// 
    /// 1. If additional_references has dependencies that are neither defined in self nor external_dependencies in self,
    /// return None, because it means the expression will attempt to retrieve an uninitialized variable. 
    /// 
    /// 2. If additional_references has definitions that are marked as dependencies or external_dependencies in self,
    /// return None, because that means that expressions were processed out of order. Expressions with a definition
    /// are supposed to be processed before all expressions that depend on it. A circular reference can cause this problem. 
    /// 
    /// 3. Otherwise, append the values in additional_references to the appropriate slots in a clone of self,
    /// taking care to not duplicate any names. 
    /// 
    /// Note: It is permissible for two expressions to define the same variable. It is likely that they have non-overlapping 
    /// applicability tests that would cause the variable to only be defined once. If the applicability tests overlap,
    /// then the variable may be defined twice, meaning that the order in which they are executed may matter.
    pub fn follow_by(&self, additional_references : &Self) -> Option<References> {
        let mut result = self.clone();
        for variable in &additional_references.dependencies {
            match self.get_reference_type(&variable) {
                ReferenceType::Definition => (),
                ReferenceType::Dependency => return None,
                ReferenceType::ExternalDependency => (),
                ReferenceType::Unknown => return None
            }
        }
        for variable in &additional_references.definitions {
            match self.get_reference_type(&variable) {
                ReferenceType::Definition => (), // Already defined, but that is okay. 
                ReferenceType::Dependency => return None, // Incorrect ordering of expressions!
                ReferenceType::ExternalDependency => return None, // Inconsistent!
                ReferenceType::Unknown => { result.definitions.push(variable.clone()) }
            }
        }
        for variable in &additional_references.external_dependencies {
            match self.get_reference_type(&variable) {
                ReferenceType::Definition => return None, // Inconsistent! 
                ReferenceType::Dependency => return None, // Incorrect ordering of expressions!
                ReferenceType::ExternalDependency => (), // Already known, but that is okay
                ReferenceType::Unknown => { result.external_dependencies.push(variable.clone()) }
            }
        }
        Some(result)
    }

    /// True if there are no internal or external dependencies.
    pub fn has_no_dependencies(&self) -> bool {
        self.dependencies.len() == 0 && self.external_dependencies.len() == 0
    }

    /// True if there are no internal dependencies.
    pub fn has_no_internal_dependencies(&self) -> bool {
        self.dependencies.len() == 0
    }

    /// True if there is an internal dependency on the given variable.
    pub fn has_internal_dependency_on(&self, variable: &String) -> bool {
        self.dependencies.contains(variable)
    }

    /// True if there is an external dependency on the given variable.
    pub fn has_external_dependency_on(&self, variable: &String) -> bool {
        self.external_dependencies.contains(variable)
    }

    /// Given a complete set of Expressions, infer which variable references are external dependencies.
    /// 
    ///   - If a variable has at least one Expression that defines it, assume it is not an external dependency. 
    ///   - If a variable only occurs as a dependency and never as a definition, assume that it is an external dependency. 
    /// 
    /// External dependencies are assumed to be provided by the ExecutionContext. 
    pub fn infer_external_dependencies<'a, X>(expressions : &mut Vec<X>) -> HashSet<String> 
    where X : Expressive<'a> + Sized
    {
        let mut all_definitions = HashSet::new();
        let mut all_dependencies = HashSet::new();
        for used_rro in expressions.iter().map(|expr| expr.express().variables_used()) {
            let used_opt = used_rro.borrow();
            {
                let used = used_opt.as_ref().unwrap();
                all_definitions.extend(used.definitions.iter().cloned());
                all_dependencies.extend(used.dependencies.iter().cloned());
            }
            // Assume that we start out not having any known external dependencies. 
        }
        // Assume that all dependencies that never receive definitions are external_dependencies. 
        // They must be provided by the context. 
        let diffs: HashSet<String> = all_dependencies.difference(&all_definitions).map(|diff| diff.clone()).collect();
        diffs
    }

    /// Now that we know the external dependencies, move all variables from self.dependencies to self.external_dependencies
    /// that are found in externals.
    pub fn apply_external_dependencies(&mut self, externals : &HashSet<String>) {
        for dependency in externals {
            if let Some(position) = self.external_dependencies.iter().position(|s| *s == *dependency) {
                self.external_dependencies.push(self.dependencies.swap_remove(position));
            }
        }
    }
    
}

//..................................................................

#[derive(Debug, Clone)]
/// Compiled Expression that can be executed.
pub struct Expression<'a> {
    marker: PhantomData<&'a i64>,

    /// Infix Expression as a string before it was compiled
    pub expression_source: String,

    /// The constants, variable references and operators parsed from the expression_source and rearranged into postfix order.
    /// This list of tokens was generated by the shunting yard algorithm.
    pub postfix_order: Vec<ShyToken>,

    /// References to variables and property chains that this expression either defines or depends upon. 
    /// Created lazily when variables_used is called.
    references: Rc<RefCell<Option<References>>>,

    /// If true, a trace of the execution of the expression is printed as a diagnostic.
    pub trace_on: bool
}

pub trait Expressive<'a> {
    fn express(&self) -> &Expression<'a>;
    fn express_mut(&mut self) -> &mut Expression<'a>;
}

impl<'a> Expressive<'a> for Expression<'a> {
    fn express(&self) -> &Self { self }
    fn express_mut(&mut self) -> &mut Self { self }
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
                references: Rc::new(RefCell::new(None)),
                trace_on: false
            }
        }
    }

    /// Create an Expression given its already compiled form. 
    pub fn new_from_parser<S>(expr_source: S, postfix : &Vec<ShyToken>) -> Expression<'a> where S: Into<String> {
        Expression { 
            marker: PhantomData,
            expression_source: expr_source.into(),
            postfix_order: postfix.clone(),
            references: Rc::new(RefCell::new(None)),
            trace_on: false
        }
    }

    /// Return true if an error occurred while compiling the expression. 
    /// The error is likely due to a syntax error in the expression, not
    /// a failure of the parser.
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
                    // Shortcut the expression evaluation at the question mark, cease execution and return false. 
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


    /// Gathers the names of all variables and property chains that are referenced by the expression,
    /// either as definitions or dependencies. 
    /// 
    /// Dependencies may be satisfied by another Rule or by the caller through the ExecutionContext itself. 
    /// 
    /// This lazily sets the Expression's references property. 
    /// 
    /// This method has no knowledge of whether a dependency is external,
    /// because to know that, you need to have a complete set of Expressions to examine, not just this one. 
    /// To determine and set the external dependencies, call apply_external_dependencies.
    pub fn variables_used(&self) -> Rc<RefCell<Option<References>>> {
        if self.references.as_ref().borrow().is_some() {
            return self.references.clone();
        }
        self.lazy_init_variables_used();
        self.references.clone()
    }

    /// A clone of the References to variables defined or used by the Expression.
    pub fn get_references(&self) -> References {
        let variables_used_rro = self.variables_used(); //.clone();
        let variables_used_ro = variables_used_rro.as_ref().borrow();
        match *variables_used_ro {
            Some(ref refs) => {
                refs.clone()
            },
            None => panic!("No References in Expression")
        }
    }

    /// Every dependency of the Expression found in external_dependencies will be restated 
    /// as an external_dependency inside references.
    pub fn apply_external_dependencies(&self, external_dependencies : &HashSet<String>) {
        let variables_used_rro = self.variables_used().clone();
        let mut variables_used_ro = variables_used_rro.as_ref().borrow_mut();
        match *variables_used_ro {
            Some(ref mut refs) => {
                refs.apply_external_dependencies(external_dependencies);
            },
            None => panic!("No References in Expression")
        }
    }

    /// If references holds a None, it is uninitialized, therefore initialize it lazily and return true, 
    /// else return false to say no initialization was necessary. 
    fn lazy_init_variables_used(&self) -> bool {
        if (*self.references.as_ref()).borrow().is_some() {
            return false;
        }
        let mut definitions = Vec::new();
        let mut dependencies = Vec::new();

        let already_recorded = |name : &String, vec1: &Vec<String>, vec2 : &Vec<String>| -> bool { vec1.contains(name) || vec2.contains(name) };

        // If a ShyToken::Value(ShyValue::Variable(...)) or a ShyToken::Value(ShyValue::PropertyChain(...)) 
        // is followed immediately by a ShyValue:: ShyOperator::Load, it could be a definition. 
        // Otherwise it could be a dependency. Only the first occurrence of that name in the 
        // expression defines whether it is a definition or a dependency. 
        let peekaboo = &mut self.postfix_order.iter().peekable();
        while let Some(item) = peekaboo.next() {
            let followed_by_load = if let Some(ShyToken::Operator(op)) = peekaboo.peek() { *op == ShyOperator::Load } else { false };
            match item {
                ShyToken::Value(ShyValue::PropertyChain(chain)) => {
                    let chain_string = chain.join(".");
                    let skip = already_recorded(&chain_string, &definitions, &dependencies);
                    if !skip {
                        if followed_by_load { dependencies.push(chain_string); }
                        else { definitions.push(chain_string); }
                    }
                },
                ShyToken::Value(ShyValue::Variable(variable)) => {
                    let skip = already_recorded(&variable, &definitions, &dependencies);
                    if !skip {
                        if followed_by_load { dependencies.push(variable.clone()); }
                        else { definitions.push(variable.clone()); }
                    }
                },
                _ => ()
            }
        }
        // Lazy initialization of references so next call to variables_used can use the memoized value. 
        println!("In lazy_init_variables_used with {} defs and {} deps", definitions.len(), dependencies.len());
        self.references.as_ref().replace(Some(References { definitions, dependencies, external_dependencies : Vec::new() }));
        true
    }

    /// Build the dependency graph for a list of Expressions. 
    ///   - Variables used in an expression that are not defined are dependencies, to be represented as nodes
    ///     that point via an incoming edge to the node corresponding to the Expression. 
    ///   - Variables that an Expression defines are definitions, to be represented as nodes
    ///     to which the Expression points via an outgoing edge. 
    fn dependency_graph<'b, X>(expressions : &mut Vec<X>) -> Graph
    where X : Expressive<'b>    
     {
        // Infer the external dependencies upon variables (which must be provided by the execution context)
        // by studying all the expressions. 
        let external_deps = References::infer_external_dependencies(expressions);

        // Apply the external dependencies to the expressions, so they can discriminate between internal and external dependencies.
        // As we go, also create a concordance, to map each dependency or definition (the variable names) to a number, 
        // starting at expressions.size(). 
        // This unique number will become the node_id for that variable name in the graph. 
        let mut variable_to_id : HashMap<String, usize> = HashMap::new();
        let mut highest_node_id : usize = expressions.len() - 1;
        for expr in expressions.iter().map(|x| x.express()) {
            expr.apply_external_dependencies(&external_deps);
            let refs = expr.get_references();
            for def_name in refs.definitions.iter() {
                variable_to_id.entry(def_name.clone()).or_insert_with(|| { highest_node_id += 1; highest_node_id });
            }
            for dep_name in refs.dependencies.iter() {
                variable_to_id.entry(dep_name.clone()).or_insert_with(|| { highest_node_id += 1; highest_node_id });
            }
        }

        // Build a Graph, where: 
        //     - every variable that is defined or depended upon is a node
        //     - every expression is a node, and its zero-based position in the expressions Vec is its node_id
        //     - every variable definition is an outgoing edge from expression node to variable node
        //     - every variable reference that is to an internal (not external) dependency is 
        //       an incoming edge from variable to expression node
        let mut graph = Graph::new(highest_node_id + 1);

        for (expr_node_id, expr) in expressions.iter().map(|x| x.express()).enumerate() {
            let refs = expr.get_references();
            // Add the outgoing edges for definitions.
            for def_name in refs.definitions.iter() {
                let def_node_id = variable_to_id[def_name];
                graph.add_edge(expr_node_id, def_node_id);
            }
            // Add the incoming edges for dependencies. 
            for dep_name in refs.dependencies.iter() {
                let dep_node_id = variable_to_id[dep_name];
                graph.add_edge(dep_node_id, expr_node_id);
            }
        }
        graph
    }

    /// Given a list of expressions in an arbitrary order, **sort them topologically** 
    /// so that no Expression that relies upon a dependency
    /// is executed before that dependency is defined by another Expression, 
    /// unless that dependency is ruled an external dependency. 
    /// 
    /// Returns a Tuple with two lists. 
    ///   - The first Vec holds all the Expressions that could be ordered correctly in topologically sorted order. 
    ///   - The second Vec holds all the Expressions that had dependencies that could not be resolved in a satisfactory way. 
    ///   - If the second Vec is empty, no error occurred and a complete topological sort was performed. 
    ///   - If the second Vec is not empty, it is likely that there is a circularity in the variable references. 
    ///     This implies that the dependency graph is not a DAG (*directed acyclic graph*). 
    /// 
    /// **Complication**: Multiple expressions can define the same variable; in practice, for some expressions, 
    /// their applicability prologue will cause the evaluation to short-circuit and the definition not be performed.
    /// Thus an expression that depends upon a variable must wait until all expressions that might define that variable
    /// have been executed. 
    pub fn untangle<'b, X>(mut exprs : Vec<X>) -> (Vec<Rc<X>>, Vec<Rc<X>>) 
    where X : Expressive<'b> {
        let expression_count = exprs.len();
        let mut untangled = Vec::with_capacity(expression_count);
        let mut tangled = Vec::new();      
        let graph : Graph;
        {
            let expressions = exprs.borrow_mut();
            // Interpret the definition and use (dependency) of variables within expressions as relationships
            // in a graph. Every expression becomes a node, as well as every variable. 
            graph = Self::dependency_graph(expressions);
        }
        // Sort the graph topologically. 
        let (sorted, unsorted) = graph.sort();

        // Reorder the expressions according to the topological sort. 
        // Recognize that some of the node_ids in the solution correspond to variable names, which should be skipped. 
        // If the node_id >= expressions.len() then it is a variable node.  
        let expr_rc : Vec<Rc<X>> = exprs.into_iter().map(|x| Rc::new(x)).collect();
        for expression_node_id in sorted.iter().filter(|node_id| **node_id < expression_count) {
            // Transcribe Expressions from expr_rc to untangled via sorted.
            untangled.push(expr_rc[*expression_node_id].clone());
        }
        for expression_node_id in unsorted.iter().filter(|node_id| **node_id < expression_count) {
            // Transcribe Expressions from expr_rc to tangled via unsorted.
            tangled.push(expr_rc[*expression_node_id].clone());
        }
        (untangled, tangled)
    }
    
}

#[cfg(test)]
/// Tests of Expressions.
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    use super::ShuntingYard;

    #[test]
    /// Check that variables_used finds the correct sets of variable dependencies and definitions.
    fn variables_used() {
        // x and y are defined. 
        // z is a dependency. 
        // Second use of x is a read, but since it was already defined, it is not a dependency. 
        let test_expression_text = "x = 3; y = x + well.depth;";
        let shy : ShuntingYard = test_expression_text.clone().into();
        let test_expression = shy.compile().unwrap();

        let variables_used_rro = test_expression.variables_used().clone();
        let variables_used_ro = variables_used_rro.as_ref().borrow();
        let used = variables_used_ro.as_ref().unwrap();
        let expected_definitions : Vec<String> = vec!["x".into(), "y".into()];
        let expected_dependencies : Vec<String> = vec!["well.depth".into()];
        asserting("Definitions match").that(&do_vecs_match(&expected_definitions, &used.definitions)).is_equal_to(true);
        asserting("Dependencies match").that(&do_vecs_match(&expected_dependencies, &used.dependencies)).is_equal_to(true);
    }

    fn do_vecs_match<T : PartialEq>(a : &Vec<T>, b : &Vec<T>) -> bool {
        let matching = a.iter().zip(b.iter()).filter(|&(a, b)| a == b).count();
        matching == a.len() && matching == b.len()
    }

}
