use serde::{Serialize, Deserialize};
use serde_json::{Value};
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};

pub mod request;
use request::ExpressionExecuteRequest;
use crate::parser::{ShuntingYard};
use crate::parser::execution_context::ExecutionContext;

// ........................................................................

#[derive(Serialize, Deserialize, Debug)]
pub struct ExpressionResponse {
    result : Option<Value>,
    context : Option<Value>,
    error : Option<Value>
}

impl ExpressionResponse {
    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }
}


// ........................................................................
//      API Endpoint Functions

const SERVICE_NAME : &str = "Shy Rules Engine";
const SERVICE_VERSION : &str  = "0.1";

#[get("/")]
fn index() -> impl Responder {
    HttpResponse::Ok().body(format!("{} version {}", SERVICE_NAME, SERVICE_VERSION))
}


#[post("/expression/execute")]
fn expression_execute(req: web::Json<ExpressionExecuteRequest>) -> HttpResponse {
    let shy: ShuntingYard = req.expression.clone().into();
    let ctx = &mut ExecutionContext::default();
    let result = 
        match shy.compile() {
            Ok(mut expr) => {
                match &req.context {
                    Some(value) => {
                        ctx.store(&req.context_name, value);
                    },
                    None => ()
                }
                let exec_result = 
                    if req.trace_on { expr.trace(ctx) }
                    else { expr.exec(ctx) };
                let final_context = 
                    if req.trace_on { Some(Value::String(format!("{:?}", ctx))) }
                    else { None };
                match exec_result {
                    Ok(answer) => ExpressionResponse { 
                        result : Some(answer.into()), 
                        context : final_context,
                        error : None
                    },
                    Err(msg) => ExpressionResponse { 
                        result : None, 
                        context : final_context,
                        error : Some(Value::String(format!("Error executing {}: {}", req.expression, msg)))
                    }
                }
            },
            Err(msg) => ExpressionResponse { 
                result : None, 
                context : None,
                error : Some(Value::String(format!("Error compiling {}: {}", req.expression, msg)))
            }
        };
    
    if result.has_error() {
        HttpResponse::Ok().json(result)
    }
    else {
        HttpResponse::BadRequest().json(result)
    }
}

// ........................................................................


/// Start the Shy Rules Engine REST Service
pub fn shy_service(ip : &str, port : &str) {
    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(expression_execute)
    })
    .bind(format!("{}:{}", ip, port))
    .unwrap()
    .run()
    .unwrap();
}

