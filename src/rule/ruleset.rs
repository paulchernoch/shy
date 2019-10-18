use serde::{Serialize, Deserialize};

use super::Rule;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleSet<'a> {
    pub name : String,
    pub rules: Vec<Rule<'a>>
}
