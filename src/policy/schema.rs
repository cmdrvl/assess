use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyFile {
    pub schema_version: u32,
    pub policy_id: String,
    pub policy_version: u32,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub when: Option<WhenClause>,
    pub then: ThenClause,
}

/// A when clause maps tool names to their match conditions.
pub type WhenClause = BTreeMap<String, ToolMatcher>;

/// Match conditions for a single tool within a when clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolMatcher {
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub outcome_in: Option<Vec<String>>,
    #[serde(default)]
    pub refusal: Option<String>,
    #[serde(default)]
    pub signals: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThenClause {
    pub decision_band: DecisionBand,
    #[serde(default)]
    pub risk_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionBand {
    #[serde(rename = "PROCEED")]
    Proceed,
    #[serde(rename = "PROCEED_WITH_RISK")]
    ProceedWithRisk,
    #[serde(rename = "ESCALATE")]
    Escalate,
    #[serde(rename = "BLOCK")]
    Block,
}

impl DecisionBand {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proceed => "PROCEED",
            Self::ProceedWithRisk => "PROCEED_WITH_RISK",
            Self::Escalate => "ESCALATE",
            Self::Block => "BLOCK",
        }
    }
}
