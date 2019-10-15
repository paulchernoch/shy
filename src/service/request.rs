
use serde::{Serialize, Deserialize};
use serde_json::{Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct ExpressionExecuteRequest {
    pub expression : String,

    #[serde(default = "default_context")]
    pub context : Option<Value>,

    #[serde(default = "default_context_name")]
    pub context_name : String,

    #[serde(default = "default_trace_on")]
    pub trace_on : bool
}

fn default_context() -> Option<Value> {
    None
}
fn default_context_name() -> String {
    "$".into()
}
fn default_trace_on() -> bool {
    false
}

