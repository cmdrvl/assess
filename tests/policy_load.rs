mod support;

use assess::policy::{DecisionBand, PolicyFile};

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

#[test]
fn loan_tape_policy_deserializes_from_yaml() -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(support::policy_path("loan_tape_monthly_v1.yaml"))?;
    let policy: PolicyFile = serde_yaml::from_str(&raw)?;

    assert_eq!(policy.schema_version, 1);
    assert_eq!(policy.policy_id, "loan_tape.monthly.v1");
    assert_eq!(policy.policy_version, 1);
    assert_eq!(policy.requires, vec!["shape", "rvl", "verify"]);
    assert_eq!(policy.rules.len(), 6);
    Ok(())
}

#[test]
fn loan_tape_clean_reconciliation_rule_uses_tool_keyed_when()
-> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(support::policy_path("loan_tape_monthly_v1.yaml"))?;
    let policy: PolicyFile = serde_yaml::from_str(&raw)?;

    let clean = &policy.rules[0];
    assert_eq!(clean.name, "clean_reconciliation");
    assert!(!clean.default);

    let when = clean
        .when
        .as_ref()
        .expect("clean_reconciliation must have a when clause");
    assert_eq!(when.len(), 3);

    let shape = when.get("shape").expect("when should have shape");
    assert_eq!(shape.outcome.as_deref(), Some("COMPATIBLE"));
    assert!(shape.outcome_in.is_none());

    let rvl = when.get("rvl").expect("when should have rvl");
    assert!(rvl.outcome.is_none());
    assert_eq!(
        rvl.outcome_in.as_deref(),
        Some(vec!["REAL_CHANGE".to_owned(), "NO_REAL_CHANGE".to_owned()].as_slice())
    );

    let verify = when.get("verify").expect("when should have verify");
    assert_eq!(verify.outcome.as_deref(), Some("PASS"));

    assert_eq!(clean.then.decision_band, DecisionBand::Proceed);
    assert!(clean.then.risk_code.is_none());
    Ok(())
}

#[test]
fn loan_tape_diffuse_rule_matches_refusal_code() -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(support::policy_path("loan_tape_monthly_v1.yaml"))?;
    let policy: PolicyFile = serde_yaml::from_str(&raw)?;

    let diffuse = &policy.rules[2];
    assert_eq!(diffuse.name, "diffuse_requires_review");

    let when = diffuse.when.as_ref().expect("diffuse rule must have when");
    let rvl = when.get("rvl").expect("when should have rvl");
    assert_eq!(rvl.refusal.as_deref(), Some("E_DIFFUSE"));

    assert_eq!(diffuse.then.decision_band, DecisionBand::Escalate);
    assert_eq!(diffuse.then.risk_code.as_deref(), Some("DIFFUSE_CHANGE"));
    Ok(())
}

#[test]
fn loan_tape_tolerable_missingness_rule_uses_signals() -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(support::policy_path("loan_tape_monthly_v1.yaml"))?;
    let policy: PolicyFile = serde_yaml::from_str(&raw)?;

    let tolerable = &policy.rules[3];
    assert_eq!(tolerable.name, "tolerable_missingness");

    let when = tolerable
        .when
        .as_ref()
        .expect("tolerable rule must have when");
    let rvl = when.get("rvl").expect("when should have rvl");
    assert_eq!(rvl.refusal.as_deref(), Some("E_MISSINGNESS"));
    assert_eq!(
        rvl.signals.get("missingness_band"),
        Some(&serde_json::json!("TOLERABLE"))
    );

    assert_eq!(tolerable.then.decision_band, DecisionBand::ProceedWithRisk);
    assert_eq!(
        tolerable.then.risk_code.as_deref(),
        Some("MISSINGNESS_TOLERATED")
    );
    Ok(())
}

#[test]
fn loan_tape_default_rule_is_last_and_marked_default() -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(support::policy_path("loan_tape_monthly_v1.yaml"))?;
    let policy: PolicyFile = serde_yaml::from_str(&raw)?;

    let last = policy.rules.last().expect("policy must have rules");
    assert_eq!(last.name, "default_block");
    assert!(last.default);
    assert!(last.when.is_none());
    assert_eq!(last.then.decision_band, DecisionBand::Block);
    assert_eq!(last.then.risk_code.as_deref(), Some("UNHANDLED_CONDITION"));

    // No other rule should be marked default
    for rule in &policy.rules[..policy.rules.len() - 1] {
        assert!(!rule.default, "rule {} should not be default", rule.name);
    }
    Ok(())
}

#[test]
fn minimal_default_policy_deserializes() -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(support::policy_path("minimal_default_only.yaml"))?;
    let policy: PolicyFile = serde_yaml::from_str(&raw)?;

    assert_eq!(policy.schema_version, 1);
    assert_eq!(policy.policy_id, "default.v0");
    assert_eq!(policy.rules.len(), 1);
    assert!(policy.requires.is_empty());

    let rule = &policy.rules[0];
    assert!(rule.default);
    assert!(rule.when.is_none());
    assert_eq!(rule.then.decision_band, DecisionBand::Block);
    Ok(())
}

#[test]
fn non_proceed_rules_require_risk_code() -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(support::policy_path("loan_tape_monthly_v1.yaml"))?;
    let policy: PolicyFile = serde_yaml::from_str(&raw)?;

    for rule in &policy.rules {
        match rule.then.decision_band {
            DecisionBand::Proceed => {
                // PROCEED rules may omit risk_code (I08)
            }
            _ => {
                assert!(
                    rule.then.risk_code.is_some(),
                    "non-PROCEED rule {} must have risk_code (I08)",
                    rule.name
                );
            }
        }
    }
    Ok(())
}
