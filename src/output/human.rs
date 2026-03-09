use crate::evaluate::Decision;

pub fn render(decision: &Decision) -> String {
    let mut output = format!("decision_band: {}", decision.decision_band.as_str());
    if let Some(rule) = &decision.matched_rule {
        output.push_str(&format!("\nmatched_rule: {rule}"));
    }
    if let Some(risk_code) = &decision.risk_code {
        output.push_str(&format!("\nrisk_code: {risk_code}"));
    }
    output.push('\n');
    output
}
