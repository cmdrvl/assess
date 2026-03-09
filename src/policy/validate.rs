use super::{DecisionBand, PolicyError, PolicyFile, Rule};

pub fn default_rule_is_last(policy: &PolicyFile) -> bool {
    let Some(position) = policy.rules.iter().position(|rule| rule.default) else {
        return true;
    };

    position + 1 == policy.rules.len()
}

pub fn validate(policy: &PolicyFile) -> Result<(), PolicyError> {
    if policy.schema_version != 1 {
        return Err(schema_violation(format!(
            "policy schema_version must be 1, found {}",
            policy.schema_version
        )));
    }

    if policy.rules.is_empty() {
        return Err(schema_violation(
            "policy must define at least one rule".to_owned(),
        ));
    }

    if !default_rule_is_last(policy) {
        return Err(schema_violation(
            "policy default rule must be last".to_owned(),
        ));
    }

    for rule in &policy.rules {
        if rule.then.decision_band != DecisionBand::Proceed && !has_non_empty_risk_code(rule) {
            return Err(schema_violation(format!(
                "rule `{}` requires non-empty risk_code for {}",
                rule.name,
                rule.then.decision_band.as_str()
            )));
        }
    }

    Ok(())
}

fn has_non_empty_risk_code(rule: &Rule) -> bool {
    rule.then
        .risk_code
        .as_deref()
        .is_some_and(|risk_code| !risk_code.trim().is_empty())
}

fn schema_violation(message: String) -> PolicyError {
    PolicyError::SchemaViolation(message)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::Value;

    use super::*;
    use crate::policy::{ThenClause, ToolMatcher};

    #[test]
    fn accepts_valid_minimal_default_policy() {
        let policy = PolicyFile {
            schema_version: 1,
            policy_id: "loan_tape.monthly.v1".to_owned(),
            policy_version: 1,
            description: None,
            requires: vec!["shape".to_owned(), "rvl".to_owned(), "verify".to_owned()],
            rules: vec![
                Rule {
                    name: "clean_bundle".to_owned(),
                    default: false,
                    when: Some(BTreeMap::from([(
                        "verify".to_owned(),
                        ToolMatcher {
                            outcome: Some("PASS".to_owned()),
                            outcome_in: None,
                            refusal: None,
                            signals: BTreeMap::<String, Value>::new(),
                        },
                    )])),
                    then: ThenClause {
                        decision_band: DecisionBand::Proceed,
                        risk_code: None,
                    },
                },
                Rule {
                    name: "default_block".to_owned(),
                    default: true,
                    when: None,
                    then: ThenClause {
                        decision_band: DecisionBand::Block,
                        risk_code: Some("UNHANDLED_CONDITION".to_owned()),
                    },
                },
            ],
        };

        assert!(validate(&policy).is_ok());
    }

    #[test]
    fn rejects_non_v0_schema_version() {
        let policy = PolicyFile {
            schema_version: 2,
            policy_id: "loan_tape.monthly.v1".to_owned(),
            policy_version: 1,
            description: None,
            requires: Vec::new(),
            rules: vec![default_rule()],
        };

        let error = validate(&policy).expect_err("schema_version mismatch should fail");
        assert_eq!(
            error.to_string(),
            "policy schema_version must be 1, found 2"
        );
        assert_eq!(error.refusal_code(), crate::refusal::RefusalCode::BadPolicy);
    }

    #[test]
    fn rejects_empty_rule_lists() {
        let policy = PolicyFile {
            schema_version: 1,
            policy_id: "loan_tape.monthly.v1".to_owned(),
            policy_version: 1,
            description: None,
            requires: Vec::new(),
            rules: Vec::new(),
        };

        let error = validate(&policy).expect_err("empty rule list should fail");
        assert_eq!(error.to_string(), "policy must define at least one rule");
    }

    #[test]
    fn rejects_default_rule_when_not_last() {
        let policy = PolicyFile {
            schema_version: 1,
            policy_id: "loan_tape.monthly.v1".to_owned(),
            policy_version: 1,
            description: None,
            requires: Vec::new(),
            rules: vec![
                default_rule(),
                Rule {
                    name: "clean_bundle".to_owned(),
                    default: false,
                    when: None,
                    then: ThenClause {
                        decision_band: DecisionBand::Proceed,
                        risk_code: None,
                    },
                },
            ],
        };

        let error = validate(&policy).expect_err("default rule must be last");
        assert_eq!(error.to_string(), "policy default rule must be last");
    }

    #[test]
    fn rejects_non_proceed_rules_without_risk_code() {
        let policy = PolicyFile {
            schema_version: 1,
            policy_id: "loan_tape.monthly.v1".to_owned(),
            policy_version: 1,
            description: None,
            requires: Vec::new(),
            rules: vec![Rule {
                name: "default_block".to_owned(),
                default: true,
                when: None,
                then: ThenClause {
                    decision_band: DecisionBand::Block,
                    risk_code: None,
                },
            }],
        };

        let error = validate(&policy).expect_err("non-proceed rule must carry risk_code");
        assert_eq!(
            error.to_string(),
            "rule `default_block` requires non-empty risk_code for BLOCK"
        );
    }

    #[test]
    fn rejects_blank_risk_codes_for_non_proceed_rules() {
        let policy = PolicyFile {
            schema_version: 1,
            policy_id: "loan_tape.monthly.v1".to_owned(),
            policy_version: 1,
            description: None,
            requires: Vec::new(),
            rules: vec![Rule {
                name: "riskful_rule".to_owned(),
                default: true,
                when: None,
                then: ThenClause {
                    decision_band: DecisionBand::Escalate,
                    risk_code: Some("   ".to_owned()),
                },
            }],
        };

        let error = validate(&policy).expect_err("blank risk_code should fail");
        assert_eq!(
            error.to_string(),
            "rule `riskful_rule` requires non-empty risk_code for ESCALATE"
        );
    }

    fn default_rule() -> Rule {
        Rule {
            name: "default_block".to_owned(),
            default: true,
            when: None,
            then: ThenClause {
                decision_band: DecisionBand::Block,
                risk_code: Some("UNHANDLED_CONDITION".to_owned()),
            },
        }
    }
}
