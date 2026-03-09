use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessRecord {
    pub tool: String,
    pub inputs: Vec<String>,
    #[serde(default)]
    pub policy_id: Option<String>,
    #[serde(default)]
    pub decision_band: Option<String>,
    pub duration_ms: u64,
}

impl WitnessRecord {
    pub fn scaffold(inputs: Vec<String>) -> Self {
        Self {
            tool: "assess".to_owned(),
            inputs,
            policy_id: None,
            decision_band: None,
            duration_ms: 0,
        }
    }
}
