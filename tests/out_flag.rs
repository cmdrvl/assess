//! `--out <PATH>` (alias `--output`): the report goes to the file, stdout stays
//! empty, and the exit code is exactly what it would have been.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("assess-out-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write(dir: &Path, name: &str, v: Value) -> String {
    let p = dir.join(name);
    std::fs::write(&p, v.to_string()).unwrap();
    p.to_str().unwrap().to_owned()
}

fn assess(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_assess"))
        .args(args)
        .output()
        .expect("run assess")
}

fn block_args(dir: &Path) -> Vec<String> {
    let verify = write(
        dir,
        "verify.json",
        json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
    );
    // default.v0 with only a verify artifact -> BLOCK (exit 3).
    vec![
        verify,
        "--policy-id".into(),
        "default.v0".into(),
        "--json".into(),
        "--no-witness".into(),
    ]
}

fn run(args: &[String], extra: &[&str]) -> Output {
    let mut all: Vec<&str> = args.iter().map(String::as_str).collect();
    all.extend_from_slice(extra);
    assess(&all)
}

#[test]
fn out_writes_report_to_file_and_keeps_exit_code() {
    let d = scratch("file");
    let args = block_args(&d);

    let to_stdout = run(&args, &[]);
    assert_eq!(to_stdout.status.code(), Some(3));

    let target = d.join("decision.json");
    let to_file = run(&args, &["--out", target.to_str().unwrap()]);
    assert_eq!(to_file.status.code(), Some(3), "exit code must not change");
    assert!(
        to_file.stdout.is_empty(),
        "stdout must stay empty with --out"
    );
    assert_eq!(std::fs::read(&target).unwrap(), to_stdout.stdout);
    let parsed: Value = serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
    assert_eq!(parsed["decision_band"], "BLOCK");
}

#[test]
fn output_is_an_alias_of_out() {
    let d = scratch("alias");
    let args = block_args(&d);
    let target = d.join("via-output.json");
    let r = run(&args, &["--output", target.to_str().unwrap()]);
    assert_eq!(r.status.code(), Some(3));
    assert!(r.stdout.is_empty());
    assert!(target.exists());
}

#[test]
fn out_also_captures_refusal_envelopes() {
    let d = scratch("refusal");
    let target = d.join("refusal.json");
    let r = assess(&[
        "/definitely/not/here.json",
        "--policy-id",
        "default.v0",
        "--json",
        "--no-witness",
        "--out",
        target.to_str().unwrap(),
    ]);
    assert_eq!(r.status.code(), Some(2));
    assert!(r.stdout.is_empty());
    let parsed: Value = serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
    assert!(parsed["refusal"]["code"].is_string(), "{parsed}");
}

#[test]
fn unwritable_out_path_is_exit_2_with_a_stderr_message() {
    let d = scratch("unwritable");
    let args = block_args(&d);
    let r = run(&args, &["--out", "/definitely/not/a/dir/report.json"]);
    assert_eq!(
        r.status.code(),
        Some(2),
        "a failed write is an error (2), never the verdict's code"
    );
    assert!(r.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(
        stderr.contains("--out") && stderr.contains("/definitely/not/a/dir"),
        "{stderr}"
    );
}
