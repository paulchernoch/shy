use std::sync::RwLock;
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use actix_web::{get, web, HttpResponse};
use super::super::service_state::ServiceState;
use crate::cache::Cache;

#[derive(Serialize, Deserialize, Debug)]
pub struct ListRulesetsResponse {
    pub ruleset_count: Option<usize>,
    pub ruleset_names : Option<Vec<String>>,
    pub success : bool,
    pub error : Option<Value>
}

impl ListRulesetsResponse {
    pub fn new_with_error(error : String) -> Self {
        ListRulesetsResponse { 
            ruleset_count : None, 
            ruleset_names : None, 
            success : false, 
            error : Some(error.into()) 
        }
    }
    pub fn new_with_success(ruleset_names : Vec<String>) -> Self {
        ListRulesetsResponse { 
            ruleset_count : Some(ruleset_names.len()), 
            ruleset_names : Some(ruleset_names), 
            success : true, 
            error : None 
        }
    }
}

#[get("/rulesets")]
fn route(data: web::Data<RwLock<ServiceState>>) -> HttpResponse {
    let mut state = data.write().unwrap();
    state.tally();
    let mut names = state.ruleset_cache.keys();
    names.sort();
    let response = ListRulesetsResponse::new_with_success(names);

    if response.success {
        HttpResponse::Ok().json(response)
    }
    else {
        HttpResponse::BadRequest().json(response)
    }
}