use std::sync::RwLock;
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use actix_web::{put, web, HttpResponse};
use crate::rule::ruleset::{RuleSet, SuccessCriteria};
use super::super::service_state::ServiceState;
use crate::cache::Cache;

#[derive(Serialize, Deserialize, Debug)]
/// Request for creating a RuleSet and adding it to the Cache.
/// 
/// The RuleSet name is specified in the URL, not the body of the PUT request.
pub struct AddRulesetRequest {
    /// Criteria by which to judge whether a RuleSet succeeds of fails when applied to a context,
    /// defaulting to `LastPasses`.
    #[serde(default = "default_success_criteria")]
    pub criteria : SuccessCriteria,

    /// Uncompiled rules as a list of strings.
    pub rule_source: Vec<String>
}

fn default_success_criteria() -> SuccessCriteria { SuccessCriteria::LastPasses }

#[derive(Serialize, Deserialize, Debug)]
pub struct AddRulesetResponse<'a> {
    pub ruleset: Option<RuleSet<'a>>,
    pub success : bool,
    pub error : Option<Value>
}

impl<'a> AddRulesetResponse<'a> {
    pub fn new_with_error(error : String, compiled_ruleset : Option<RuleSet<'a>>) -> Self {
        AddRulesetResponse { ruleset : compiled_ruleset, success : false, error : Some(error.into()) }
    }
    pub fn new_with_success(compiled_ruleset : RuleSet<'a>) -> Self {
        AddRulesetResponse { ruleset : Some(compiled_ruleset), success : true, error : None }
    }
}


/// Route handler for PUT /rulesets/{name}. 
#[put("/rulesets/{name}")]
fn route((path, req, data): (web::Path<String>, web::Json<AddRulesetRequest>, web::Data<RwLock<ServiceState>>)) -> HttpResponse {
    let mut state = data.write().unwrap();
    state.tally();

    let ruleset_result = RuleSet::new((*path).clone(), req.criteria, &req.rule_source);

    println!("Add Ruleset for {}", *path);

    let response = 
        match ruleset_result {
            Ok(ruleset) => {
                state.ruleset_cache.add_or_replace(&ruleset.name, &ruleset, true);
                AddRulesetResponse::new_with_success(ruleset)
            },
            Err(ruleset_with_errors) => AddRulesetResponse::new_with_error("RuleSet had compilation errors".into(), Some(ruleset_with_errors))
        };
    if response.success {
        HttpResponse::Ok().json(response)
    }
    else {
        HttpResponse::BadRequest().json(response)
    }
}
