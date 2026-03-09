mod support;

use serde_json::json;

use assess::{
    cli::Cli,
    execute,
    witness::{
        WitnessRecord,
        ledger::{append_to_path, load_from_path},
        query::{
            count_from_path, last_from_path, query_from_path, render_count_from_path,
            render_last_from_path, render_query_from_path, supported_modes,
        },
    },
};
use clap::Parser;

#[test]
fn witness_scaffold_shapes_exist() {
    let record = WitnessRecord::scaffold(vec!["shape.json".to_owned()]);
    assert_eq!(record.tool, "assess");
    assert_eq!(record.command, "run");
    assert_eq!(supported_modes().len(), 3);
}

#[test]
fn witness_ledger_append_and_load_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("witness-suite")?;
    let ledger_path = workspace.child("witness.jsonl");
    let record = WitnessRecord::scaffold(vec!["shape.json".to_owned()])
        .with_policy_id("loan_tape.monthly.v1")
        .with_decision_band("PROCEED")
        .with_duration_ms(42)
        .with_timestamp("12345");

    append_to_path(&ledger_path, &record)?;

    let loaded = load_from_path(&ledger_path)?;
    assert_eq!(loaded, vec![record]);
    Ok(())
}

#[test]
fn witness_query_filters_by_policy_and_input() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("witness-query")?;
    let ledger_path = workspace.child("witness.jsonl");

    append_to_path(
        &ledger_path,
        &WitnessRecord::scaffold(vec!["shape.json".to_owned(), "verify.json".to_owned()])
            .with_policy_id("loan_tape.monthly.v1")
            .with_decision_band("ESCALATE")
            .with_timestamp("1"),
    )?;
    append_to_path(
        &ledger_path,
        &WitnessRecord::scaffold(vec!["benchmark.json".to_owned()])
            .with_policy_id("other.policy.v1")
            .with_decision_band("BLOCK")
            .with_timestamp("2"),
    )?;

    let matches = query_from_path(
        &ledger_path,
        &[
            "policy=loan_tape.monthly.v1".to_owned(),
            "input=verify.json".to_owned(),
        ],
    )?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].decision_band.as_deref(), Some("ESCALATE"));
    assert_eq!(
        count_from_path(&ledger_path, &["policy=other.policy.v1".to_owned()])?,
        1
    );
    Ok(())
}

#[test]
fn witness_last_returns_latest_record() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("witness-last")?;
    let ledger_path = workspace.child("witness.jsonl");

    append_to_path(
        &ledger_path,
        &WitnessRecord::scaffold(vec!["shape.json".to_owned()]).with_timestamp("1"),
    )?;
    append_to_path(
        &ledger_path,
        &WitnessRecord::scaffold(vec!["verify.json".to_owned()])
            .with_decision_band("PROCEED_WITH_RISK")
            .with_timestamp("2"),
    )?;

    let last = last_from_path(&ledger_path)?.expect("latest record should exist");
    assert_eq!(last.inputs, vec!["verify.json".to_owned()]);
    assert_eq!(last.decision_band.as_deref(), Some("PROCEED_WITH_RISK"));
    Ok(())
}

#[test]
fn witness_render_helpers_support_json_and_human_modes() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("witness-render")?;
    let ledger_path = workspace.child("witness.jsonl");
    append_to_path(
        &ledger_path,
        &WitnessRecord::scaffold(vec!["shape.json".to_owned()])
            .with_policy_id("loan_tape.monthly.v1")
            .with_decision_band("PROCEED")
            .with_duration_ms(7)
            .with_timestamp("100"),
    )?;

    let query_json = render_query_from_path(&ledger_path, &[], true)?;
    let query_value: serde_json::Value = serde_json::from_str(&query_json.stdout)?;
    assert_eq!(query_json.exit_code, 0);
    assert_eq!(query_value.as_array().map(Vec::len), Some(1));

    let last_human = render_last_from_path(&ledger_path, false)?;
    assert_eq!(last_human.exit_code, 0);
    support::assert_human_lines(
        &last_human.stdout,
        &["100 PROCEED policy=loan_tape.monthly.v1 inputs=1 duration_ms=7"],
    );

    let count_json = render_count_from_path(&ledger_path, &[], true)?;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&count_json.stdout)?,
        json!({ "count": 1 })
    );
    Ok(())
}

#[test]
fn witness_execute_route_returns_json_count() -> Result<(), Box<dyn std::error::Error>> {
    let execution = execute(Cli::parse_from(["assess", "witness", "count", "--json"]))?;
    let json_value: serde_json::Value = serde_json::from_str(execution.stdout.trim())?;

    assert!(execution.exit_code <= 1);
    assert!(json_value.get("count").is_some());
    Ok(())
}
