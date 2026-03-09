use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::codes::RefusalCode;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefusalPayload {
    pub code: String,
    pub message: String,
    #[serde(default = "empty_detail")]
    pub detail: Value,
    #[serde(default)]
    pub next_command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefusalEnvelope {
    pub tool: String,
    pub version: String,
    pub decision_band: Option<String>,
    pub refusal: RefusalPayload,
}

impl RefusalEnvelope {
    pub fn new(code: RefusalCode, message: impl Into<String>) -> Self {
        Self {
            tool: "assess".to_owned(),
            version: "assess.v0".to_owned(),
            decision_band: None,
            refusal: RefusalPayload {
                code: code.as_str().to_owned(),
                message: message.into(),
                detail: empty_detail(),
                next_command: None,
            },
        }
    }
}

fn empty_detail() -> Value {
    json!({})
}
