use std::result::Result;
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use crate::parser::execution_context::ExecutionContext;
use crate::parser::expression::{Expressive, Expression};
use crate::parser::shy_token::ShyValue;
use crate::parser::shy_scalar::ShyScalar;
use super::{Rule, RuleType};

#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
/// Criteria to to decide if a `RuleSet` passes. 
/// 
/// If the execution of any `Rules` results in an error, the `RuleSet` may still pass 
/// or fail (instead of yielding an error) in the following situations:
/// 
///   - For `AlwaysPass`, pass in all situations
///   - For `NeverPass`, fail in all situations
///   - For `LastPasses`, if the final predicate that is applicable does not have an error, 
///     the `RuleSet` will pass or fail based on that one `Rule`
///   - For `AnyPass`, if at least one predicate is applicable and passes, the `RuleSet` passes. 
///   - For `MajorityPass`, if at least one more than half of the predicate `Rules` pass, the `RuleSet` passes,
///     and if at least one more than half fail (without error), the `RuleSet` fails. 
///   - For `AllPass`, if any predicate has an error, the result is an error. 
pub enum SuccessCriteria {
    /// The RuleSet is asserted to always fail. 
    NeverPass,
    
    /// If all predicates that are relevant are true, and at least one predicate is relevant, the RuleSet passes. 
    AllPass,
    
    /// If one more than half of all relevant predicate rules pass, the RuleSet passes. 
    MajorityPass,

    /// If at least one predicate Rule is relevant and true, the RuleSet passes. 
    AnyPass,

    /// If the last predicate that is relevant passes, the RuleSet passes. 
    /// This handles the use case where the last Rule combines the results of all previous
    /// Rules with AND/OR logic to decide the overall result of the RuleSet. 
    LastPasses,

    /// The RuleSet is asserted to always pass. 
    /// The use case is RuleSets that merely set properties and make no assertion of pass or fail. 
    AlwaysPass
}

impl From<&str> for SuccessCriteria {
    fn from(s : &str) -> Self {
        if s == "NeverPass" { SuccessCriteria::NeverPass }
        else if s == "AllPass" { SuccessCriteria::AllPass }
        else if s == "MajorityPass" { SuccessCriteria::MajorityPass }
        else if s == "AnyPass" { SuccessCriteria::AnyPass }
        else if s == "LastPasses" { SuccessCriteria::LastPasses }
        else if s == "AlwaysPass" { SuccessCriteria::AlwaysPass }
        else { SuccessCriteria::LastPasses }
    }
}

impl From<String> for SuccessCriteria {
    fn from(s : String) -> Self {
        s.as_str().into()
    }
}

// ............................................................................

/// Holds the results of executing a RuleSet. 
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleSetResult<'a> {
    pub ruleset_name : String, 

    /// Criteria in force when the overall decision was made as to whether the `RuleSet` passed or failed.
    pub criteria_used : SuccessCriteria,

    /// Count of Rules that only set properties.
    /// These are not expected to contribute the decision of whether the `RuleSet` passed or failed. 
    pub property_rule_count : usize,

    /// Count of Rules whose type is `Category`.
    pub category_rule_count : usize,

    /// Count of all Rules found to be **applicable** in the give context.
    /// Only applicable Rules are expected to contribute to the decision 
    /// as to whether the `RuleSet` passed or failed. 
    pub applicable_rule_count : usize,

    /// Count of all Rules found to be **inapplicable** in the give context.
    /// Such Rules are NOT expected to contribute to the decision 
    /// as to whether the `RuleSet` passed or failed. 
    pub inapplicable_rule_count : usize,

    /// Count of how many applicable `Rules` passed. 
    pub passing_applicable_rule_count : usize,

    /// True if the last applicable `Rule` (when taken in evaluation order) passed. 
    /// If criteria_used is `LastPasses`, then this is all we need to know to decide if the `RuleSet` passed. 
    pub did_last_applicable_rule_pass : bool,

    /// Value on top of the execution stack after the last applicable `Rule` is executed. 
    /// If all rules had errors or none were applicable, this will equal None. 
    pub last_applicable_rule_value : Option<Value>,

    /// Given `criteria_used` and the various counts of passing and failing rules,
    /// this value is computed and is the final judgement on whether the `RuleSet` passed or failed. 
    /// If true, the `RuleSet` decisively passed, regardless of whether there were any errors. 
    /// If false, it does not necessarily mean that the `RuleSet` failed;
    /// it could have resulted in an error.
    pub did_ruleset_pass : bool,

    /// If true, the `RuleSet` decisively failed, regardless of whether there were any errors. 
    /// If this is false, it does not necessarily mean that the `RuleSet` passed;
    /// it could have resulted in an error.
    pub did_ruleset_fail : bool,

    /// Count of the `Rules` of all types that had an error during execution. 
    /// It is possible for some rules to have errors yet have the `RuleSet` pass. 
    /// It all depends on `criteria_used`. 
    pub rules_with_errors_count : usize,

    /// A summary of any and all errors encountered during execution.
    /// The final result of the `RuleSet` is only an error if both 
    /// `did_ruleset_pass` and `did_ruleset_fail` are false.
    pub errors : Vec<String>,

    pub context : ExecutionContext<'a>
}

