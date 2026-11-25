//! Exit-code contract (spine-rules docs/exit-codes.md), exercised through the
//! real binary so `main.rs` exit mapping is covered:
//! PROCEED 0, PROCEED_WITH_RISK 0, ESCALATE 1, REFUSAL/CLI error 2, BLOCK 3.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("assess-exit-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn artifact(dir: &Path, name: &str, v: Value) -> String {
    let p = dir.join(name);
    std::fs::write(&p, serde_json::to_string(&v).unwrap()).unwrap();
    p.to_str().unwrap().to_owned()
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn assess(args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_assess"))
        .args(args)
        .output()
        .expect("run assess");
    Run {
        code: out.status.code().expect("exit code, not a signal"),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn shape(outcome: &str, band: Option<&str>) -> Value {
    let mut v = json!({"tool": "shape", "version": "shape.report.v1", "outcome": outcome});
    if let Some(b) = band {
        v["policy_signals"] = json!({"compatibility_band": b});
    }
    v
}

fn rvl_ok() -> Value {
    json!({"tool": "rvl", "version": "rvl.report.v1", "outcome": "REAL_CHANGE"})
}

fn verify_pass() -> Value {
    json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"})
}

fn run_loan_tape(d: &Path, shape_v: Value, rvl_v: Value) -> Run {
    let s = artifact(d, "shape.json", shape_v);
    let r = artifact(d, "rvl.json", rvl_v);
    let v = artifact(d, "verify.json", verify_pass());
    assess(&[
        &s,
        &r,
        &v,
        "--policy-id",
        "loan_tape.monthly.v1",
        "--json",
        "--no-witness",
    ])
}

fn band(run: &Run) -> String {
    let v: Value = serde_json::from_str(run.stdout.trim()).expect("json stdout");
    v["decision_band"].as_str().unwrap_or("<none>").to_owned()
}

#[test]
fn proceed_exits_0() {
    let r = run_loan_tape(&dir("proceed"), shape("COMPATIBLE", None), rvl_ok());
    assert_eq!((band(&r).as_str(), r.code), ("PROCEED", 0), "{}", r.stderr);
}

#[test]
fn proceed_with_risk_exits_0() {
    let r = run_loan_tape(
        &dir("risk"),
        shape("INCOMPATIBLE", Some("PARTIAL")),
        rvl_ok(),
    );
    assert_eq!(
        (band(&r).as_str(), r.code),
        ("PROCEED_WITH_RISK", 0),
        "{}",
        r.stderr
    );
}

#[test]
fn escalate_exits_1() {
    let rvl = json!({
        "tool": "rvl", "version": "rvl.report.v1", "outcome": null,
        "refusal": {"code": "E_DIFFUSE", "message": "diffuse change detected"}
    });
    let r = run_loan_tape(&dir("escalate"), shape("COMPATIBLE", None), rvl);
    assert_eq!((band(&r).as_str(), r.code), ("ESCALATE", 1), "{}", r.stderr);
}

#[test]
fn block_exits_3_and_is_a_decision_not_a_refusal() {
    let d = dir("block");
    let v = artifact(&d, "verify.json", verify_pass());
    let r = assess(&[&v, "--policy-id", "default.v0", "--json", "--no-witness"]);
    assert_eq!(r.code, 3, "stderr: {}", r.stderr);
    let parsed: Value = serde_json::from_str(r.stdout.trim()).unwrap();
    assert_eq!(parsed["decision_band"], "BLOCK");
    assert!(
        parsed["refusal"].is_null() && parsed["matched_rule"] == "default_block",
        "BLOCK must be a decision artifact, not a refusal envelope: {parsed}"
    );
}

#[test]
fn refusal_exits_2_and_differs_from_block() {
    let d = dir("refusal");
    let v = artifact(&d, "verify.json", verify_pass());
    let r = assess(&[
        &v,
        "--policy-id",
        "no.such.policy.v9",
        "--json",
        "--no-witness",
    ]);
    assert_eq!(r.code, 2, "stdout: {}", r.stdout);
    let parsed: Value = serde_json::from_str(r.stdout.trim()).unwrap();
    assert!(
        parsed["refusal"]["code"]
            .as_str()
            .is_some_and(|c| c.starts_with("E_")),
        "{parsed}"
    );
    assert!(parsed["decision_band"].is_null(), "{parsed}");
}

#[test]
fn cli_syntax_error_exits_2() {
    let r = assess(&["--definitely-not-a-flag"]);
    assert_eq!(r.code, 2, "stdout: {} stderr: {}", r.stdout, r.stderr);
    assert!(
        r.stdout.trim().is_empty(),
        "usage errors go to stderr: {}",
        r.stdout
    );
}

#[test]
fn missing_artifact_file_is_a_refusal_not_a_block() {
    let r = assess(&[
        "/definitely/not/here.json",
        "--policy-id",
        "default.v0",
        "--json",
        "--no-witness",
    ]);
    assert_eq!(r.code, 2, "stdout: {}", r.stdout);
}
