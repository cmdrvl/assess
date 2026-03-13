use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use assess::cli::Cli;
use assess::{OPERATOR_JSON, execute};
use clap::Parser;
use serde_json::{Value, json};

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn unique_dir(prefix: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("assess-e2e-{}-{prefix}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_artifact(dir: &std::path::Path, name: &str, json_val: &Value) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_string_pretty(json_val).unwrap()).unwrap();
    path
}

fn write_policy(dir: &std::path::Path, name: &str, yaml: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, yaml).unwrap();
    path
}

// ---------------------------------------------------------------------------
// Meta / describe / schema / version
// ---------------------------------------------------------------------------

#[test]
fn describe_surface_works_before_runtime_semantics() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse_from(["assess", "--describe"]);
    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 0);
    let expected = if OPERATOR_JSON.ends_with('\n') {
        OPERATOR_JSON.to_owned()
    } else {
        format!("{OPERATOR_JSON}\n")
    };
    assert_eq!(execution.stdout, expected);
    Ok(())
}

#[test]
fn schema_output_is_valid_json() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse_from(["assess", "--schema"]);
    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 0);
    let _: Value = serde_json::from_str(execution.stdout.trim())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Full pipeline: PROCEED (exit 0)
// ---------------------------------------------------------------------------

#[test]
fn full_pipeline_proceed_json() -> Result<(), Box<dyn std::error::Error>> {
    let dir = unique_dir("proceed-json");

    let shape = write_artifact(
        &dir,
        "shape.json",
        &json!({"tool": "shape", "version": "shape.report.v1", "outcome": "COMPATIBLE"}),
    );
    let rvl = write_artifact(
        &dir,
        "rvl.json",
        &json!({"tool": "rvl", "version": "rvl.report.v1", "outcome": "REAL_CHANGE"}),
    );
    let verify = write_artifact(
        &dir,
        "verify.json",
        &json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
    );

    let cli = Cli::parse_from([
        "assess",
        shape.to_str().unwrap(),
        rvl.to_str().unwrap(),
        verify.to_str().unwrap(),
        "--policy-id",
        "loan_tape.monthly.v1",
        "--json",
        "--no-witness",
    ]);

    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 0);

    let parsed: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(parsed["tool"], "assess");
    assert_eq!(parsed["version"], "assess.v0");
    assert_eq!(parsed["decision_band"], "PROCEED");
    assert_eq!(parsed["matched_rule"], "clean_reconciliation");
    assert!(parsed["policy"]["sha256"].is_string());
    assert!(parsed["refusal"].is_null());

    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

#[test]
fn full_pipeline_proceed_human() -> Result<(), Box<dyn std::error::Error>> {
    let dir = unique_dir("proceed-human");

    let shape = write_artifact(
        &dir,
        "shape.json",
        &json!({"tool": "shape", "version": "shape.report.v1", "outcome": "COMPATIBLE"}),
    );
    let rvl = write_artifact(
        &dir,
        "rvl.json",
        &json!({"tool": "rvl", "version": "rvl.report.v1", "outcome": "REAL_CHANGE"}),
    );
    let verify = write_artifact(
        &dir,
        "verify.json",
        &json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
    );

    let cli = Cli::parse_from([
        "assess",
        shape.to_str().unwrap(),
        rvl.to_str().unwrap(),
        verify.to_str().unwrap(),
        "--policy-id",
        "loan_tape.monthly.v1",
        "--no-witness",
    ]);

    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 0);
    assert!(execution.stdout.contains("PROCEED"));
    assert!(execution.stdout.contains("clean_reconciliation"));

    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

#[test]
fn full_pipeline_proceed_summary() -> Result<(), Box<dyn std::error::Error>> {
    let dir = unique_dir("proceed-summary");

    let shape = write_artifact(
        &dir,
        "shape.json",
        &json!({"tool": "shape", "version": "shape.report.v1", "outcome": "COMPATIBLE"}),
    );
    let rvl = write_artifact(
        &dir,
        "rvl.json",
        &json!({"tool": "rvl", "version": "rvl.report.v1", "outcome": "REAL_CHANGE"}),
    );
    let verify = write_artifact(
        &dir,
        "verify.json",
        &json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
    );

    let cli = Cli::parse_from([
        "assess",
        shape.to_str().unwrap(),
        rvl.to_str().unwrap(),
        verify.to_str().unwrap(),
        "--policy-id",
        "loan_tape.monthly.v1",
        "--render",
        "summary",
        "--no-witness",
    ]);

    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 0);
    assert_eq!(
        execution.stdout.trim(),
        "tool=assess version=assess.v0 outcome=DECISION decision=PROCEED matched_rule=clean_reconciliation risk_code=- required_tools=shape,rvl,verify observed_tools=shape,rvl,verify witness=disabled refusal_code=-"
    );

    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

// ---------------------------------------------------------------------------
// BLOCK path (exit 2)
// ---------------------------------------------------------------------------