impl<'a> RuleSetResult<'a> {
    pub fn new<T>(name: T, criteria_used : SuccessCriteria, context : ExecutionContext<'a>) -> Self 
    where T : Into<String>
    {
        RuleSetResult {
            ruleset_name : name.into(),
            criteria_used,
            property_rule_count : 0,
            category_rule_count : 0,
            applicable_rule_count : 0,
            inapplicable_rule_count : 0,
            passing_applicable_rule_count : 0, 
            did_last_applicable_rule_pass : false,
            last_applicable_rule_value : None,
            did_ruleset_pass : false,
            did_ruleset_fail : false,
            rules_with_errors_count : 0,
            errors : Vec::new(),
            context
        }
    }

    pub fn empty() -> Self 
    {
        RuleSetResult {
            ruleset_name : "empty".into(),
            criteria_used : SuccessCriteria::LastPasses,
            property_rule_count : 0,
            category_rule_count : 0,
            applicable_rule_count : 0,
            inapplicable_rule_count : 0,
            passing_applicable_rule_count : 0, 
            did_last_applicable_rule_pass : false,
            last_applicable_rule_value : None,
            did_ruleset_pass : false,
            did_ruleset_fail : false,
            rules_with_errors_count : 0,
            errors : Vec::new(),
            context : ExecutionContext::empty()
        }
    }

    /// After all other values in the structure have been computed, decide on the values of did_ruleset_pass and did_ruleset_fail.
    pub fn decide_pass_fail(mut self) -> Self {
        // Interpret the RuleSet execution according to the criteria_used.
        let (passed, failed) = 
            match self.criteria_used {
                SuccessCriteria::NeverPass => (false, true),
                SuccessCriteria::AllPass => (
                    self.passing_applicable_rule_count == self.applicable_rule_count && self.applicable_rule_count > 0,
                    self.passing_applicable_rule_count < self.applicable_rule_count || self.applicable_rule_count == 0
                ),
                SuccessCriteria::MajorityPass => (
                    self.passing_applicable_rule_count > self.applicable_rule_count / 2,
                    self.passing_applicable_rule_count <= self.applicable_rule_count / 2
                ),
                SuccessCriteria::AnyPass => (self.passing_applicable_rule_count > 0, self.passing_applicable_rule_count == 0),
                SuccessCriteria::LastPasses => (self.did_last_applicable_rule_pass, !self.did_last_applicable_rule_pass),
                SuccessCriteria::AlwaysPass => (true, false)
            };
        self.did_ruleset_pass = passed;
        self.did_ruleset_fail = failed;
        self
    }
}

// ............................................................................


/// A collection of Rules that must be executed in a prescribed order to the end of producing a single result, 
/// usually a true-false value to indicate whether the RuleSet passes or fails.
/// 
/// RuleSets can produce a result other than a boolean, such as those that yield more than two categories.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleSet<'a> {
    /// Unique Name of RuleSet
    pub name : String,

