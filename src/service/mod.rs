
use actix_web::{get, App, HttpResponse, HttpServer, Responder};


// ........................................................................
//      API Endpoint Functions

const SERVICE_NAME : &str = "Shy Rules Engine";
const SERVICE_VERSION : &str  = "0.1";

#[get("/")]
fn index() -> impl Responder {
    HttpResponse::Ok().body(format!("{} version {}", SERVICE_NAME, SERVICE_VERSION))
}


// ........................................................................


/// Start the Shy Rules Engine REST Service
pub fn shy_service(ip : &str, port : &str) {
    HttpServer::new(|| {
        App::new()
            .service(index)
    })
    .bind(format!("{}:{}", ip, port))
    .unwrap()
    .run()
    .unwrap();
}

