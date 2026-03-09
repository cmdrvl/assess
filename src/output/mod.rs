pub mod human;
pub mod json;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::bundle::{ArtifactBasisEntry, ArtifactBundle};
use crate::evaluate::Decision;
use crate::policy::LoadedPolicy;
use crate::refusal::RefusalEnvelope;

/// Policy reference embedded in the assess output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRef {
    pub id: String,
    pub version: u32,
    pub sha256: String,
}

/// A risk factor identified during assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskFactor {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Full assess.v0 output for a successful decision (no refusal).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssessOutput {
    pub tool: String,
    pub version: String,
    pub decision_band: String,
    pub policy: PolicyRef,
    pub matched_rule: String,
    pub required_tools: Vec<String>,
    pub observed_tools: Vec<String>,
    pub risk_factors: Vec<RiskFactor>,
    pub epistemic_basis: Vec<ArtifactBasisEntry>,
    /// Always `null` for successful decisions. Present for schema completeness.
    pub refusal: Option<Value>,
}

impl AssessOutput {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("AssessOutput is always serializable")
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("AssessOutput is always serializable")
    }
}

/// Unified output type for assess: either a decision or a refusal.
#[derive(Debug, Clone, PartialEq)]
pub enum AssessResult {
    Decision(AssessOutput),
    Refusal(RefusalEnvelope),
}

/// Build a full `AssessOutput` from a decision, the artifact bundle, and the loaded policy.
pub fn build_output(
    decision: &Decision,
    bundle: &ArtifactBundle,
    loaded: &LoadedPolicy,
) -> AssessOutput {
    let required_tools = loaded.policy.requires.clone();
    let risk_factors = decision
        .risk_code
        .as_ref()
        .map(|code| {
            vec![RiskFactor {
                code: code.clone(),
                source_tool: None,
                detail: None,
            }]
        })
        .unwrap_or_default();

    let observed_tools = ordered_observed_tools(bundle, &required_tools);
    let epistemic_basis = ordered_basis(bundle, &required_tools);

    AssessOutput {
        tool: "assess".to_owned(),
        version: "assess.v0".to_owned(),
        decision_band: decision.decision_band.as_str().to_owned(),
        policy: PolicyRef {
            id: loaded.policy.policy_id.clone(),
            version: loaded.policy.policy_version,
            sha256: loaded.sha256.clone(),
        },
        matched_rule: decision.matched_rule.clone(),
        required_tools,
        observed_tools,
        risk_factors,
        epistemic_basis,
        refusal: None,
    }
}

/// Render an `AssessResult` for output.
pub fn render(result: &AssessResult, json_output: bool) -> String {
    match result {
        AssessResult::Decision(output) => {
            if json_output {
                json::render_output(output)
            } else {
                human::render_output(output)
            }
        }
        AssessResult::Refusal(envelope) => {
            if json_output {
                json::render_refusal(envelope)
            } else {
                human::render_refusal(envelope)
            }
        }
    }
}

fn ordered_observed_tools(bundle: &ArtifactBundle, required_tools: &[String]) -> Vec<String> {
    let observed = bundle.observed_tools();
    let observed_index: BTreeMap<&str, usize> = observed
        .iter()
        .enumerate()
        .map(|(index, tool)| (tool.as_str(), index))
        .collect();

    let mut ordered = Vec::new();
    for tool in required_tools {
        if observed_index.contains_key(tool.as_str()) {
            ordered.push(tool.clone());
        }
    }

    for tool in observed {
        if !required_tools.contains(&tool) {
            ordered.push(tool);
        }
    }

    ordered
}

fn ordered_basis(bundle: &ArtifactBundle, required_tools: &[String]) -> Vec<ArtifactBasisEntry> {
    let required_rank: BTreeMap<&str, usize> = required_tools
        .iter()
        .enumerate()
        .map(|(index, tool)| (tool.as_str(), index))
        .collect();

    let mut basis = bundle.basis().to_vec();
    basis.sort_by(|left, right| {
        let left_rank = required_rank
            .get(left.tool.as_str())
            .copied()
            .unwrap_or(usize::MAX);
        let right_rank = required_rank
            .get(right.tool.as_str())
            .copied()
            .unwrap_or(usize::MAX);

        left_rank
            .cmp(&right_rank)
            .then_with(|| left.tool.cmp(&right.tool))
            .then_with(|| left.artifact.cmp(&right.artifact))
    });
    basis
}