    /// Presumed first part of path variables used in expressions as part of user supplied context. 
    /// 
    /// For example, if your rules are all about the properties of Subsea oil wells, such as well.water_depth
    /// and well.drilling_depth, then "well" would be the appropriate `context_name`.
    /// This name must be a valid expression name conforming to the `Expression` syntax.
    /// That means no spaces, hyphens, or leading digits. Underscores, letters, non-leading digits and the
    /// dolloar sign ($) are permitted, and possibly some other characters.
    pub context_name : String,

    /// Criteria used during execution to decide if the RuleSet passes. 
    pub criteria : SuccessCriteria,

    /// Optional RuleSet category, useful for filtering, but not involved in rule execution.
    pub category : Option<String>,

    /// The Rules to be executed, assumed to be properly sorted so that no Rule with a dependency on another Rule
    /// is listed before that dependency. 
    pub rules: Vec<Rule<'a>>
}

impl<'a> RuleSet<'a> {
    /// Construct a RuleSet from a list of uncompiled rules and sort them in dependency order. 
    /// 
    /// The `uncompiled_rules` must follow the syntax for valid `Expressions`.
    /// The parsing and compilation of individual `Rules`, if it includes assignments to path variables
    /// of the form `rule.some_name`, will be used to infer values for properties of the `Rule`.
    /// See the definition of the `Rule` struct for specifics.
    /// 
    /// If any of the rules fail to compile, do not sort the rules, then return an Err, otherwise an Ok. 
    /// If sorting fails because of circular dependencies, return an Err.
    /// If an Err is returned, all compiled rules will still be returned, and some may be marked as having an error. 
    pub fn new<T>(name : T, context_name : T, criteria : SuccessCriteria, category : Option<String>, uncompiled_rules : &Vec<String>) -> Result<Self,Self> 
    where T : Into<String>
    {
        let mut ruleset = RuleSet { name : name.into(), context_name : context_name.into(), criteria, category, rules : Vec::new() };
        let mut has_errors = false;
        let mut unsorted_rules = Vec::new();
        for (i, rule_source) in uncompiled_rules.iter().enumerate() {
            let rule = Rule::new(rule_source, i+1, None);
            if rule.expression.had_compile_error() {
                has_errors = false;
            }
            unsorted_rules.push(rule);
        }
        if has_errors { 
            ruleset.rules = unsorted_rules;
            Err(ruleset)
        } 
        else {
            // Sort the rules
            match Expression::sort(unsorted_rules) {
                Ok(rules) => {
                    ruleset.rules.extend(rules);
                    Ok(ruleset)
                },
                Err(rules) => {
                    ruleset.rules.extend(rules);
                    Err(ruleset)
                }
            }
        }
    }

