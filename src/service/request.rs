
use serde::{Serialize, Deserialize};
use serde_json::{Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct ExpressionExecuteRequest {
    pub expression : String,
    pub context : Option<Value>,
    pub trace_on : bool
}

