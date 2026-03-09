use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessRecord {
    pub tool: String,
    pub command: String,
    pub inputs: Vec<String>,
    #[serde(default)]
    pub policy_id: Option<String>,
    #[serde(default)]
    pub decision_band: Option<String>,
    #[serde(default)]
    pub duration_ms: u64,
    pub ts: String,
}

impl WitnessRecord {
    pub fn scaffold(inputs: Vec<String>) -> Self {
        Self {
            tool: "assess".to_owned(),
            command: "run".to_owned(),
            inputs,
            policy_id: None,
            decision_band: None,
            duration_ms: 0,
            ts: "0".to_owned(),
        }
    }

    pub fn with_policy_id(mut self, policy_id: impl Into<String>) -> Self {
        self.policy_id = Some(policy_id.into());
        self
    }

    pub fn with_decision_band(mut self, decision_band: impl Into<String>) -> Self {
        self.decision_band = Some(decision_band.into());
        self
    }

    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    pub fn with_timestamp(mut self, timestamp: impl Into<String>) -> Self {
        self.ts = timestamp.into();
        self
    }
}
