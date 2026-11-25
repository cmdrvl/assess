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

    assert_eq!(payload["schema_version"], "cmdrvl.doctor.v1");
    assert_eq!(payload["mode"], "health");
    assert_eq!(payload["read_only_contract"], "cmdrvl.read_only_doctor.v1");
    assert_eq!(payload["tool"], "assess");
    assert_eq!(payload["tool_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(payload["healthy"], true);
    assert_eq!(payload["findings"], serde_json::json!([]));
    assert_eq!(output.status.code(), Some(0));
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

    assert_eq!(payload["schema_version"], "cmdrvl.doctor.v1");
    assert_eq!(payload["mode"], "capabilities");
    assert_eq!(payload["read_only_contract"], "cmdrvl.read_only_doctor.v1");
    assert_eq!(payload["contract"]["schema_version"], "operator.v1");
    assert_eq!(payload["read_only"], true);
    assert_eq!(payload["config_footprint"]["self_contained"], true);
    assert_all_side_effects_false(side_effects(&payload)?)?;
    assert!(payload["fixers"].as_array().is_some_and(Vec::is_empty));
    assert!(
        payload["commands"]
            .as_array()
            .map(|commands| {
                commands.iter().any(|command| {
                    command["name"] == "doctor-robot-triage"
                        && command["usage"] == "assess doctor --robot-triage"
                })
            })
            .unwrap_or(false)
    );
    assert_eq!(
        payload["agent_entrypoints"][1]["usage"],
        "assess capabilities --json"
    );
    assert_eq!(
        payload["agent_entrypoints"][0]["usage"],
        "assess doctor triage --json"
    );
    assert_eq!(payload["composition"]["role"], "decision_classifier");
    assert_eq!(
        payload["composition"]["canonical_chains"][0]["upstream_tools"][0],
        "shape"
    );
    assert_eq!(
        payload["composition"]["canonical_chains"][0]["commands"][4],
        "assess shape.json rvl.json verify.json benchmark.json --policy <policy.yaml> --json > decision.json"
    );
    assert_eq!(payload["refusal_codes"][5]["code"], "E_INCOMPLETE_BASIS");
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

    assert_eq!(payload["schema_version"], "cmdrvl.doctor.v1");
    assert_eq!(payload["mode"], "triage");
    assert_eq!(payload["read_only_contract"], "cmdrvl.read_only_doctor.v1");
    assert_eq!(payload["healthy"], true);
    assert!(
        payload["next_actions"]
            .as_array()
            .is_some_and(|a| !a.is_empty())
    );
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
    assert!(stdout.contains("assess --robot-triage"));
    assert!(stdout.contains("assess capabilities --json"));
    assert!(stdout.contains("assess robot-docs guide"));
    assert!(stdout.contains("assess doctor health --json"));
    assert!(stdout.contains("assess shape.json rvl.json verify.json benchmark.json --policy <policy.yaml> --json > decision.json"));
    assert!(stdout.contains(
        "pack seal shape.json rvl.json verify.json benchmark.json decision.json --output evidence/"
    ));
    assert!(stdout.contains("~/.cmdrvl/config/assess/policies"));
    assert!(stdout.contains("no --fix surface"));
    assert_doctor_side_effects_absent(workspace.path(), &witness_path);
    Ok(())
}

#[test]
fn top_level_agent_aliases_are_read_only() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("doctor-top-level")?;
    let witness_path = workspace.child("witness/assess-witness.jsonl");

    let capabilities = assess_cmd_in(workspace.path(), &witness_path)
        .args(["capabilities", "--json"])
        .output()?;
    let payload = parse_stdout_json(&capabilities)?;
    // Top-level `capabilities --json` is the operator.v1 document, byte-for-byte `--describe`.
    assert_eq!(payload["schema_version"], "operator.v1");
    assert_eq!(payload["name"], "assess");
    let describe = assess_cmd_in(workspace.path(), &witness_path)
        .arg("--describe")
        .output()?;
    assert_eq!(capabilities.stdout, describe.stdout);

    let triage = assess_cmd_in(workspace.path(), &witness_path)
        .arg("--robot-triage")
        .output()?;
    let payload = parse_stdout_json(&triage)?;
    assert_eq!(payload["schema_version"], "cmdrvl.doctor.v1");
    assert_eq!(payload["mode"], "triage");
    assert_eq!(payload["read_only"], true);

    let docs = assess_cmd_in(workspace.path(), &witness_path)
        .args(["robot-docs", "guide"])
        .output()?;
    assert!(docs.status.success());
    assert!(String::from_utf8(docs.stdout)?.contains("assess robot-docs guide"));

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
    assert!(stdout.contains("triage"));
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

/// Every string anywhere in `value` that names an operator schema (`operator.v<N>`), with the
/// JSON path it was found at.
fn operator_schema_labels(value: &Value, path: &str, found: &mut Vec<(String, String)>) {
    match value {
        Value::String(text) => {
            let is_label = text
                .strip_prefix("operator.v")
                .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()));
            if is_label {
                found.push((path.to_owned(), text.clone()));
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                operator_schema_labels(item, &format!("{path}[{index}]"), found);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                operator_schema_labels(item, &format!("{path}.{key}"), found);
            }
        }
        _ => {}
    }
}

