
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use actix_web::{post, web, HttpResponse};
use crate::parser::execution_context::ExecutionContext;
use crate::parser::ShuntingYard;
use super::super::request::ExpressionExecuteRequest;

// ........................................................................

#[derive(Serialize, Deserialize, Debug)]
pub struct ExpressionResponse<'a> {
    pub result : Option<Value>,
    pub context : Option<ExecutionContext<'a>>,
    pub error : Option<Value>
}

impl<'a> ExpressionResponse<'a> {
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }
    pub fn new() -> ExpressionResponse<'a> {
        ExpressionResponse { result: None, context: None, error: None }
    }
}

// ........................................................................

// NOTE: The function name "route" is not required - it is a convention of Shy Service.

/// Route handler for /expression/execute. 
#[post("/expression/execute")]
fn route(req: web::Json<ExpressionExecuteRequest>) -> HttpResponse {
    let mut response = ExpressionResponse::new();
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
    
    if response.has_error() {
        HttpResponse::Ok().json(response)
    }
    else {
        HttpResponse::BadRequest().json(response)
    }
}