#[test]
fn default_block_exit_code_2() -> Result<(), Box<dyn std::error::Error>> {
    let dir = unique_dir("block");

    let verify = write_artifact(
        &dir,
        "verify.json",
        &json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
    );

    let cli = Cli::parse_from([
        "assess",
        verify.to_str().unwrap(),
        "--policy-id",
        "default.v0",
        "--json",
        "--no-witness",
    ]);

    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 2);

    let parsed: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(parsed["decision_band"], "BLOCK");
    assert_eq!(parsed["matched_rule"], "default_block");

    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

// ---------------------------------------------------------------------------
// PROCEED_WITH_RISK path (exit 1)
// ---------------------------------------------------------------------------

#[test]
fn proceed_with_risk_exit_code_1() -> Result<(), Box<dyn std::error::Error>> {
    let dir = unique_dir("risk");

    let shape = write_artifact(
        &dir,
        "shape.json",
        &json!({
            "tool": "shape",
            "version": "shape.report.v1",
            "outcome": "INCOMPATIBLE",
            "policy_signals": {"compatibility_band": "PARTIAL"}
        }),
    );
    let rvl = write_artifact(
        &dir,
        "rvl.json",
        &json!({"tool": "rvl", "version": "rvl.report.v1", "outcome": "REAL_CHANGE"}),
    );
    let verify = write_artifact(
        &dir,
        "verify.json",
        &json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
    );

    let cli = Cli::parse_from([
        "assess",
        shape.to_str().unwrap(),
        rvl.to_str().unwrap(),
        verify.to_str().unwrap(),
        "--policy-id",
        "loan_tape.monthly.v1",
        "--json",
        "--no-witness",
    ]);

    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 1);

    let parsed: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(parsed["decision_band"], "PROCEED_WITH_RISK");
    assert_eq!(parsed["matched_rule"], "partial_overlap_acceptable");

    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

// ---------------------------------------------------------------------------
// ESCALATE path (exit 1)
// ---------------------------------------------------------------------------

#[test]
fn escalate_exit_code_1() -> Result<(), Box<dyn std::error::Error>> {
    let dir = unique_dir("escalate");

    let shape = write_artifact(
        &dir,
        "shape.json",
        &json!({"tool": "shape", "version": "shape.report.v1", "outcome": "COMPATIBLE"}),
    );
    let rvl = write_artifact(
        &dir,
        "rvl.json",
        &json!({
            "tool": "rvl",
            "version": "rvl.report.v1",
            "outcome": null,
            "refusal": {"code": "E_DIFFUSE", "message": "diffuse change detected"}
        }),
    );
    let verify = write_artifact(
        &dir,
        "verify.json",
        &json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
    );

    let cli = Cli::parse_from([
        "assess",
        shape.to_str().unwrap(),
        rvl.to_str().unwrap(),
        verify.to_str().unwrap(),
        "--policy-id",
        "loan_tape.monthly.v1",
        "--json",
        "--no-witness",
    ]);

    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 1);

    let parsed: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(parsed["decision_band"], "ESCALATE");
    assert_eq!(parsed["matched_rule"], "diffuse_requires_review");

    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

// ---------------------------------------------------------------------------
// Refusal paths (exit 2, refusal in output)
// ---------------------------------------------------------------------------

#[test]
fn incomplete_basis_refusal_json() -> Result<(), Box<dyn std::error::Error>> {
    let dir = unique_dir("incomplete");

    // Only provide verify, but loan_tape requires shape + rvl + verify
    let verify = write_artifact(
        &dir,
        "verify.json",
        &json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
    );

    let cli = Cli::parse_from([
        "assess",
        verify.to_str().unwrap(),
        "--policy-id",
        "loan_tape.monthly.v1",
        "--json",
        "--no-witness",
    ]);

    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 2);

    let parsed: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(parsed["refusal"]["code"], "E_INCOMPLETE_BASIS");
    assert!(parsed["decision_band"].is_null());

    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

#[test]
fn unknown_policy_refusal() -> Result<(), Box<dyn std::error::Error>> {
    let dir = unique_dir("unknown");

    let verify = write_artifact(
        &dir,
        "verify.json",
        &json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
    );

    let cli = Cli::parse_from([
        "assess",
        verify.to_str().unwrap(),
        "--policy-id",
        "nonexistent.policy.v99",
        "--json",
        "--no-witness",
    ]);

    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 2);

    let parsed: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(parsed["refusal"]["code"], "E_UNKNOWN_POLICY");

    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

#[test]
fn bad_artifact_refusal() -> Result<(), Box<dyn std::error::Error>> {
    let dir = unique_dir("badart");

    let bad = dir.join("garbage.json");
    std::fs::write(&bad, "this is not json").unwrap();

    let cli = Cli::parse_from([
        "assess",
        bad.to_str().unwrap(),
        "--policy-id",
        "default.v0",
        "--json",
        "--no-witness",
    ]);

    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 2);

    let parsed: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(parsed["refusal"]["code"], "E_BAD_ARTIFACT");

    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

