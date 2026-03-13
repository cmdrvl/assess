mod support;

use std::collections::BTreeSet;

use assess::{cli::AssessExit, policy::DecisionBand, refusal::RefusalCode};
use serde_json::Value;

#[test]
fn operator_manifest_describes_assess_contract() -> Result<(), Box<dyn std::error::Error>> {
    let operator: Value = serde_json::from_str(include_str!("../operator.json"))?;

    assert_eq!(operator["schema_version"], "operator.v0");
    assert_eq!(operator["name"], "assess");
    assert_eq!(operator["version"], env!("CARGO_PKG_VERSION"));
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
    assert!(usage.contains(&"assess witness <query|last|count> [OPTIONS]"));

    assert_eq!(operator["exit_codes"]["0"]["meaning"], "PROCEED");
    assert_eq!(
        operator["exit_codes"]["1"]["meaning"],
        "PROCEED_WITH_RISK or ESCALATE"
    );
    assert_eq!(
        operator["exit_codes"]["2"]["meaning"],
        "BLOCK / REFUSAL / CLI error"
    );

    assert_eq!(
        AssessExit::from_decision_band(DecisionBand::Proceed).code(),
        0
    );
    assert_eq!(
        AssessExit::from_decision_band(DecisionBand::ProceedWithRisk).code(),
        1
    );
    assert_eq!(
        AssessExit::from_decision_band(DecisionBand::Escalate).code(),
        1
    );
    assert_eq!(
        AssessExit::from_decision_band(DecisionBand::Block).code(),
        2
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
        operator["capabilities"]["formats"],
        serde_json::json!(["human", "json", "summary", "summary-tsv"])
    );
    assert_eq!(operator["options"][3]["flag"], "--render");
    assert_eq!(
        operator["options"][3]["values"],
        serde_json::json!(["summary", "summary-tsv"])
    );
    assert_eq!(operator["subcommands"][0]["name"], "witness");
    assert_eq!(operator["subcommands"][0]["status"], "implemented");
    assert_eq!(
        operator["subcommands"][0]["current_runtime_behavior"]["status"],
        "implemented"
    );
    assert_eq!(
        operator["notes"],
        serde_json::json!([
            "Metadata surfaces (--describe, --schema, --version) are implemented.",
            "Decision execution and witness execution are implemented.",
            "Run mode supports compact summary and TSV summary render surfaces via --render."
        ])
    );

    Ok(())
}

#[test]
fn golden_rule_files_match_spine_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let exit_rule = support::read_text(support::rule_path("exit-code-range.yml"))?;
    assert!(exit_rule.contains("id: exit-code-range"));
    assert!(exit_rule.contains("R-002: Spine tools use only exit codes 0, 1, 2."));
    assert!(exit_rule.contains("pattern: process::exit(255)"));

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
