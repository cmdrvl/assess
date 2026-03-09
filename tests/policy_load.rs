mod support;

#[test]
fn policy_fixtures_exist() -> Result<(), Box<dyn std::error::Error>> {
    let policy =
        std::fs::read_to_string(support::fixture_path("policies/loan_tape_monthly_v1.yaml"))?;
    let minimal =
        std::fs::read_to_string(support::fixture_path("policies/minimal_default_only.yaml"))?;

    assert!(policy.contains("policy_id: loan_tape.monthly.v1"));
    assert!(minimal.contains("default: true"));
    Ok(())
}
