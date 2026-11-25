mod support;

use std::collections::BTreeSet;

use assess::{cli::AssessExit, policy::DecisionBand, refusal::RefusalCode};
use serde_json::Value;

// Family contract: spine-rules golden_rules_suite! (operator.v1 validation, version and
// subcommand parity, capabilities == describe, doctor cmdrvl.doctor.v1, flag dictionary,
// ANSI-free piped output, bare invocation, empty witness query).
spine_rules::golden_rules_suite! {
    binary: env!("CARGO_BIN_EXE_assess"),
    operator_json: include_str!("../operator.json"),
    source_files: &["src/refusal/codes.rs"],
    fixture_success_args: &[
        "fixtures/artifacts/shape_compatible.json",
        "fixtures/artifacts/rvl_real_change.json",
        "fixtures/artifacts/verify_pass.json",
        "--policy-id",
        "loan_tape.monthly.v1",
        "--json",
        "--no-witness",
    ],
    fixture_refusal_args: &[
        "fixtures/artifacts/does_not_exist.json",
        "--policy-id",
        "default.v0",
        "--json",
        "--no-witness",
    ],
    fixture_empty_query_args: &["witness", "query", "policy=no.such.policy.v9", "--json"],
}

#[test]
fn operator_manifest_describes_assess_contract() -> Result<(), Box<dyn std::error::Error>> {
    let operator: Value = serde_json::from_str(include_str!("../operator.json"))?;

    assert_eq!(operator["schema_version"], "operator.v1");
    assert_eq!(operator["name"], "assess");
    assert_eq!(operator["tool_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        operator["pipeline"]["upstream"],
        serde_json::json!(["shape", "rvl", "verify", "benchmark"])
    );
    assert_eq!(
        operator["pipeline"]["downstream"],
        serde_json::json!(["pack"])
    );

    let usage = operator["invocation"]["usage"]
        .as_array()
        .expect("operator invocation usage should be an array")
        .iter()
        .map(|entry| entry.as_str().expect("usage entries should be strings"))
        .collect::<Vec<_>>();
    assert!(usage.contains(&"assess <ARTIFACT>... --policy <POLICY> [OPTIONS]"));
    assert!(usage.contains(&"assess <ARTIFACT>... --policy-id <ID> [OPTIONS]"));
    assert!(usage.contains(&"assess --robot-triage"));
    assert!(usage.contains(&"assess capabilities --json"));
    assert!(usage.contains(&"assess robot-docs guide"));
    assert!(usage.contains(&"assess witness <query|last|count> [OPTIONS]"));

    for (code, domain) in [
        ("0", "positive"),
        ("1", "negative"),
        ("2", "error"),
        ("3", "policy"),
    ] {
        assert_eq!(operator["exit_codes"][code]["domain"], domain);
        assert_eq!(operator["exit_codes"][code]["retryable"], false);
    }
    assert!(
        operator["exit_codes"]["0"]["meaning"]
            .as_str()
            .unwrap()
            .starts_with("PROCEED | PROCEED_WITH_RISK")
    );
    assert_eq!(
        operator["exit_codes"]["1"]["meaning"],
        "ESCALATE (doctor: unhealthy)"
    );
    assert!(
        operator["exit_codes"]["2"]["meaning"]
            .as_str()
            .unwrap()
            .starts_with("REFUSAL | CLI error")
    );
    assert!(
        operator["exit_codes"]["3"]["meaning"]
            .as_str()
            .unwrap()
            .starts_with("BLOCK")
    );

    assert_eq!(
        AssessExit::from_decision_band(DecisionBand::Proceed).code(),
        0
    );
    assert_eq!(
        AssessExit::from_decision_band(DecisionBand::ProceedWithRisk).code(),
        0
    );
    assert_eq!(
        AssessExit::from_decision_band(DecisionBand::Escalate).code(),
        1
    );
    assert_eq!(
        AssessExit::from_decision_band(DecisionBand::Block).code(),
        3
    );

    let refusal_codes = operator["refusals"]
        .as_array()
        .expect("operator refusals should be an array")
        .iter()
        .map(|entry| {
            entry["code"]
                .as_str()
                .expect("refusal code should be a string")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    let expected_refusal_codes = RefusalCode::ALL
        .iter()
        .map(|code| code.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(refusal_codes, expected_refusal_codes);

    assert_eq!(
        operator["formats"],
        serde_json::json!(["human", "json", "summary", "summary-tsv"])
    );
    assert_eq!(operator["options"][3]["flag"], "--render");
    assert_eq!(
        operator["options"][3]["values"],
        serde_json::json!(["summary", "summary-tsv"])
    );
    let subcommands = operator["commands"]
        .as_array()
        .expect("operator commands should be an array");
    let witness = subcommands
        .iter()
        .find(|entry| entry["name"] == "witness")
        .expect("witness subcommand should be declared");
    assert_eq!(witness["status"], "implemented");
    assert_eq!(witness["current_runtime_behavior"]["status"], "implemented");
    let capabilities = subcommands
        .iter()
        .find(|entry| entry["name"] == "capabilities")
        .expect("top-level capabilities alias should be declared");
    assert_eq!(capabilities["read_only"], true);
    let robot_docs = subcommands
        .iter()
        .find(|entry| entry["name"] == "robot-docs")
        .expect("top-level robot-docs alias should be declared");
    assert_eq!(robot_docs["usage"], "assess robot-docs guide");
    let notes = operator["notes"].as_array().expect("notes array");
    assert!(
        notes.iter().any(|n| n
            .as_str()
            .is_some_and(|n| n.contains("retryable is a local assess extension"))),
        "the retryable extension must stay documented as local"
    );

    Ok(())
}

#[test]
fn golden_rule_files_match_spine_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let exit_rule = support::read_text(support::rule_path("exit-code-range.yml"))?;
    assert!(exit_rule.contains("id: exit-code-range"));
    assert!(
        exit_rule.contains(
            "R-002: Spine tools use exit codes 0, 1, 2 and, only for a documented policy"
        )
    );
    assert!(exit_rule.contains("pattern: process::exit(255)"));
    assert!(
        !exit_rule.contains("pattern: process::exit(3)"),
        "exit 3 is the BLOCK policy band, not a range violation"
    );

    let band_rule = support::read_text(support::rule_path("exit-code-policy-band.yml"))?;
    assert!(band_rule.contains("id: exit-code-policy-band"));
    assert!(band_rule.contains("pattern: process::exit(3)"));

    let hashmap_rule = support::read_text(support::rule_path("no-hashmap-in-output.yml"))?;
    assert!(hashmap_rule.contains("id: no-hashmap-in-output"));
    assert!(hashmap_rule.contains("HashMap has non-deterministic iteration order."));
    assert!(hashmap_rule.contains("pattern: HashMap::new()"));
    assert!(hashmap_rule.contains("BTreeMap"));

    let witness_rule = support::read_text(support::rule_path("witness-must-append.yml"))?;
    assert!(witness_rule.contains("id: witness-must-append"));
    assert!(witness_rule.contains("Witness files are append-only."));
    assert!(witness_rule.contains("pattern: File::create($PATH)"));

    Ok(())
}

#[test]
fn current_sources_respect_local_golden_rules() -> Result<(), Box<dyn std::error::Error>> {
    let main_source = support::read_text(support::repo_path().join("src/main.rs"))?;
    assert!(
        !main_source.contains("process::exit("),
        "exit-code golden rule forbids hardcoded process exits"
    );

    let output_source = support::read_text(support::repo_path().join("src/output/json.rs"))?;
    let refusal_source = support::read_text(support::repo_path().join("src/refusal/payload.rs"))?;
    assert!(
        !output_source.contains("HashMap"),
        "output rendering must avoid HashMap for deterministic ordering"
    );
    assert!(
        !refusal_source.contains("HashMap"),
        "refusal payloads must avoid HashMap for deterministic ordering"
    );

    let witness_source = support::read_text(support::repo_path().join("src/witness/ledger.rs"))?;
    assert!(
        !witness_source.contains("File::create("),
        "witness ledger must remain append-only when implemented"
    );

    Ok(())
}
