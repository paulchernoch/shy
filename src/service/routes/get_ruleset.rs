use std::sync::RwLock;
use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use actix_web::{get, web, HttpResponse};
extern crate chrono;
use chrono::offset::Utc;
use chrono::DateTime;
use super::super::service_state::ServiceState;
use crate::cache::Cache;
use crate::rule::ruleset::RuleSet;

#[derive(Serialize, Deserialize, Debug)]
pub struct GetRulesetResponse<'a> {
    /// Requested RuleSet, compiled and with all rules sorted in dependency order.
    pub ruleset: Option<RuleSet<'a>>,

    /// When was the RuleSet added to the cache?
    pub created: Option<String>,

    /// Did the request succeed?
    pub success : bool,

    /// Optional error message if request failed.
    pub error : Option<Value>
}

impl<'a> GetRulesetResponse<'a> {
    pub fn new_with_error(error : String, compiled_ruleset : Option<RuleSet<'a>>) -> Self {
        GetRulesetResponse { ruleset : compiled_ruleset, created : None, success : false, error : Some(error.into()) }
    }
    pub fn new_with_success(compiled_ruleset : RuleSet<'a>, created : Option<SystemTime>) -> Self {
        let date_time_string_opt = match created {
            Some(dt) => {
              let datetime: DateTime<Utc> = dt.into();
              Some(datetime.to_rfc2822())
            },
            None => None
        };
        GetRulesetResponse { ruleset : Some(compiled_ruleset), created : date_time_string_opt, success : true, error : None }
    }
}

/// Route handler for GET /rulesets/{name}. 
#[get("/rulesets/{name}")]
fn route((path, data): (web::Path<String>, web::Data<RwLock<ServiceState>>)) -> HttpResponse {
    let mut state = data.write().unwrap();
    state.tally();

    let ruleset_name = (*path).clone();
    let response = 
        match state.ruleset_cache.get(&ruleset_name) {
            Some((ruleset, time)) => GetRulesetResponse::new_with_success(ruleset, Some(time)),
            None => GetRulesetResponse::new_with_error(format!("Unable to find RuleSet {}", ruleset_name), None)
        };
    if response.success { HttpResponse::Ok().json(response) }
    else { HttpResponse::NotFound().json(response) }
}
