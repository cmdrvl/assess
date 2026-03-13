use std::path::PathBuf;

use assess::cli::{AssessExit, Cli, PolicySelector, Route, WitnessInvocationCommand, route};
use assess::output::RenderMode;
use assess::policy::DecisionBand;
use assess::{ASSESS_SCHEMA_JSON, execute};
use clap::{Parser, error::ErrorKind};

#[test]
fn missing_run_inputs_stay_on_parse_error_path() {
    let missing_artifacts = Cli::try_parse_from([
        "assess",
        "--policy",
        "fixtures/policies/loan_tape_monthly_v1.yaml",
    ])
    .expect_err("artifact list should be required for assess runs");
    assert_eq!(missing_artifacts.kind(), ErrorKind::MissingRequiredArgument);

    let missing_policy = Cli::try_parse_from(["assess", "fixtures/artifacts/shape_clean.json"])
        .expect_err("policy selector should be required for assess runs");
    assert_eq!(missing_policy.kind(), ErrorKind::MissingRequiredArgument);
}

#[test]
fn special_flags_precede_subcommand_routing() -> Result<(), Box<dyn std::error::Error>> {
    let execution = execute(Cli::parse_from(["assess", "witness", "last", "--schema"]))?;

    assert_eq!(execution.exit_code, 0);
    let expected = if ASSESS_SCHEMA_JSON.ends_with('\n') {
        ASSESS_SCHEMA_JSON.to_owned()
    } else {
        format!("{ASSESS_SCHEMA_JSON}\n")
    };
    assert_eq!(execution.stdout, expected);
    Ok(())
}

#[test]
fn ambiguous_policy_selector_returns_refusal_json() -> Result<(), Box<dyn std::error::Error>> {
    let execution = execute(Cli::parse_from([
        "assess",
        "fixtures/artifacts/shape_clean.json",
        "--policy",
        "fixtures/policies/loan_tape_monthly_v1.yaml",
        "--policy-id",
        "loan_tape.monthly.v1",
        "--json",
    ]))?;

    assert_eq!(execution.exit_code, 2);
    let refusal: serde_json::Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(refusal["tool"], "assess");
    assert_eq!(refusal["version"], "assess.v0");
    assert!(refusal["decision_band"].is_null());
    assert_eq!(refusal["refusal"]["code"], "E_AMBIGUOUS_POLICY");
    assert_eq!(
        refusal["refusal"]["detail"]["policy"],
        "fixtures/policies/loan_tape_monthly_v1.yaml"
    );
    assert_eq!(
        refusal["refusal"]["detail"]["policy_id"],
        "loan_tape.monthly.v1"
    );
    Ok(())
}

#[test]
fn successful_routes_preserve_run_and_witness_shape() -> Result<(), Box<dyn std::error::Error>> {
    let run_route = route(Cli::parse_from([
        "assess",
        "fixtures/artifacts/shape_clean.json",
        "--policy-id",
        "loan_tape.monthly.v1",
    ]))
    .expect("run route should parse and validate");

    assert_eq!(
        run_route,
        Route::Run(assess::cli::RunCommand {
            artifacts: vec![PathBuf::from("fixtures/artifacts/shape_clean.json")],
            policy_selector: PolicySelector::Id("loan_tape.monthly.v1".to_owned()),
            render_mode: RenderMode::Human,
            no_witness: false,
        })
    );

    let witness_route = route(Cli::parse_from([
        "assess",
        "witness",
        "count",
        "policy=loan_tape.monthly.v1",
        "--json",
    ]))
    .expect("witness route should parse and validate");

    assert_eq!(
        witness_route,
        Route::Witness(assess::cli::WitnessInvocation {
            command: WitnessInvocationCommand::Count {
                filters: vec!["policy=loan_tape.monthly.v1".to_owned()],
            },
            json: true,
        })
    );

    Ok(())
}

#[test]
fn render_modes_route_and_conflicts_stay_explicit() {
    let summary_route = route(Cli::parse_from([
        "assess",
        "fixtures/artifacts/shape_clean.json",
        "--policy-id",
        "loan_tape.monthly.v1",
        "--render",
        "summary",
    ]))
    .expect("summary route should parse");

    assert_eq!(
        summary_route,
        Route::Run(assess::cli::RunCommand {
            artifacts: vec![PathBuf::from("fixtures/artifacts/shape_clean.json")],
            policy_selector: PolicySelector::Id("loan_tape.monthly.v1".to_owned()),
            render_mode: RenderMode::Summary,
            no_witness: false,
        })
    );

    let conflict = Cli::try_parse_from([
        "assess",
        "fixtures/artifacts/shape_clean.json",
        "--policy-id",
        "loan_tape.monthly.v1",
        "--json",
        "--render",
        "summary",
    ])
    .expect_err("json and render should conflict");
    assert_eq!(conflict.kind(), ErrorKind::ArgumentConflict);
}

#[test]
fn witness_subcommand_rejects_run_render_modes() {
    let err = route(Cli::parse_from([
        "assess", "--render", "summary", "witness", "last",
    ]))
    .expect_err("witness route should reject render mode");

    match err {
        assess::cli::RouteError::Usage(error) => {
            assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
            assert!(
                error
                    .to_string()
                    .contains("`--render` is not supported with `assess witness`")
            );
        }
        other => panic!("expected usage error, got {other:?}"),
    }
}

#[test]
fn decision_bands_map_to_cli_exit_codes() {
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
}
