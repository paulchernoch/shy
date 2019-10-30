use std::sync::RwLock;
use std::env;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder, middleware::{Logger}};
use env_logger;

pub mod routes;
pub mod service_state;

use routes::expression_execute;
use routes::list_rulesets;
use routes::add_ruleset;
use routes::get_ruleset;
use routes::delete_ruleset;
use routes::execute_ruleset;
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

/// Initialize the env_logger, preferring to get the log level from an environment variable, but
/// using the supplied `default_log_level` if that environment variable is not set. 
fn init_logging(
        level_env_variable_name : &str, default_log_level : &str, 
        format_env_variable_name : &str, default_log_format_string : &str) -> (String, String) {
    let message1 = match env::var(level_env_variable_name) {
        Ok(val) => format!("Logging level set to {:?} using environment variable {}", val, level_env_variable_name),
        Err(_e) => { 
            env::set_var(level_env_variable_name, default_log_level);
            format!("Logging level set to default value of {:?} because environment variable {} unset.", default_log_level, level_env_variable_name)
        }
    };
    let format_string;
    let message2 = match env::var(format_env_variable_name) {
        Ok(val) => {
            format_string = val.to_string();
            format!("Logging format set to {:?} using environment variable {}", val, format_env_variable_name)
        },
        Err(_e) => {
            format_string = default_log_format_string.to_string();
            env::set_var(format_env_variable_name, default_log_format_string);
            format!("Logging format set to default value of {:?} because environment variable {} unset.", default_log_format_string, format_env_variable_name)
        }
    };
    env_logger::init();
    (format_string, format!("  - {}\n  - {}", message1, message2))
}

/// Start the Shy Rules Engine REST Service
/// 
/// Every call to "service" sets up a route handler. 
pub fn shy_service<'a>(ip : &str, port : &str) {
    let service_data = web::Data::new(ServiceState::new(20000));
    {
        println!("{} version {} running on {}:{}", SERVICE_NAME, SERVICE_VERSION, ip, port);
        // For the available log message format specifiers, see this page: 
        //   https://docs.rs/actix-web/1.0.0/actix_web/middleware/struct.Logger.html
        /*
            %% The percent sign
            %a Remote IP-address (IP-address of proxy if using reverse proxy)
            %t Time when the request was started to process (in rfc3339 format)
            %r First line of request
            %s Response status code
            %b Size of response in bytes, including HTTP headers
            %T Time taken to serve the request, in seconds with floating fraction in .06f format
            %D Time taken to serve the request, in milliseconds
            %U Request URL
            %{FOO}i request.headers['FOO']
            %{FOO}o response.headers['FOO']
            %{FOO}e os.environ['FOO']
        */

        let (log_format_string, message) = init_logging(
            "RUST_LOG", 
            "actix_web=info", 
            "RUST_LOG_FORMAT", // Unique to Shy - Not a Rust or Actix variable name
            r#"%a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#
        );
        println!("Logging initialized.\n{}", message);
        HttpServer::new(move || {
            App::new()
                .wrap(Logger::new(&log_format_string))
                .register_data(service_data.clone())
                .service(index)
                .service(expression_execute::route)
                .service(list_rulesets::route)
                .service(add_ruleset::route)
                .service(get_ruleset::route)
                .service(delete_ruleset::route)
                .service(execute_ruleset::route)
        })
        .bind(format!("{}:{}", ip, port))
        .unwrap()
        .run()
        .unwrap();
    }
}

