use serde::{Serialize, Deserialize};

use super::Rule;

#[derive(Serialize, Deserialize, Debug)]
pub struct RuleSet {
    pub name : String,
    pub rules: Vec<Rule>
}
