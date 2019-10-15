
use serde::{Serialize, Deserialize};
use serde_json::{Value};

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

