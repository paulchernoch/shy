use std::sync::RwLock;
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use actix_web::{get, web, HttpResponse};
use super::super::service_state::ServiceState;
use crate::cache::Cache;

/// Holds the optional query parameters for the route
#[derive(Serialize, Deserialize, Debug)]
pub struct ListRulesetsQuery {
    #[serde(default = "default_category")]
    pub category: String
}

/// The default category for the query is asterisk "*" which means "all".
fn default_category() -> String { "*".into() }

/// Defines the response sent to the caller for this route, which lists the names of 
/// all RuleSets matching the criteria, sorted alphabetically.
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

/// Route handler for GET /rulesets with query parameter to filter by category.
/// If no parameter is given, all names are returned.
/// 
/// Usage:
/// 
///   - GET /rulesets
///   - GET /rulesets/category=cat
#[get("/rulesets")]
fn route((query, data): (web::Query<ListRulesetsQuery>, web::Data<RwLock<ServiceState>>)) -> HttpResponse {
    let mut state = data.write().unwrap();
    state.tally();
    let wildcard = "*".to_string();
    
    let mut names : Vec<String> = state.ruleset_cache
      .values().iter()
      .filter(|r| 
          if let Some(ref cat) = (*r).category { *cat == query.category || query.category == wildcard } 
          else { query.category == wildcard }
      )
      .map(|r| r.name.clone())
      .collect();
    names.sort();
    let response = ListRulesetsResponse::new_with_success(names);

    if response.success {
        HttpResponse::Ok().json(response)
    }
    else {
        HttpResponse::BadRequest().json(response)
    }
}