    /// Construct a `RuleSet` from `ruleset_text`, a single block of text, then sort the `Rules` in dependency order. 
    /// The `Rules` must conform to the syntax permitted for `Expression` structs.
    /// 
    /// Several attributes of the `RuleSet` will be optionally parsed from the text:
    /// 
    ///   - ruleset.name - If present, use this to set the `name`. If omitted, use "Untitled".
    ///   - ruleset.context_name - If present, use this to set the `context_name`. If omitted, use "$".
    ///   - ruleset.criteria - If present, use to set the `criteria`. If omitted, use `LastPasses`.
    ///   - ruleset.category - If present, use to set the `category`. If omitted, use `None`.
    /// 
    /// In like fashion, for each individual `Rule`, properties of that `Rule` may be inferred 
    /// by searching the executable statements for assignments to path variables like `rule.name` and `rule.id`. 
    /// See the definition of the `Rule` struct for specifics.
    /// 
    /// If `single_newline_separates_rules` is true, then assume one rule per line.
    /// Otherwise, assume that `Rules` may span multiple lines and are separated by one or more consecutive 
    /// blank lines. A blank line consists of zero or more spaces or tabs followed by a newline.
    /// 
    ///   - If any of the rules fail to compile, do not sort the rules, then return an `Err`, otherwise an `Ok`. 
    ///   - If sorting fails because of circular dependencies, return an `Err`.
    ///   - If an `Err` is returned, all compiled rules will still be returned, and some may be marked as having an error. 
    pub fn new_from_text<T>(ruleset_text : T, single_newline_separates_rules : bool) -> Result<Self,Self> 
    where T : Into<String> {
        let mut rule_source = Vec::new();
        let mut hold = String::new();
        for line in ruleset_text.into().lines() {
            // Check if the string is all white space
            if line.trim().is_empty() {
                if hold.len() > 0 {
                    rule_source.push(hold);
                    hold = String::new();
                }
            }
            else {
                if single_newline_separates_rules {
                    rule_source.push(line.to_string());
                }
                else {
                    hold.push_str(line);
                    hold.push('\n');
                }
            }
        }
        if hold.len() > 0 {
            rule_source.push(hold);
        }
        let ruleset_opt = RuleSet::new("Untitled", "$", SuccessCriteria::LastPasses, None, &rule_source);
        if ruleset_opt.is_err() { return ruleset_opt }
        let mut ruleset = ruleset_opt.unwrap();
        ruleset.apply_ruleset_variables();
        Ok(ruleset)
    }

    /// Execute the `RuleSet` and extract some variables from the context to set the `RuleSet` `name`, `criteria` and `category`. 
    fn apply_ruleset_variables(&mut self) {
        // The Context does not need any of the variables expected by the formulas in the RuleSet.
        // Most of the Rules can fail, but the parts that define these properties will likely succeed, as all they
        // do is assign a string to a variable. 
        let ruleset_name;
        let ruleset_context_name;
        let ruleset_criteria;
        let ruleset_category;
        {
            // TODO: The lifetimes of RuleSet, RuleSetResult and ExecutionContext become entangled,
            // so we need the latter two to go out of scope so that we can release the borrow on RuleSet, then continue initializing it. 
            let mut context = ExecutionContext::default();
            let exec_result = self.exec(&mut context, false);
            ruleset_name = exec_result.context.get_string_property_chain("ruleset.name", "Untitled".into());
            ruleset_context_name = exec_result.context.get_string_property_chain("ruleset.context_name", "$".into());
            ruleset_criteria = exec_result.context.get_string_property_chain("ruleset.criteria", "LastPasses".into());
            ruleset_category = Rule::string_or_none(&exec_result.context.get_string_property_chain("rule.category", "".into()));
        }
        self.name = ruleset_name;
        self.context_name = ruleset_context_name;
        self.criteria = ruleset_criteria.into();
        self.category = ruleset_category;
    }