/// The labels in `doctor_document` that disagree with the schema `--describe` emits.
fn mismatched_operator_labels(doctor_document: &Value, describe_schema: &str) -> Vec<String> {
    let mut found = Vec::new();
    operator_schema_labels(doctor_document, "$", &mut found);
    found
        .into_iter()
        .filter(|(_, label)| label != describe_schema)
        .map(|(path, label)| format!("{path} = {label}"))
        .collect()
}

#[test]
fn every_operator_schema_label_in_doctor_output_matches_what_describe_emits()
-> Result<(), Box<dyn std::error::Error>> {
    let describe = parse_stdout_json(
        &Command::new(env!("CARGO_BIN_EXE_assess"))
            .arg("--describe")
            .output()?,
    )?;
    let emitted = describe["schema_version"]
        .as_str()
        .ok_or_else(|| missing_json_field("schema_version"))?
        .to_owned();
    assert_eq!(emitted, "operator.v1");

    for args in [
        &["doctor", "capabilities", "--json"][..],
        &["doctor", "health", "--json"][..],
        &["doctor", "triage", "--json"][..],
        &["capabilities", "--json"][..],
    ] {
        let payload = parse_stdout_json(
            &Command::new(env!("CARGO_BIN_EXE_assess"))
                .args(args)
                .output()?,
        )?;
        let mut found = Vec::new();
        operator_schema_labels(&payload, "$", &mut found);
        assert!(
            args == ["doctor", "health", "--json"]
                || args == ["doctor", "triage", "--json"]
                || !found.is_empty(),
            "{args:?}: expected at least one operator schema label"
        );
        assert_eq!(
            mismatched_operator_labels(&payload, &emitted),
            Vec::<String>::new(),
            "{args:?}: operator schema labels must equal what --describe emits ({emitted})"
        );
    }
    Ok(())
}

#[test]
fn a_stale_operator_v0_label_is_caught_by_the_label_check() {
    // Planted negative: the exact defect this test exists for (doctor capabilities labelling
    // the describe action `operator.v0` while --describe emits operator.v1).
    let planted = serde_json::json!({
        "agent_entrypoints": [
            { "name": "describe", "output_schema": "operator.v0" },
            { "name": "capabilities", "output_schema": "operator.v1" }
        ]
    });
    assert_eq!(
        mismatched_operator_labels(&planted, "operator.v1"),
        vec!["$.agent_entrypoints[0].output_schema = operator.v0".to_owned()]
    );
}

#[test]
fn describe_manifest_includes_doctor() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_assess"))
        .arg("--describe")
        .output()?;

    let payload = parse_stdout_json(&output)?;
    assert_eq!(payload["tool_version"], env!("CARGO_PKG_VERSION"));
    assert!(
        payload["commands"]
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
