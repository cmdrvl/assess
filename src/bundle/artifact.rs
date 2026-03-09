use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactReport {
    #[serde(default)]
    pub tool: Option<String>,
    pub version: String,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub refusal: Option<ArtifactRefusal>,
    #[serde(default)]
    pub policy_signals: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRefusal {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactBasisEntry {
    pub artifact: String,
    pub tool: String,
    pub version: String,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub policy_signals: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<ArtifactRefusal>,
}
