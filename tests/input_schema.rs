mod support;

use std::process::Command;

use assess::cli::Cli;
use assess::policy;
use assess::{POLICY_SCHEMA_JSON, execute};
use clap::Parser;
use serde_json::Value;

const MINIMAL_POLICY: &str = r#"schema_version: 1
policy_id: example.v0
policy_version: 1
rules:
  - name: default_proceed
    default: true
    then:
      decision_band: PROCEED
"#;

const INVALID_POLICY: &str = r#"schema_version: 1
policy_id: example.invalid.v0
policy_version: 1
rules:
  - name: malformed_default
    default: true
    then:
      decision_band: MAYBE
"#;

fn policy_schema() -> Value {
    serde_json::from_str(POLICY_SCHEMA_JSON).expect("policy schema should parse")
}

fn schema_errors(instance: &Value) -> Vec<String> {
    let validator = jsonschema::validator_for(&policy_schema()).expect("schema should compile");
    validator
        .iter_errors(instance)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect()
}

fn policy_as_json(yaml: &str) -> Value {
    serde_yaml::from_str(yaml).expect("policy YAML should be representable as JSON")
}

#[test]
fn input_schema_is_zero_argument_policy_json_schema() -> Result<(), Box<dyn std::error::Error>> {
    let execution = execute(Cli::parse_from(["assess", "--input-schema"]))?;
    let child = Command::new(env!("CARGO_BIN_EXE_assess"))
        .arg("--input-schema")
        .output()?;

    assert_eq!(execution.exit_code, 0);
    assert!(child.status.success());
    assert_eq!(child.stdout, execution.stdout.as_bytes());
    let schema: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(schema["$schema"], policy_schema()["$schema"]);
    assert_eq!(schema["title"], "policy.v0");
    assert!(
        schema["required"]
            .as_array()
            .is_some_and(|required| required.iter().any(|field| field == "rules"))
    );
    assert!(schema_errors(&policy_as_json(MINIMAL_POLICY)).is_empty());
    for fixture in [
        "loan_tape_monthly_v1.yaml",
        "claims_verifier_graam_v1.yaml",
        "minimal_default_only.yaml",
    ] {
        let path = support::policy_path(fixture);
        let raw = support::read_text(&path)?;
        assert!(
            schema_errors(&policy_as_json(&raw)).is_empty(),
            "shipped policy fixture {fixture} must satisfy policy.v0"
        );
        policy::load_path(path)?;
    }
    Ok(())
}

#[test]
fn help_robot_docs_and_capabilities_publish_the_same_policy_guidance() {
    let shared = [
        "assess --input-schema",
        "schema_version: 1",
        "policy_id: example.v0",
        "decision_band: PROCEED",
    ];

    let help = assess::cli::long_help();
    let guide = execute(Cli::parse_from(["assess", "robot-docs", "guide"]))
        .expect("robot guide should render")
        .stdout;
    let capabilities = execute(Cli::parse_from(["assess", "capabilities", "--json"]))
        .expect("capabilities should render")
        .stdout;
    let capabilities_json: Value = serde_json::from_str(capabilities.trim())
        .expect("capabilities should remain valid operator JSON");

    for needle in shared {
        assert!(help.contains(needle), "help omitted {needle:?}: {help}");
        assert!(guide.contains(needle), "guide omitted {needle:?}: {guide}");
    }
    assert!(help.contains("matcher keys: outcome | outcome_in | refusal | signals"));
    assert!(guide.contains("matcher_keys: outcome, outcome_in, refusal, signals"));
    assert!(help.contains("decision bands: PROCEED | PROCEED_WITH_RISK | ESCALATE | BLOCK"));
    assert!(guide.contains("decision_bands: PROCEED, PROCEED_WITH_RISK, ESCALATE, BLOCK"));
    assert_eq!(
        capabilities_json["capabilities"]["policy_authoring"]["schema"],
        "assess --input-schema"
    );
    assert!(
        capabilities_json["capabilities"]["policy_authoring"]["minimal_policy_yaml"]
            .as_str()
            .is_some_and(|yaml| yaml.contains("policy_id: example.v0"))
    );
    assert_eq!(
        capabilities_json["capabilities"]["policy_authoring"]["matcher_keys"],
        serde_json::json!(["outcome", "outcome_in", "refusal", "signals"])
    );
    assert_eq!(
        capabilities_json["capabilities"]["policy_authoring"]["decision_bands"],
        serde_json::json!(["PROCEED", "PROCEED_WITH_RISK", "ESCALATE", "BLOCK"])
    );
}

#[test]
fn documented_policy_loads_and_produces_a_decision() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("policy-input-schema")?;
    let policy_path = workspace.write("policy.yaml", MINIMAL_POLICY)?;
    let artifact = support::artifact_path("shape_compatible.json");

    let loaded = policy::load_path(&policy_path)?;
    assert_eq!(loaded.policy.policy_id, "example.v0");
    assert!(schema_errors(&policy_as_json(MINIMAL_POLICY)).is_empty());

    let execution = execute(Cli::parse_from([
        "assess",
        artifact.to_str().expect("fixture path is UTF-8"),
        "--policy",
        policy_path.to_str().expect("policy path is UTF-8"),
        "--json",
        "--no-witness",
    ]))?;
    assert_eq!(execution.exit_code, 0);
    let decision: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(decision["decision_band"], "PROCEED");
    assert_eq!(decision["policy"]["id"], "example.v0");
    Ok(())
}

#[test]
fn invalid_policy_is_rejected_by_schema_loader_and_cli() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("invalid-policy-input-schema")?;
    let policy_path = workspace.write("invalid.yaml", INVALID_POLICY)?;
    let artifact = support::artifact_path("shape_compatible.json");

    assert!(!schema_errors(&policy_as_json(INVALID_POLICY)).is_empty());
    assert!(policy::load_path(&policy_path).is_err());

    let execution = execute(Cli::parse_from([
        "assess",
        artifact.to_str().expect("fixture path is UTF-8"),
        "--policy",
        policy_path.to_str().expect("policy path is UTF-8"),
        "--json",
        "--no-witness",
    ]))?;
    assert_eq!(execution.exit_code, 2);
    let refusal: Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(refusal["refusal"]["code"], "E_BAD_POLICY");
    Ok(())
}

#[test]
fn shipped_example_can_be_loaded_by_the_child_binary() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("policy-child")?;
    let policy_path = workspace.write("policy.yaml", MINIMAL_POLICY)?;
    let artifact = support::artifact_path("shape_compatible.json");
    let witness = workspace.child("witness.jsonl");
    let output = Command::new(env!("CARGO_BIN_EXE_assess"))
        .env("EPISTEMIC_WITNESS", &witness)
        .args([
            artifact.to_str().expect("fixture path is UTF-8"),
            "--policy",
            policy_path.to_str().expect("policy path is UTF-8"),
            "--json",
            "--no-witness",
        ])
        .output()?;
    assert!(output.status.success());
    let decision: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(decision["decision_band"], "PROCEED");
    assert!(!witness.exists());
    Ok(())
}
