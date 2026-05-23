mod support;

use std::{io, path::Path, process::Command};

use serde_json::Value;

fn assess_cmd_in(dir: &Path, witness_path: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_assess"));
    command
        .current_dir(dir)
        .env("EPISTEMIC_WITNESS", witness_path);
    command
}

fn assert_doctor_side_effects_absent(dir: &Path, witness_path: &Path) {
    assert!(!witness_path.exists());
    if let Some(parent) = witness_path.parent() {
        assert!(!parent.exists());
    }
    assert!(!dir.join(".doctor").exists());
    assert!(!dir.join(".epistemic").exists());
    assert!(!dir.join(".cmdrvl").exists());
}

fn missing_json_field(name: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("missing JSON field {name}"),
    )
}

fn assert_all_side_effects_false(side_effects: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let object = side_effects
        .as_object()
        .ok_or_else(|| missing_json_field("side_effects object"))?;
    assert!(!object.is_empty());
    for (name, value) in object {
        assert_eq!(value, false, "side effect {name} should be false");
    }
    Ok(())
}

fn parse_stdout_json(output: &std::process::Output) -> Result<Value, Box<dyn std::error::Error>> {
    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn side_effects(payload: &Value) -> Result<&Value, Box<dyn std::error::Error>> {
    payload
        .get("side_effects")
        .ok_or_else(|| missing_json_field("side_effects").into())
}

#[test]
fn doctor_health_json_is_read_only() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("doctor-health")?;
    let witness_path = workspace.child("witness/assess-witness.jsonl");

    let output = assess_cmd_in(workspace.path(), &witness_path)
        .args(["doctor", "health", "--json"])
        .output()?;
    let payload = parse_stdout_json(&output)?;

    assert_eq!(payload["schema"], "assess.doctor.health.v1");
    assert_eq!(payload["contract"], "cmdrvl.read_only_doctor.v1");
    assert_eq!(payload["tool"], "assess");
    assert_eq!(payload["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["read_only"], true);
    assert_eq!(payload["summary"]["checks_failed"], 0);
    assert_eq!(
        payload["observed_paths"]["witness_ledger"],
        witness_path.display().to_string()
    );
    assert_eq!(payload["side_effects"]["opens_witness_ledger"], false);
    assert_eq!(payload["side_effects"]["appends_witness_ledger"], false);
    assert_eq!(payload["side_effects"]["creates_witness_directory"], false);
    assert_eq!(payload["side_effects"]["writes_migration_logs"], false);
    assert_eq!(payload["side_effects"]["writes_deprecation_notices"], false);
    assert_eq!(
        payload["config_footprint"]["managed_config_paths"][0],
        "~/.cmdrvl/config/assess/policies/"
    );
    assert_eq!(
        payload["config_footprint"]["managed_state_paths"][0],
        "~/.cmdrvl/state/witness/witness.jsonl"
    );
    assert_all_side_effects_false(side_effects(&payload)?)?;
    assert!(payload["fixers"].as_array().is_some_and(Vec::is_empty));
    assert_doctor_side_effects_absent(workspace.path(), &witness_path);
    Ok(())
}

#[test]
fn doctor_capabilities_json_has_no_fixers_or_side_effects() -> Result<(), Box<dyn std::error::Error>>
{
    let workspace = support::TempWorkspace::new("doctor-capabilities")?;
    let witness_path = workspace.child("witness/assess-witness.jsonl");

    let output = assess_cmd_in(workspace.path(), &witness_path)
        .args(["doctor", "capabilities", "--json"])
        .output()?;
    let payload = parse_stdout_json(&output)?;

    assert_eq!(payload["schema"], "assess.doctor.capabilities.v1");
    assert_eq!(payload["contract"], "cmdrvl.read_only_doctor.v1");
    assert_eq!(payload["read_only"], true);
    assert_eq!(payload["config_footprint"]["self_contained"], true);
    assert_all_side_effects_false(side_effects(&payload)?)?;
    assert!(payload["fixers"].as_array().is_some_and(Vec::is_empty));
    assert!(
        payload["commands"]
            .as_array()
            .map(|commands| {
                commands.iter().any(|command| {
                    command["name"] == "robot-triage"
                        && command["usage"] == "assess doctor --robot-triage"
                })
            })
            .unwrap_or(false)
    );
    assert_doctor_side_effects_absent(workspace.path(), &witness_path);
    Ok(())
}

#[test]
fn doctor_robot_triage_json_is_machine_readable() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("doctor-triage")?;
    let witness_path = workspace.child("witness/assess-witness.jsonl");

    let output = assess_cmd_in(workspace.path(), &witness_path)
        .args(["doctor", "--robot-triage"])
        .output()?;
    let payload = parse_stdout_json(&output)?;

    assert_eq!(payload["schema"], "assess.doctor.triage.v1");
    assert_eq!(payload["contract"], "cmdrvl.read_only_doctor.v1");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["score"], 100);
    assert_eq!(payload["read_only"], true);
    assert_eq!(
        payload["config_footprint"]["deprecation_notices"],
        "~/.cmdrvl/notices/deprecated-paths.jsonl"
    );
    assert_all_side_effects_false(side_effects(&payload)?)?;
    assert_doctor_side_effects_absent(workspace.path(), &witness_path);
    Ok(())
}

#[test]
fn doctor_robot_docs_is_plain_text_and_read_only() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("doctor-docs")?;
    let witness_path = workspace.child("witness/assess-witness.jsonl");

    let output = assess_cmd_in(workspace.path(), &witness_path)
        .args(["doctor", "robot-docs"])
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("cmdrvl.read_only_doctor.v1"));
    assert!(stdout.contains("assess doctor health --json"));
    assert!(stdout.contains("~/.cmdrvl/config/assess/policies"));
    assert!(stdout.contains("no --fix surface"));
    assert_doctor_side_effects_absent(workspace.path(), &witness_path);
    Ok(())
}

#[test]
fn doctor_help_is_available() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_assess"))
        .args(["doctor", "--help"])
        .output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("health"));
    assert!(stdout.contains("capabilities"));
    assert!(stdout.contains("robot-docs"));
    assert!(stdout.contains("--robot-triage"));
    Ok(())
}

#[test]
fn doctor_fix_is_not_available() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("doctor-fix")?;
    let witness_path = workspace.child("witness/assess-witness.jsonl");

    let output = assess_cmd_in(workspace.path(), &witness_path)
        .args(["doctor", "--fix"])
        .output()?;

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("--fix"));
    assert_doctor_side_effects_absent(workspace.path(), &witness_path);
    Ok(())
}

#[test]
fn describe_manifest_includes_doctor() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_assess"))
        .arg("--describe")
        .output()?;

    let payload = parse_stdout_json(&output)?;
    assert_eq!(payload["version"], env!("CARGO_PKG_VERSION"));
    assert!(
        payload["subcommands"]
            .as_array()
            .map(|subcommands| {
                subcommands.iter().any(|entry| {
                    entry["name"] == "doctor"
                        && entry["status"] == "implemented"
                        && entry["read_only"] == true
                })
            })
            .unwrap_or(false)
    );
    Ok(())
}
