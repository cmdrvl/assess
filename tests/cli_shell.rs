use std::path::PathBuf;

use assess::cli::{
    AssessExit, Cli, DoctorInvocationCommand, PolicySelector, Route, WitnessInvocationCommand,
    format_cli_error, route,
};
use assess::output::RenderMode;
use assess::policy::DecisionBand;
use assess::{ASSESS_SCHEMA_JSON, execute};
use clap::{Parser, error::ErrorKind};

#[test]
fn missing_run_inputs_stay_on_usage_error_path() {
    let missing_artifacts = route(Cli::parse_from([
        "assess",
        "--policy",
        "fixtures/policies/loan_tape_monthly_v1.yaml",
    ]))
    .expect_err("artifact list should be required for assess runs");
    assert!(matches!(
        missing_artifacts,
        assess::cli::RouteError::Usage(ref error)
            if error.kind() == ErrorKind::MissingRequiredArgument
    ));

    let missing_policy = route(Cli::parse_from([
        "assess",
        "fixtures/artifacts/shape_clean.json",
    ]))
    .expect_err("policy selector should be required for assess runs");
    assert!(matches!(
        missing_policy,
        assess::cli::RouteError::Usage(ref error)
            if error.kind() == ErrorKind::MissingRequiredArgument
    ));
}

#[test]
fn bare_invocation_prints_agent_entrypoints() -> Result<(), Box<dyn std::error::Error>> {
    let execution = execute(Cli::parse_from(["assess"]))?;

    assert_eq!(execution.exit_code, 0);
    assert!(execution.stdout.contains("Agent entrypoints:"));
    assert!(execution.stdout.contains("assess --robot-triage"));
    assert!(execution.stdout.contains("assess capabilities --json"));
    assert!(execution.stdout.contains("assess robot-docs guide"));
    Ok(())
}

#[test]
fn json_only_returns_capabilities_contract() -> Result<(), Box<dyn std::error::Error>> {
    let execution = execute(Cli::parse_from(["assess", "--json"]))?;

    assert_eq!(execution.exit_code, 0);
    let payload: serde_json::Value = serde_json::from_str(execution.stdout.trim())?;
    assert_eq!(payload["schema"], "assess.doctor.capabilities.v1");
    assert_eq!(
        payload["agent_entrypoints"][0]["usage"],
        "assess --robot-triage"
    );
    Ok(())
}

#[test]
fn top_level_robot_triage_routes_to_read_only_json() {
    let route =
        route(Cli::parse_from(["assess", "--robot-triage"])).expect("robot triage should route");

    assert_eq!(
        route,
        Route::Doctor(assess::cli::DoctorInvocation {
            command: DoctorInvocationCommand::RobotTriage,
            json: true,
        })
    );
}

#[test]
fn top_level_capabilities_and_robot_docs_aliases_route() {
    let capabilities = route(Cli::parse_from(["assess", "capabilities", "--json"]))
        .expect("capabilities alias should route");
    assert_eq!(
        capabilities,
        Route::Doctor(assess::cli::DoctorInvocation {
            command: DoctorInvocationCommand::Capabilities,
            json: true,
        })
    );

    let robot_docs = route(Cli::parse_from(["assess", "robot-docs", "guide"]))
        .expect("robot-docs guide alias should route");
    assert_eq!(
        robot_docs,
        Route::Doctor(assess::cli::DoctorInvocation {
            command: DoctorInvocationCommand::RobotDocs,
            json: false,
        })
    );
}

#[test]
fn json_typo_error_names_corrected_command() {
    let error = Cli::try_parse_from(["assess", "--jsno"]).expect_err("--jsno should fail");
    let rendered = format_cli_error(&error);

    assert!(rendered.contains("did you mean `--json`"));
    assert!(rendered.contains("next: assess capabilities --json"));
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

    if let assess::cli::RouteError::Usage(error) = &err {
        assert_eq!(error.kind(), ErrorKind::ArgumentConflict);
        assert!(
            error
                .to_string()
                .contains("`--render` is not supported with `assess witness`")
        );
    }
    assert!(matches!(err, assess::cli::RouteError::Usage(_)));
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
