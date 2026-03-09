use std::fmt::Write;

use super::AssessOutput;
use crate::refusal::RefusalEnvelope;

/// Render a successful assess decision in human-readable form.
pub fn render_output(output: &AssessOutput) -> String {
    let mut buf = String::new();
    writeln!(buf, "ASSESS {}", output.decision_band).unwrap();
    writeln!(
        buf,
        "policy: {} v{} [{}]",
        output.policy.id, output.policy.version, output.policy.sha256
    )
    .unwrap();
    writeln!(buf, "matched_rule: {}", output.matched_rule).unwrap();

    for factor in &output.risk_factors {
        write!(buf, "risk: {}", factor.code).unwrap();
        if let Some(tool) = &factor.source_tool {
            write!(buf, " (source: {tool})").unwrap();
        }
        buf.push('\n');
    }

    writeln!(buf, "required_tools: {}", output.required_tools.join(", ")).unwrap();
    writeln!(buf, "observed_tools: {}", output.observed_tools.join(", ")).unwrap();

    for entry in &output.epistemic_basis {
        write!(buf, "basis: {} ({})", entry.tool, entry.artifact).unwrap();
        if let Some(outcome) = &entry.outcome {
            write!(buf, " -> {outcome}").unwrap();
        }
        if let Some(refusal) = &entry.refusal {
            write!(buf, " -> refusal {}", refusal.code).unwrap();
        }
        buf.push('\n');
    }

    buf
}

/// Render a refusal envelope in human-readable form.
pub fn render_refusal(envelope: &RefusalEnvelope) -> String {
    let mut buf = String::new();
    writeln!(buf, "ASSESS REFUSAL").unwrap();
    writeln!(buf, "code: {}", envelope.refusal.code.as_str()).unwrap();
    writeln!(buf, "message: {}", envelope.refusal.message).unwrap();
    writeln!(buf, "next: {}", envelope.refusal.next_command).unwrap();
    buf
}
