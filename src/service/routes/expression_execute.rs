use std::sync::RwLock;
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use actix_web::{post, web, HttpResponse};
use crate::parser::execution_context::ExecutionContext;
use crate::parser::ShuntingYard;
use super::super::service_state::ServiceState;


/// Holds the data deserialized from the HTTP request body for the "/expression/execute" route.
#[derive(Serialize, Deserialize, Debug)]
pub struct ExpressionExecuteRequest {
    /// Expression to be evaluated. 
    pub expression : String,

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
    /// having values added or changed by the execution of the expression.
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


// ........................................................................

#[derive(Serialize, Deserialize, Debug)]
pub struct ExpressionExecuteResponse<'a> {
    pub result : Option<Value>,
    pub context : Option<ExecutionContext<'a>>,
    pub error : Option<Value>
}

impl<'a> ExpressionExecuteResponse<'a> {
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }
    pub fn new() -> ExpressionExecuteResponse<'a> {
        ExpressionExecuteResponse { result: None, context: None, error: None }
    }
}

// ........................................................................

// NOTE: The function name "route" is not required - it is a convention of Shy Service.

/// Route handler for /expression/execute. 
#[post("/expression/execute")]
fn route((req, data): (web::Json<ExpressionExecuteRequest>, web::Data<RwLock<ServiceState>>)) -> HttpResponse {
    let mut state = data.write().unwrap();
    state.tally();
    let mut response = ExpressionExecuteResponse::new();
    let shy: ShuntingYard = req.expression.clone().into();
    response.context = Some(ExecutionContext::default());
    match shy.compile() {
        Ok(mut expr) => {
            let exec_result;
            {
                let ctx = response.context.as_mut().unwrap();
                match &req.context {
                    Some(value) => { ctx.store(&req.context_name, value); },
                    None => ()
                }
                exec_result =
                  if req.trace_on { expr.trace(ctx) }
                  else { expr.exec(ctx) };
            }
            if !req.return_context { response.context = None; }
            match exec_result {
                Ok(answer) => { response.result = Some(answer.into()); },
                Err(msg) => { response.error = Some(Value::String(format!("Error executing {}: {}", req.expression, msg))); }
            };
        },
        Err(msg) => { 
            response.context = None;
            response.error = Some(Value::String(format!("Error compiling {}: {}", req.expression, msg)));
        }
    };
    
    if !response.has_error() {
        HttpResponse::Ok().json(response)
    }
    else {
        HttpResponse::BadRequest().json(response)
    }
}
