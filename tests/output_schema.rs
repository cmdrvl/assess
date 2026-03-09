mod support;

use std::path::{Path, PathBuf};

use assess::bundle;
use assess::evaluate;
use assess::output::{self, AssessOutput, AssessResult};
use assess::policy;
use assess::refusal::{RefusalCode, RefusalEnvelope};
use assess::{ASSESS_SCHEMA_JSON, POLICY_SCHEMA_JSON};
use serde_json::Value;

fn schema() -> Value {
    serde_json::from_str(ASSESS_SCHEMA_JSON).expect("assess schema must parse")
}

fn validate(instance: &Value) -> Result<(), String> {
    let validator = jsonschema::validator_for(&schema()).expect("schema should compile");
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn load_policy() -> policy::LoadedPolicy {
    policy::load_and_validate(Path::new("fixtures/policies/loan_tape_monthly_v1.yaml"))
        .expect("policy fixture should load")
}

fn build_output(paths: &[&str]) -> AssessOutput {
    let bundle_paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    let bundle = bundle::load(&bundle_paths).expect("bundle should load");
    let loaded = load_policy();
    let decision = evaluate::evaluate(&loaded.policy, &bundle).expect("evaluation should succeed");
    output::build_output(&decision, &bundle, &loaded)
}

fn golden(name: &str) -> String {
    std::fs::read_to_string(support::golden_path(name))
        .expect("golden fixture should read")
        .trim_end_matches('\n')
        .to_owned()
}

#[test]
fn embedded_schemas_are_present() {
    assert!(ASSESS_SCHEMA_JSON.contains("\"title\": \"assess.v0\""));
    assert!(POLICY_SCHEMA_JSON.contains("\"title\": \"policy.v0\""));
}

#[test]
fn assess_output_schema_compiles() {
    let _ = jsonschema::validator_for(&schema()).expect("schema should compile");
}

#[test]
fn proceed_output_matches_golden_and_schema() -> Result<(), Box<dyn std::error::Error>> {
    let output = build_output(&[
        "fixtures/artifacts/shape_compatible.json",
        "fixtures/artifacts/rvl_real_change.json",
        "fixtures/artifacts/verify_pass.json",
    ]);
    let rendered = output::render(&AssessResult::Decision(output.clone()), true);
    let parsed: Value = serde_json::from_str(&rendered)?;
    validate(&parsed).map_err(std::io::Error::other)?;

    let expected = golden("proceed.json");
    assert_eq!(rendered, expected);
    assert_eq!(output.decision_band, "PROCEED");
    assert_eq!(output.matched_rule, "clean_reconciliation");
    assert!(output.risk_factors.is_empty());
    Ok(())
}

#[test]
fn proceed_with_risk_output_matches_golden_and_schema() -> Result<(), Box<dyn std::error::Error>> {
    let output = build_output(&[
        "fixtures/artifacts/shape_incompatible_partial.json",
        "fixtures/artifacts/rvl_no_real_change.json",
        "fixtures/artifacts/verify_pass.json",
    ]);
    let rendered = output::render(&AssessResult::Decision(output.clone()), true);
    let parsed: Value = serde_json::from_str(&rendered)?;
    validate(&parsed).map_err(std::io::Error::other)?;

    let expected = golden("proceed_with_risk.json");
    assert_eq!(rendered, expected);
    assert_eq!(output.decision_band, "PROCEED_WITH_RISK");
    assert_eq!(output.matched_rule, "partial_overlap_acceptable");
    assert_eq!(output.risk_factors[0].code, "PARTIAL_SCHEMA_OVERLAP");
    Ok(())
}

#[test]
fn escalate_output_matches_golden_and_schema() -> Result<(), Box<dyn std::error::Error>> {
    let output = build_output(&[
        "fixtures/artifacts/shape_compatible.json",
        "fixtures/artifacts/rvl_refusal_diffuse.json",
        "fixtures/artifacts/verify_pass.json",
    ]);
    let rendered = output::render(&AssessResult::Decision(output.clone()), true);
    let parsed: Value = serde_json::from_str(&rendered)?;
    validate(&parsed).map_err(std::io::Error::other)?;

    let expected = golden("escalate.json");
    assert_eq!(rendered, expected);
    assert_eq!(output.decision_band, "ESCALATE");
    assert_eq!(output.matched_rule, "diffuse_requires_review");
    assert_eq!(output.risk_factors[0].code, "DIFFUSE_CHANGE");
    Ok(())
}

#[test]
fn block_output_matches_golden_and_schema() -> Result<(), Box<dyn std::error::Error>> {
    let output = build_output(&[
        "fixtures/artifacts/shape_incompatible_partial.json",
        "fixtures/artifacts/rvl_refusal_missingness_tolerable.json",
        "fixtures/artifacts/verify_pass.json",
    ]);
    let rendered = output::render(&AssessResult::Decision(output.clone()), true);
    let parsed: Value = serde_json::from_str(&rendered)?;
    validate(&parsed).map_err(std::io::Error::other)?;

    let expected = golden("block.json");
    assert_eq!(rendered, expected);
    assert_eq!(output.decision_band, "BLOCK");
    assert_eq!(output.matched_rule, "default_block");
    assert_eq!(output.risk_factors[0].code, "UNHANDLED_CONDITION");
    Ok(())
}

#[test]
fn human_output_shows_band_risk_and_basis_summary() {
    let output = build_output(&[
        "fixtures/artifacts/shape_compatible.json",
        "fixtures/artifacts/rvl_refusal_diffuse.json",
        "fixtures/artifacts/verify_pass.json",
    ]);
    let rendered = output::render(&AssessResult::Decision(output), false);

    support::assert_human_lines(
        &rendered,
        &[
            "ASSESS ESCALATE",
            "matched_rule: diffuse_requires_review",
            "risk: DIFFUSE_CHANGE",
            "basis: shape (fixtures/artifacts/shape_compatible.json) -> COMPATIBLE",
            "basis: rvl (fixtures/artifacts/rvl_refusal_diffuse.json) -> refusal E_DIFFUSE",
            "basis: verify (fixtures/artifacts/verify_pass.json) -> PASS",
        ],
    );
}

#[test]
fn refusal_rendering_stays_structured_in_both_modes() -> Result<(), Box<dyn std::error::Error>> {
    let envelope = RefusalEnvelope::new(RefusalCode::MissingRule, "no rule matched");
    let json_rendered = output::render(&AssessResult::Refusal(envelope.clone()), true);
    let json_value: Value = serde_json::from_str(&json_rendered)?;

    assert_eq!(json_value["refusal"]["code"], "E_MISSING_RULE");
    validate(&json_value).map_err(std::io::Error::other)?;

    let human_rendered = output::render(&AssessResult::Refusal(envelope), false);
    support::assert_human_lines(
        &human_rendered,
        &[
            "ASSESS REFUSAL",
            "code: E_MISSING_RULE",
            "message: no rule matched",
        ],
    );
    Ok(())
}

#[test]
fn json_output_is_byte_stable_across_repeated_renders() {
    let output = build_output(&[
        "fixtures/artifacts/shape_compatible.json",
        "fixtures/artifacts/rvl_real_change.json",
        "fixtures/artifacts/verify_pass.json",
    ]);
    let result = AssessResult::Decision(output);
    let first = output::render(&result, true);
    let second = output::render(&result, true);
    assert_eq!(first, second);
}
