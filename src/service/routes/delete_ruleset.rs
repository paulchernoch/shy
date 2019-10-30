use std::sync::RwLock;
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use actix_web::{delete, web, HttpResponse};
use log::{warn, info};
extern crate chrono;
use super::super::service_state::ServiceState;
use crate::cache::Cache;

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteRulesetResponse {
    /// Did the request succeed?
    pub success : bool,

    /// Optional error message if request failed.
    pub error : Option<Value>
}

impl DeleteRulesetResponse {
    pub fn new_with_error(error : String) -> Self {
        warn!(target: "service::routes", "Delete RuleSet. {}", error);
        DeleteRulesetResponse { success : false, error : Some(error.into()) }
    }
    pub fn new_with_success() -> Self {
        DeleteRulesetResponse { success : true, error : None }
    }
}

/// Route handler for DELETE /rulesets/{name}. 
#[delete("/rulesets/{name}")]
fn route((path, data): (web::Path<String>, web::Data<RwLock<ServiceState>>)) -> HttpResponse {
    let mut state = data.write().unwrap();
    state.tally();

    let ruleset_name = (*path).clone();
    info!(target: "service::routes", "Delete a RuleSet named '{}'", ruleset_name);
    let response =
      if state.ruleset_cache.remove(&ruleset_name) { DeleteRulesetResponse::new_with_success() }
      else { DeleteRulesetResponse::new_with_error(format!("Unable to delete {}. RuleSet not found.", ruleset_name)) };
    if response.success { HttpResponse::Ok().json(response) }
    else { HttpResponse::NotFound().json(response) }
}
