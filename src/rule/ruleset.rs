use serde::{Serialize, Deserialize};

use super::Rule;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
/// Criteria to to decide if a RuleSet passes. 
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleSet<'a> {
    pub name : String,
    pub criteria : SuccessCriteria,
    pub rules: Vec<Rule<'a>>
}

impl<'a> RuleSet<'a> {
    /// Construct a RuleSet from a list of uncompiled rules. 
    /// 
    /// If any of the rules fail to compile, return an Err, otherwise an Ok. 
    /// If an Err is returned, all compiled rules will still be returned, with some marked as having an error. 
    pub fn new(name : String, criteria : SuccessCriteria, uncompiled_rules : &Vec<String>) -> Result<Self,Self> {
        let mut ruleset = RuleSet { name, criteria, rules : Vec::new() };
        let mut has_errors = false;
        for (i, rule_source) in uncompiled_rules.iter().enumerate() {
            let rule = Rule::new(rule_source, i+1, None);
            if rule.expression.had_compile_error() {
                has_errors = false;
            }
            ruleset.rules.push(rule);
        }
        if has_errors { Err(ruleset) } 
        else { Ok(ruleset) }
    }
}
