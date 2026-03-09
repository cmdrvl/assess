use assess::cli::Cli;
use assess::{ASSESS_SCHEMA_JSON, OPERATOR_JSON, execute};
use clap::Parser;

// ---------------------------------------------------------------------------
// Determinism proof: metadata surfaces
// ---------------------------------------------------------------------------

#[test]
fn schema_output_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    let first = execute(Cli::parse_from(["assess", "--schema"]))?;
    let second = execute(Cli::parse_from(["assess", "--schema"]))?;

    let expected = if ASSESS_SCHEMA_JSON.ends_with('\n') {
        ASSESS_SCHEMA_JSON.to_owned()
    } else {
        format!("{ASSESS_SCHEMA_JSON}\n")
    };

    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stdout, expected);
    Ok(())
}

#[test]
fn describe_output_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    let first = execute(Cli::parse_from(["assess", "--describe"]))?;
    let second = execute(Cli::parse_from(["assess", "--describe"]))?;

    let expected = if OPERATOR_JSON.ends_with('\n') {
        OPERATOR_JSON.to_owned()
    } else {
        format!("{OPERATOR_JSON}\n")
    };

    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stdout, expected);
    Ok(())
}

#[test]
fn version_output_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    let first = execute(Cli::parse_from(["assess", "--version"]))?;
    let second = execute(Cli::parse_from(["assess", "--version"]))?;
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.exit_code, 0);
    Ok(())
}

// ---------------------------------------------------------------------------
// Determinism proof: full pipeline across all four decision bands
// ---------------------------------------------------------------------------

fn run_pipeline(artifacts: &[&str], policy: &str, json: bool) -> String {
    let mut args: Vec<&str> = vec!["assess"];
    args.extend_from_slice(artifacts);
    args.push("--policy");
    args.push(policy);
    if json {
        args.push("--json");
    }
    args.push("--no-witness");
    execute(Cli::parse_from(args)).unwrap().stdout
}

#[test]
fn proceed_json_is_byte_deterministic() {
    let artifacts = &[
        "fixtures/artifacts/shape_compatible.json",
        "fixtures/artifacts/rvl_real_change.json",
        "fixtures/artifacts/verify_pass.json",
    ];
    let policy = "fixtures/policies/loan_tape_monthly_v1.yaml";

    let a = run_pipeline(artifacts, policy, true);
    let b = run_pipeline(artifacts, policy, true);
    assert_eq!(a, b, "PROCEED JSON must be byte-identical across runs");
}

#[test]
fn proceed_human_is_byte_deterministic() {
    let artifacts = &[
        "fixtures/artifacts/shape_compatible.json",
        "fixtures/artifacts/rvl_real_change.json",
        "fixtures/artifacts/verify_pass.json",
    ];
    let policy = "fixtures/policies/loan_tape_monthly_v1.yaml";

    let a = run_pipeline(artifacts, policy, false);
    let b = run_pipeline(artifacts, policy, false);
    assert_eq!(a, b, "PROCEED human must be byte-identical across runs");
}

#[test]
fn proceed_with_risk_json_is_byte_deterministic() {
    let artifacts = &[
        "fixtures/artifacts/shape_incompatible_partial.json",
        "fixtures/artifacts/rvl_no_real_change.json",
        "fixtures/artifacts/verify_pass.json",
    ];
    let policy = "fixtures/policies/loan_tape_monthly_v1.yaml";

    let a = run_pipeline(artifacts, policy, true);
    let b = run_pipeline(artifacts, policy, true);
    assert_eq!(
        a, b,
        "PROCEED_WITH_RISK JSON must be byte-identical across runs"
    );
}

#[test]
fn escalate_json_is_byte_deterministic() {
    let artifacts = &[
        "fixtures/artifacts/shape_compatible.json",
        "fixtures/artifacts/rvl_refusal_diffuse.json",
        "fixtures/artifacts/verify_pass.json",
    ];
    let policy = "fixtures/policies/loan_tape_monthly_v1.yaml";

    let a = run_pipeline(artifacts, policy, true);
    let b = run_pipeline(artifacts, policy, true);
    assert_eq!(a, b, "ESCALATE JSON must be byte-identical across runs");
}

#[test]
fn block_json_is_byte_deterministic() {
    let artifacts = &[
        "fixtures/artifacts/shape_incompatible_partial.json",
        "fixtures/artifacts/rvl_refusal_missingness_tolerable.json",
        "fixtures/artifacts/verify_pass.json",
    ];
    let policy = "fixtures/policies/loan_tape_monthly_v1.yaml";

    let a = run_pipeline(artifacts, policy, true);
    let b = run_pipeline(artifacts, policy, true);
    assert_eq!(a, b, "BLOCK JSON must be byte-identical across runs");
}

// ---------------------------------------------------------------------------
// Determinism proof: refusal output
// ---------------------------------------------------------------------------

fn run_refusal(artifacts: &[&str], policy_flag: &str, policy_val: &str) -> String {
    let mut args: Vec<&str> = vec!["assess"];
    args.extend_from_slice(artifacts);
    args.push(policy_flag);
    args.push(policy_val);
    args.push("--json");
    args.push("--no-witness");
    execute(Cli::parse_from(args)).unwrap().stdout
}

#[test]
fn incomplete_basis_refusal_is_byte_deterministic() {
    let artifacts = &["fixtures/artifacts/verify_pass.json"];
    let a = run_refusal(artifacts, "--policy-id", "loan_tape.monthly.v1");
    let b = run_refusal(artifacts, "--policy-id", "loan_tape.monthly.v1");
    assert_eq!(
        a, b,
        "E_INCOMPLETE_BASIS refusal must be byte-identical across runs"
    );
}

#[test]
fn unknown_policy_refusal_is_byte_deterministic() {
    let artifacts = &["fixtures/artifacts/verify_pass.json"];
    let a = run_refusal(artifacts, "--policy-id", "nonexistent.policy.v99");
    let b = run_refusal(artifacts, "--policy-id", "nonexistent.policy.v99");
    assert_eq!(
        a, b,
        "E_UNKNOWN_POLICY refusal must be byte-identical across runs"
    );
}

// ---------------------------------------------------------------------------
// Structural determinism invariants
// ---------------------------------------------------------------------------

#[test]
fn output_path_sources_contain_no_hashmap() {
    let output_modules = &[
        "src/output/mod.rs",
        "src/output/json.rs",
        "src/output/human.rs",
        "src/refusal/payload.rs",
        "src/refusal/codes.rs",
        "src/bundle/artifact.rs",
    ];

    for module in output_modules {
        let source = std::fs::read_to_string(module)
            .unwrap_or_else(|_| panic!("{module} should be readable"));
        assert!(
            !source.contains("HashMap"),
            "R-005 violation: {module} must not use HashMap in output path (use BTreeMap)"
        );
    }
}

#[test]
fn main_contains_no_process_exit() {
    let source = std::fs::read_to_string("src/main.rs").expect("main.rs should be readable");
    assert!(
        !source.contains("process::exit("),
        "R-002 violation: main.rs must not use hardcoded process::exit()"
    );
}

#[test]
fn witness_ledger_is_append_only() {
    let source =
        std::fs::read_to_string("src/witness/ledger.rs").expect("ledger.rs should be readable");
    assert!(
        !source.contains("File::create("),
        "R-003 violation: witness ledger must be append-only (no File::create)"
    );
}