    /// Execute all the `Expressions` in the `RuleSet`, decide if it passes or fails, and return a structure
    /// that explains the results, which could be an error.  
    pub fn exec(&mut self, context : &ExecutionContext<'a>, trace_on : bool) -> RuleSetResult 
    {
        // TODO: This cloning of the context needs rework - it won't copy the functions. 
        // Thus any beside the default functions will be lost. 
        // However, merely changing the signature to a mutable reference to the passed in context won't work,
        // because the RuleSet and ExecutionContext get tangled by the borrow checker and I can't find a resolution. 
        let mut result = RuleSetResult::new(self.name.clone(), self.criteria, context.clone());
        let mut exec_result;

        // Loop through all the Rules in the RuleSet and evaluate them against the context. 
        for rule in self.rules.iter_mut() {
            {
                // Execute a rule
                let expr = rule.express_mut();
                exec_result =
                    if trace_on { expr.trace(&mut result.context) }
                    else { expr.exec(&mut result.context) };
            }
            // Capture value of rule execution, including whether it had an error.
            let (rule_value, rule_had_error) =
                match exec_result {
                    Ok(ShyValue::Scalar(ShyScalar::Error(error_val))) => {
                        result.rules_with_errors_count += 1;
                        result.errors.push(format!("Rule `{}` had error: {:?}", rule.name, error_val));
                        (ShyValue::error(error_val), true)
                    },
                    Ok(val) => {
                        (val, false)
                    },
                    Err(error_val) => {
                        result.rules_with_errors_count += 1;
                        result.errors.push(format!("Rule `{}` had error: {:?}", rule.name, error_val));
                        (ShyValue::error(error_val), true)
                    }
                };
            // Interpret the rule execution value according to the RuleType.
            match rule.rule_type {
                RuleType::Property => { result.property_rule_count += 1; },
                RuleType::Category => { result.category_rule_count += 1; }
                RuleType::Predicate => {
                    if result.context.is_applicable {
                        result.applicable_rule_count += 1;
                        if !rule_had_error {
                            result.passing_applicable_rule_count += 1;
                            result.did_last_applicable_rule_pass = true;
                        }
                        else {
                            result.did_last_applicable_rule_pass = false;
                        }
                        result.last_applicable_rule_value = Some(rule_value.into());
                    }
                    else {
                        result.inapplicable_rule_count += 1;
                    }
                },
            }
        }
        result.decide_pass_fail()
    }
}

#[cfg(test)]
/// Tests of the RuleSet.
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    const CAR_RULESET : &str = r#"
          rule.name = "RuleSet header"
          rule.type = "Property";
          ruleset.name = "Decide if car worth buying";
          ruleset.context_name = "car";
          ruleset.criteria = "MajorityPass";
          ruleset.category = "Test";
          applicable = false? ;

          rule.name = "car age";
          rule.type = "Predicate";
          not_too_old = car.age < 8 || (car.age < 12 && car.make == "Honda");

          rule.name = "car price";
          rule.type = "Predicate";
          good_price = min(50000 / car.age, 30000);
          not_too_expensive = car.price < good_price;

          rule.name = "car miles driven";
          rule.type = "Predicate";
          good_miles_driven = car.miles_driven < 100000 || (car.miles_driven < 150000 && car.make == "Honda");
          
          rule.name = "car accidents";
          rule.type = "Predicate";
          not_too_many_accidents = car.accidents == 0 || (car.accidents <= 1 && car.make == "BMW");
        "#;

    /// Test the execution of a RuleSet where the Rules come in already ordered properly and we expect the majority to pass.
    /// Assume that a blank line separates Rules, so that Rules may span multiple lines of text, for readability.
    #[test]
    fn exec_ordered_majority_pass() {
        let result = RuleSet::new_from_text(CAR_RULESET, false);

        asserting("Compiling rule").that(&result.is_err()).is_equal_to(false);

        let mut ruleset = result.unwrap();

        asserting("Number of rules is correct").that(&ruleset.rules.len()).is_equal_to(5);
        asserting("RuleSet name is correct").that(&ruleset.name).is_equal_to("Decide if car worth buying".to_string());
        asserting("RuleSet criteria is correct").that(&ruleset.criteria).is_equal_to(SuccessCriteria::MajorityPass);

        let mut context = ExecutionContext::default();
        let mut _r;
        _r = context.store_chain_string("car.make", "Honda".into());
        _r = context.store_chain_string("car.age", 10.into());
        _r = context.store_chain_string("car.miles_driven", 120000.into());
        _r = context.store_chain_string("car.price", 4750.into());
        _r = context.store_chain_string("car.accidents", 2.into());

        let exec_result = ruleset.exec(&context, false);

        println!("For exec_ordered_majority_pass, Execution Result =\n{:?}\n", exec_result);

        asserting("Car should be worth buying").that(&exec_result.did_ruleset_pass).is_equal_to(true);
        asserting("3 of 4 tests should have passed").that(&exec_result.passing_applicable_rule_count).is_equal_to(3);
        asserting("4 of 5 tests should be applicable").that(&exec_result.applicable_rule_count).is_equal_to(4);
        
    }

}
