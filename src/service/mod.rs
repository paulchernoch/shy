use std::sync::RwLock;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};

pub mod request;
pub mod routes;
pub mod service_state;

use routes::expression_execute;
use service_state::ServiceState;
use crate::cache::Cache;

// ........................................................................
//      Simple API Endpoint Functions
// (Complex routes each have their own file in the routes directory.)

const SERVICE_NAME : &str = "Shy Rules Engine";
const SERVICE_VERSION : &str  = "0.1";

#[get("/")]
fn index(data: web::Data<RwLock<ServiceState>>) -> impl Responder {
    let mut state = data.write().unwrap();
    state.tally();
    let plural = if state.request_counter == 1 { "" } else { "s" };
    HttpResponse::Ok().body(format!("{} version {}. \n{} request{} received since service started.\nRulesets in cache: {}", 
    SERVICE_NAME, SERVICE_VERSION, state.request_counter, plural, state.ruleset_cache.size()))
}


// ........................................................................


/// Start the Shy Rules Engine REST Service
/// 
/// Every call to "service" sets up a route handler. 
pub fn shy_service<'a>(ip : &str, port : &str) {
    let service_data = web::Data::new(ServiceState::new(20000));
    {
        println!("{} version {} running on {}:{}", SERVICE_NAME, SERVICE_VERSION, ip, port);
        HttpServer::new(move || {
            App::new()
                .register_data(service_data.clone())
                .service(index)
                .service(expression_execute::route)
        })
        .bind(format!("{}:{}", ip, port))
        .unwrap()
        .run()
        .unwrap();
    }
}