#[test]
fn ambiguous_policy_refusal() {
    let cli = Cli::parse_from([
        "assess",
        "/dev/null",
        "--policy",
        "some.yaml",
        "--policy-id",
        "some.id",
        "--json",
        "--no-witness",
    ]);

    let execution = execute(cli).unwrap();
    assert_eq!(execution.exit_code, 2);

    let parsed: Value = serde_json::from_str(execution.stdout.trim()).unwrap();
    assert_eq!(parsed["refusal"]["code"], "E_AMBIGUOUS_POLICY");
}

#[test]
fn ambiguous_policy_refusal_respects_summary_render_mode() {
    let cli = Cli::parse_from([
        "assess",
        "/dev/null",
        "--policy",
        "some.yaml",
        "--policy-id",
        "some.id",
        "--render",
        "summary-tsv",
        "--no-witness",
    ]);

    let execution = execute(cli).unwrap();
    assert_eq!(execution.exit_code, 2);
    assert_eq!(
        execution.stdout.trim(),
        "tool\tversion\toutcome\tdecision\tmatched_rule\trisk_code\trequired_tools\tobserved_tools\twitness\trefusal_code\nassess\tassess.v0\tREFUSAL\t-\t-\t-\t-\t-\tdisabled\tE_AMBIGUOUS_POLICY"
    );
}

// ---------------------------------------------------------------------------
// Policy loaded from file path (--policy)
// ---------------------------------------------------------------------------

#[test]
fn policy_from_file_path() -> Result<(), Box<dyn std::error::Error>> {
    let dir = unique_dir("filepath");

    let policy_path = write_policy(
        &dir,
        "custom.yaml",
        "schema_version: 1\npolicy_id: custom.v0\npolicy_version: 1\nrules:\n  - name: always_proceed\n    default: true\n    then:\n      decision_band: PROCEED\n",
    );

    let verify = write_artifact(
        &dir,
        "verify.json",
        &json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
    );

    let cli = Cli::parse_from([
        "assess",
        verify.to_str().unwrap(),
        "--policy",
        policy_path.to_str().unwrap(),
        "--json",
        "--no-witness",
    ]);

    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 0);

    let parsed: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(parsed["decision_band"], "PROCEED");
    assert_eq!(parsed["policy"]["id"], "custom.v0");

    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

// ---------------------------------------------------------------------------
// Epistemic basis populated correctly
// ---------------------------------------------------------------------------

#[test]
fn epistemic_basis_includes_all_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    let dir = unique_dir("basis");

    let shape = write_artifact(
        &dir,
        "shape.json",
        &json!({"tool": "shape", "version": "shape.report.v1", "outcome": "COMPATIBLE"}),
    );
    let rvl = write_artifact(
        &dir,
        "rvl.json",
        &json!({"tool": "rvl", "version": "rvl.report.v1", "outcome": "REAL_CHANGE"}),
    );
    let verify = write_artifact(
        &dir,
        "verify.json",
        &json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
    );

    let cli = Cli::parse_from([
        "assess",
        shape.to_str().unwrap(),
        rvl.to_str().unwrap(),
        verify.to_str().unwrap(),
        "--policy-id",
        "loan_tape.monthly.v1",
        "--json",
        "--no-witness",
    ]);

    let execution = execute(cli)?;
    let parsed: Value = serde_json::from_str(execution.stdout.trim())?;
    let basis = parsed["epistemic_basis"].as_array().unwrap();

    assert_eq!(basis.len(), 3);
    let tools: Vec<&str> = basis.iter().map(|b| b["tool"].as_str().unwrap()).collect();
    assert!(tools.contains(&"shape"));
    assert!(tools.contains(&"rvl"));
    assert!(tools.contains(&"verify"));

    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}

// ---------------------------------------------------------------------------
// JSON output is deterministic (I14)
// ---------------------------------------------------------------------------

#[test]
fn json_output_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    let dir = unique_dir("determinism");

    let shape = write_artifact(
        &dir,
        "shape.json",
        &json!({"tool": "shape", "version": "shape.report.v1", "outcome": "COMPATIBLE"}),
    );
    let rvl = write_artifact(
        &dir,
        "rvl.json",
        &json!({"tool": "rvl", "version": "rvl.report.v1", "outcome": "REAL_CHANGE"}),
    );
    let verify = write_artifact(
        &dir,
        "verify.json",
        &json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
    );

    let run = |artifacts: &[&str]| -> String {
        let cli = Cli::parse_from(
            std::iter::once("assess")
                .chain(artifacts.iter().copied())
                .chain([
                    "--policy-id",
                    "loan_tape.monthly.v1",
                    "--json",
                    "--no-witness",
                ]),
        );
        execute(cli).unwrap().stdout
    };

    let a = run(&[
        shape.to_str().unwrap(),
        rvl.to_str().unwrap(),
        verify.to_str().unwrap(),
    ]);
    let b = run(&[
        shape.to_str().unwrap(),
        rvl.to_str().unwrap(),
        verify.to_str().unwrap(),
    ]);
    assert_eq!(a, b, "JSON output must be deterministic across runs");

    std::fs::remove_dir_all(&dir).ok();
    Ok(())
}
