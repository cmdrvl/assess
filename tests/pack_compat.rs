mod support;

use assess::cli::Cli;
use assess::execute;
use clap::Parser;
use serde_json::{Value, json};

fn detect_pack_member_type(content: &[u8]) -> (&'static str, Option<String>) {
    let Ok(text) = std::str::from_utf8(content) else {
        return ("other", None);
    };

    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return ("other", None);
    };

    let Some(version) = value.get("version").and_then(Value::as_str) else {
        return ("other", None);
    };

    match version {
        "assess.v0" => ("artifact", Some(version.to_owned())),
        _ => ("other", None),
    }
}

#[test]
fn successful_json_output_is_pack_compatible_artifact() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("pack-compat-success")?;

    let shape = workspace.write_json(
        "shape.json",
        &json!({
            "tool": "shape",
            "version": "shape.report.v1",
            "outcome": "COMPATIBLE"
        }),
    )?;
    let rvl = workspace.write_json(
        "rvl.json",
        &json!({
            "tool": "rvl",
            "version": "rvl.report.v1",
            "outcome": "REAL_CHANGE"
        }),
    )?;
    let verify = workspace.write_json(
        "verify.json",
        &json!({
            "tool": "verify",
            "version": "verify.report.v1",
            "outcome": "PASS"
        }),
    )?;

    let execution = execute(Cli::parse_from([
        "assess",
        shape.to_str().expect("shape path should be utf-8"),
        rvl.to_str().expect("rvl path should be utf-8"),
        verify.to_str().expect("verify path should be utf-8"),
        "--policy-id",
        "loan_tape.monthly.v1",
        "--json",
        "--no-witness",
    ]))?;

    assert_eq!(execution.exit_code, 0);

    let (member_type, artifact_version) = detect_pack_member_type(execution.stdout.as_bytes());
    assert_eq!(member_type, "artifact");
    assert_eq!(artifact_version.as_deref(), Some("assess.v0"));

    let parsed: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(parsed["tool"], "assess");
    assert_eq!(parsed["version"], "assess.v0");

    Ok(())
}

#[test]
fn refusal_json_output_still_matches_pack_artifact_detection()
-> Result<(), Box<dyn std::error::Error>> {
    let execution = execute(Cli::parse_from([
        "assess",
        "fixtures/artifacts/shape_compatible.json",
        "--policy",
        "fixtures/policies/loan_tape_monthly_v1.yaml",
        "--policy-id",
        "loan_tape.monthly.v1",
    ]))?;

    assert_eq!(execution.exit_code, 2);

    let (member_type, artifact_version) = detect_pack_member_type(execution.stdout.as_bytes());
    assert_eq!(member_type, "artifact");
    assert_eq!(artifact_version.as_deref(), Some("assess.v0"));

    let parsed: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(parsed["tool"], "assess");
    assert_eq!(parsed["version"], "assess.v0");
    assert_eq!(parsed["refusal"]["code"], "E_AMBIGUOUS_POLICY");

    Ok(())
}
