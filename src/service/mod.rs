
use actix_web::{get, App, HttpResponse, HttpServer, Responder};

pub mod request;
pub mod routes;

use routes::expression_execute;


// ........................................................................
//      Simple API Endpoint Functions
// (Complex routes each have their own file in the routes directory.)

const SERVICE_NAME : &str = "Shy Rules Engine";
const SERVICE_VERSION : &str  = "0.1";

#[get("/")]
fn index() -> impl Responder {
    HttpResponse::Ok().body(format!("{} version {}", SERVICE_NAME, SERVICE_VERSION))
}


// ........................................................................


/// Start the Shy Rules Engine REST Service
/// 
/// Every call to "service" sets up a route handler. 
pub fn shy_service(ip : &str, port : &str) {
    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(expression_execute::route)
    })
    .bind(format!("{}:{}", ip, port))
    .unwrap()
    .run()
    .unwrap();
}

