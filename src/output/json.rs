use super::AssessOutput;
use crate::refusal::RefusalEnvelope;

/// Render a successful assess decision as JSON.
pub fn render_output(output: &AssessOutput) -> String {
    output.to_json_pretty()
}

/// Render a refusal envelope as JSON.
pub fn render_refusal(envelope: &RefusalEnvelope) -> String {
    envelope.to_json_pretty()
}
