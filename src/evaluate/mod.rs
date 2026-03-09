pub mod matcher;

use thiserror::Error;

use crate::bundle::ArtifactBundle;
use crate::policy::{DecisionBand, PolicyFile};
use crate::refusal::RefusalCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub decision_band: DecisionBand,
    pub matched_rule: String,
    pub risk_code: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EvalError {
    #[error("incomplete basis: missing tools {missing:?}")]
    IncompleteBasis { missing: Vec<String> },
    #[error("no rule matched and no default rule exists")]
    NoMatchingRule,
}

impl EvalError {
    pub const fn refusal_code(&self) -> RefusalCode {
        match self {
            Self::IncompleteBasis { .. } => RefusalCode::IncompleteBasis,
            Self::NoMatchingRule => RefusalCode::MissingRule,
        }
    }
}

/// Evaluate a policy against an artifact bundle.
///
/// 1. Checks that all tools in `policy.requires` are present in the bundle.
/// 2. Evaluates rules in declaration order; first match wins.
/// 3. Default rule always matches when reached.
/// 4. Returns `EvalError::NoMatchingRule` if nothing matches.
pub fn evaluate(policy: &PolicyFile, bundle: &ArtifactBundle) -> Result<Decision, EvalError> {
    check_requires(policy, bundle)?;

    let observed = bundle.observed_tools();
    for rule in &policy.rules {
        if rule.default {
            return Ok(Decision {
                decision_band: rule.then.decision_band,
                matched_rule: rule.name.clone(),
                risk_code: rule.then.risk_code.clone(),
            });
        }

        let Some(when) = &rule.when else {
            continue;
        };

        if matcher::matches_bundle(when, bundle, &observed) {
            return Ok(Decision {
                decision_band: rule.then.decision_band,
                matched_rule: rule.name.clone(),
                risk_code: rule.then.risk_code.clone(),
            });
        }
    }

    Err(EvalError::NoMatchingRule)
}

fn check_requires(policy: &PolicyFile, bundle: &ArtifactBundle) -> Result<(), EvalError> {
    let observed = bundle.observed_tools();
    let missing: Vec<String> = policy
        .requires
        .iter()
        .filter(|tool| !observed.contains(tool))
        .cloned()
        .collect();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(EvalError::IncompleteBasis { missing })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::Value;

    use super::*;
    use crate::policy::{Rule, ThenClause, ToolMatcher};

    fn make_policy(requires: Vec<&str>, rules: Vec<Rule>) -> PolicyFile {
        PolicyFile {
            schema_version: 1,
            policy_id: "test.v0".to_owned(),
            policy_version: 1,
            description: None,
            requires: requires.into_iter().map(String::from).collect(),
            rules,
        }
    }

    fn proceed_rule(name: &str, when: BTreeMap<String, ToolMatcher>) -> Rule {
        Rule {
            name: name.to_owned(),
            default: false,
            when: Some(when),
            then: ThenClause {
                decision_band: DecisionBand::Proceed,
                risk_code: None,
            },
        }
    }

    fn default_block() -> Rule {
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

    fn write_artifact(dir: &std::path::Path, name: &str, json: &Value) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, serde_json::to_string_pretty(json).unwrap()).unwrap();
        path
    }

    fn load_bundle(paths: &[std::path::PathBuf]) -> ArtifactBundle {
        crate::bundle::load(paths).expect("bundle should load")
    }

    #[test]
    fn incomplete_basis_returns_error() {
        let dir =
            std::env::temp_dir().join(format!("assess-eval-{}-incomplete", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let verify = write_artifact(
            &dir,
            "verify.json",
            &serde_json::json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
        );

        let bundle = load_bundle(&[verify]);
        let policy = make_policy(vec!["verify", "shape"], vec![default_block()]);

        let err = evaluate(&policy, &bundle).unwrap_err();
        assert_eq!(err.refusal_code(), RefusalCode::IncompleteBasis);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn first_matching_rule_wins() {
        let dir = std::env::temp_dir().join(format!("assess-eval-{}-first", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let verify = write_artifact(
            &dir,
            "verify.json",
            &serde_json::json!({"tool": "verify", "version": "verify.report.v1", "outcome": "PASS"}),
        );

        let bundle = load_bundle(&[verify]);

        let rule1 = proceed_rule(
            "clean",
            BTreeMap::from([(
                "verify".to_owned(),
                ToolMatcher {
                    outcome: Some("PASS".to_owned()),
                    outcome_in: None,
                    refusal: None,
                    signals: BTreeMap::<String, Value>::new(),
                },
            )]),
        );

        let policy = make_policy(vec!["verify"], vec![rule1, default_block()]);
        let decision = evaluate(&policy, &bundle).unwrap();
        assert_eq!(decision.decision_band, DecisionBand::Proceed);
        assert_eq!(decision.matched_rule, "clean");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn default_rule_catches_unmatched() {
        let dir = std::env::temp_dir().join(format!("assess-eval-{}-default", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let verify = write_artifact(
            &dir,
            "verify.json",
            &serde_json::json!({"tool": "verify", "version": "verify.report.v1", "outcome": "FAIL"}),
        );

        let bundle = load_bundle(&[verify]);

        let rule1 = proceed_rule(
            "clean",
            BTreeMap::from([(
                "verify".to_owned(),
                ToolMatcher {
                    outcome: Some("PASS".to_owned()),
                    outcome_in: None,
                    refusal: None,
                    signals: BTreeMap::<String, Value>::new(),
                },
            )]),
        );

        let policy = make_policy(vec!["verify"], vec![rule1, default_block()]);
        let decision = evaluate(&policy, &bundle).unwrap();
        assert_eq!(decision.decision_band, DecisionBand::Block);
        assert_eq!(decision.matched_rule, "default_block");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn no_default_no_match_returns_error() {
        let dir = std::env::temp_dir().join(format!("assess-eval-{}-nomatch", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let verify = write_artifact(
            &dir,
            "verify.json",
            &serde_json::json!({"tool": "verify", "version": "verify.report.v1", "outcome": "FAIL"}),
        );

        let bundle = load_bundle(&[verify]);

        let rule1 = proceed_rule(
            "clean",
            BTreeMap::from([(
                "verify".to_owned(),
                ToolMatcher {
                    outcome: Some("PASS".to_owned()),
                    outcome_in: None,
                    refusal: None,
                    signals: BTreeMap::<String, Value>::new(),
                },
            )]),
        );

        let policy = make_policy(vec!["verify"], vec![rule1]);
        let err = evaluate(&policy, &bundle).unwrap_err();
        assert_eq!(err.refusal_code(), RefusalCode::MissingRule);

        std::fs::remove_dir_all(&dir).ok();
    }
}
