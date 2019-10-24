use std::sync::RwLock;
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use actix_web::{post, web, HttpResponse};
use super::super::service_state::ServiceState;
use crate::cache::Cache;
use crate::parser::execution_context::ExecutionContext;

#[derive(Serialize, Deserialize, Debug)]
/// Request for when you load a RuleSet from Cache and execute it against a context supplied as the posted data.
/// 
/// The RuleSet name is specified in the URL, not the body of the POST request.
pub struct ExecuteRulesetRequest {
    /// Optional context that defines variables accessible to the expression. 
    #[serde(default = "default_context")]
    pub context : Option<Value>,

    /// The caller supplied context will be stored in the ExecutionContext under this name,
    /// which defaults to a dollar sign ($). 
    /// Thus if the context has a property "depth" and context_name is "well", then to
    /// access the variable in an expression, use "well.depth". 
    #[serde(default = "default_context_name")]
    pub context_name : String,

    /// If true, the updated context will be returned,
    /// having values added or changed by the execution of the RuleSet.
    #[serde(default = "default_return_context")]
    pub return_context : bool,

    /// If true, a detailed log of the execution of the expression will be logged to the console.
    #[serde(default = "default_trace_on")]
    pub trace_on : bool
}

// Supply these default values for missing fields when Deserializing. 

fn default_context() -> Option<Value> { None }
fn default_return_context() -> bool { false }
fn default_context_name() -> String { "$".into() }
fn default_trace_on() -> bool { false }

/// Response object to send back to caller with results of executing the `RuleSet`.
#[derive(Serialize, Deserialize, Debug)]
pub struct ExecuteRulesetResponse {
    /// Did the command succeed or produce an error? 
    /// A true value here does not mean that the `RuleSet` passed, just that no service errors occurred.
    pub did_command_succeed : bool,

    /// Did enough of the Expressions in the `RuleSet` return `true` such that the `RuleSet` passed the test?
    pub passed : bool,

    /// Did too many of the Expressions in the `RuleSet` return `false` such that the `RuleSet` failed the test?
    pub failed : bool,

    /// If requested, the context that shows the results of many intermediate computations.
    /// 
    /// NOTE: Would have preferred this to be an ExecutionContext, but the Borrow Checker made it problematic.
    pub context : Option<Value>,

    /// Errors found while processing the `RuleSet`.
    ///   - It is possible for a `RuleSet` to pass or fail even in the presence of errors, 
    ///     depending on the `SuccessCriteria` employed.
    ///   - If `passed` and `failed` are both `false` but `did_command_succeed` is `true`, the execution encountered a data error, 
    ///     likely due to syntax errors in the expressions or expected values missing from the supplied context. 
    ///   - If `passed` and `failed` and `did_command_succeed` are all `false`, the service encountered a more serious error, possibly
    ///     because the `RuleSet` was not in the cache. 
    pub errors : Option<Value>
}

impl ExecuteRulesetResponse {
    pub fn new_with_error(error : String) -> Self {
        ExecuteRulesetResponse { context : None, did_command_succeed : false, passed : false, failed : false, errors : Some(error.into()) }
    }
    pub fn new_without_context() -> Self {
        ExecuteRulesetResponse { context : None, did_command_succeed : false, passed : false, failed : false, errors : None }
    }
}

/// Execute a RuleSet: the route handler for POST /rulesets/{name}. 
#[post("/rulesets/{name}")]
fn route((path, req, data): (web::Path<String>, web::Json<ExecuteRulesetRequest>, web::Data<RwLock<ServiceState>>)) -> HttpResponse {
    let mut state = data.write().unwrap();
    state.tally();

    let ruleset_name = (*path).clone();
    let http_response; 
    let found_in_cache;
    let mut ruleset;
    let mut exec_response = ExecuteRulesetResponse::new_without_context();
    {
        match state.ruleset_cache.get(&ruleset_name) {
            Some((ruleset_from_cache, _time)) => {
                found_in_cache = true;
                
                // TODO: This clone of a whole RuleSet is an expensive abomination,
                // but the calls to `trace` deeper in the code modify the Expression temporarily, 
                // and that is not threadsafe. Need to refactor trace to have state passed in, maybe a logger. 
                ruleset = ruleset_from_cache.clone();
                let mut context = ExecutionContext::default();

                // Add data sent by caller to the context.
                if let Some(value) = &req.context {
                    context.store(&req.context_name, value.clone()); 
                }
                let exec_ruleset_result = ruleset.exec(&context, req.trace_on).clone();
            
                // Transcribe values from exec_ruleset_result into exec_response, then into HttpResponse.
                exec_response.did_command_succeed = exec_ruleset_result.did_ruleset_pass || exec_ruleset_result.did_ruleset_fail;
                exec_response.passed = exec_ruleset_result.did_ruleset_pass;
                exec_response.failed = exec_ruleset_result.did_ruleset_fail;
                if exec_ruleset_result.errors.len() > 0 {
                    exec_response.errors = Some(exec_ruleset_result.errors.clone().into());
                }
                if req.return_context { exec_response.context = Some(exec_ruleset_result.context.into()); }
            },
            None => { found_in_cache = false; }
        }
    }
    if found_in_cache {
        http_response = HttpResponse::Ok().json(exec_response);
    }
    else {
        http_response = HttpResponse::NotFound().json(ExecuteRulesetResponse::new_with_error(format!("Unable to find RuleSet {} in cache", ruleset_name))); 
    }
    http_response
}
