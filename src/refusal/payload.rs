use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::codes::RefusalCode;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefusalPayload {
    pub code: RefusalCode,
    pub message: String,
    #[serde(default = "empty_detail")]
    pub detail: Value,
    pub next_command: String,
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
                code,
                message: message.into(),
                detail: empty_detail(),
                next_command: code.next_command().to_owned(),
            },
        }
    }

    pub fn with_detail(mut self, detail: Value) -> Self {
        self.refusal.detail = detail;
        self
    }

    pub fn with_next_command(mut self, next_command: impl Into<String>) -> Self {
        self.refusal.next_command = next_command.into();
        self
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("RefusalEnvelope is always serializable")
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("RefusalEnvelope is always serializable")
    }
}

fn empty_detail() -> Value {
    json!({})
}
