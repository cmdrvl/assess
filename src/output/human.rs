use std::fmt::Write;

use super::{AssessOutput, AssessResult, RenderContext};
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

pub fn render_summary(result: &AssessResult, context: RenderContext) -> String {
    let row = SummaryRow::from_result(result, context);
    format!(
        "tool={} version={} outcome={} decision={} matched_rule={} risk_code={} required_tools={} observed_tools={} witness={} refusal_code={}",
        row.tool,
        row.version,
        row.outcome,
        row.decision,
        row.matched_rule,
        row.risk_code,
        row.required_tools,
        row.observed_tools,
        row.witness,
        row.refusal_code
    )
}

pub fn render_summary_tsv(result: &AssessResult, context: RenderContext) -> String {
    let row = SummaryRow::from_result(result, context);
    let header = [
        "tool",
        "version",
        "outcome",
        "decision",
        "matched_rule",
        "risk_code",
        "required_tools",
        "observed_tools",
        "witness",
        "refusal_code",
    ]
    .join("\t");
    let values = [
        row.tool,
        row.version,
        row.outcome,
        row.decision,
        row.matched_rule,
        row.risk_code,
        row.required_tools,
        row.observed_tools,
        row.witness,
        row.refusal_code,
    ]
    .map(sanitize_tsv)
    .join("\t");

    format!("{header}\n{values}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SummaryRow {
    tool: String,
    version: String,
    outcome: String,
    decision: String,
    matched_rule: String,
    risk_code: String,
    required_tools: String,
    observed_tools: String,
    witness: String,
    refusal_code: String,
}

impl SummaryRow {
    fn from_result(result: &AssessResult, context: RenderContext) -> Self {
        match result {
            AssessResult::Decision(output) => Self {
                tool: output.tool.clone(),
                version: output.version.clone(),
                outcome: "DECISION".to_owned(),
                decision: output.decision_band.clone(),
                matched_rule: output.matched_rule.clone(),
                risk_code: joined_or_placeholder(
                    output
                        .risk_factors
                        .iter()
                        .map(|factor| factor.code.as_str()),
                ),
                required_tools: joined_or_placeholder(
                    output.required_tools.iter().map(String::as_str),
                ),
                observed_tools: joined_or_placeholder(
                    output.observed_tools.iter().map(String::as_str),
                ),
                witness: context.witness_status.as_str().to_owned(),
                refusal_code: placeholder(""),
            },
            AssessResult::Refusal(envelope) => Self {
                tool: envelope.tool.clone(),
                version: envelope.version.clone(),
                outcome: "REFUSAL".to_owned(),
                decision: placeholder(""),
                matched_rule: placeholder(""),
                risk_code: placeholder(""),
                required_tools: placeholder(""),
                observed_tools: placeholder(""),
                witness: context.witness_status.as_str().to_owned(),
                refusal_code: envelope.refusal.code.as_str().to_owned(),
            },
        }
    }
}

fn joined_or_placeholder<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let joined = values.collect::<Vec<_>>().join(",");
    placeholder(&joined)
}

fn placeholder(value: &str) -> String {
    if value.is_empty() {
        "-".to_owned()
    } else {
        value.to_owned()
    }
}

fn sanitize_tsv(value: String) -> String {
    value.replace(['\t', '\n', '\r'], " ")
}
