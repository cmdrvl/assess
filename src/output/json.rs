use serde_json::json;

use crate::evaluate::Decision;

pub fn render(decision: &Decision) -> String {
    format!(
        "{}\n",
        json!({
            "tool": "assess",
            "version": "assess.v0",
            "decision_band": decision.decision_band.as_str(),
            "matched_rule": decision.matched_rule,
            "risk_code": decision.risk_code,
        })
    )
}
