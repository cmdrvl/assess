use crate::bundle::{ArtifactBundle, ArtifactReport};
use crate::policy::{PolicyFile, ToolMatcher, WhenClause};

pub fn has_default_rule(policy: &PolicyFile) -> bool {
    policy.rules.iter().any(|rule| rule.default)
}

/// Check whether a when-clause matches the given artifact bundle.
///
/// A when-clause is a map of tool names to match conditions. Every tool
/// referenced in the when-clause must:
/// 1. Be present in the bundle (observed_tools)
/// 2. Have its conditions satisfied by the corresponding artifact report
///
/// If any tool condition fails, the whole clause fails.
pub fn matches_bundle(when: &WhenClause, bundle: &ArtifactBundle, observed: &[String]) -> bool {
    for (tool_name, conditions) in when {
        if !observed.contains(tool_name) {
            return false;
        }

        let Some(report) = bundle.get(tool_name) else {
            return false;
        };

        if !matches_tool(conditions, report) {
            return false;
        }
    }

    true
}

/// Check whether a single tool's conditions match its artifact report.
fn matches_tool(conditions: &ToolMatcher, report: &ArtifactReport) -> bool {
    // Check outcome exact match
    if let Some(expected) = &conditions.outcome {
        match &report.outcome {
            Some(actual) if actual == expected => {}
            _ => return false,
        }
    }

    // Check outcome_in list match
    if let Some(expected_list) = &conditions.outcome_in {
        match &report.outcome {
            Some(actual) if expected_list.contains(actual) => {}
            _ => return false,
        }
    }

    // Check refusal code match
    if let Some(expected_code) = &conditions.refusal {
        match &report.refusal {
            Some(refusal) if &refusal.code == expected_code => {}
            _ => return false,
        }
    }

    // Check signals exact equality
    for (key, expected_value) in &conditions.signals {
        match report.policy_signals.get(key) {
            Some(actual_value) if actual_value == expected_value => {}
            _ => return false,
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::{Value, json};

    use crate::bundle::{ArtifactRefusal, ArtifactReport};
    use crate::policy::ToolMatcher;

    use super::*;

    fn make_report(outcome: Option<&str>) -> ArtifactReport {
        ArtifactReport {
            tool: Some("verify".to_owned()),
            version: "verify.report.v1".to_owned(),
            outcome: outcome.map(str::to_owned),
            refusal: None,
            policy_signals: BTreeMap::new(),
        }
    }

    fn make_conditions() -> ToolMatcher {
        ToolMatcher {
            outcome: None,
            outcome_in: None,
            refusal: None,
            signals: BTreeMap::new(),
        }
    }

    #[test]
    fn outcome_exact_match() {
        let report = make_report(Some("PASS"));
        let mut conditions = make_conditions();
        conditions.outcome = Some("PASS".to_owned());
        assert!(matches_tool(&conditions, &report));
    }

    #[test]
    fn outcome_exact_mismatch() {
        let report = make_report(Some("FAIL"));
        let mut conditions = make_conditions();
        conditions.outcome = Some("PASS".to_owned());
        assert!(!matches_tool(&conditions, &report));
    }

    #[test]
    fn outcome_in_match() {
        let report = make_report(Some("REAL_CHANGE"));
        let mut conditions = make_conditions();
        conditions.outcome_in = Some(vec!["REAL_CHANGE".to_owned(), "NO_REAL_CHANGE".to_owned()]);
        assert!(matches_tool(&conditions, &report));
    }

    #[test]
    fn outcome_in_mismatch() {
        let report = make_report(Some("UNKNOWN"));
        let mut conditions = make_conditions();
        conditions.outcome_in = Some(vec!["REAL_CHANGE".to_owned(), "NO_REAL_CHANGE".to_owned()]);
        assert!(!matches_tool(&conditions, &report));
    }

    #[test]
    fn refusal_code_match() {
        let mut report = make_report(None);
        report.refusal = Some(ArtifactRefusal {
            code: "E_DIFFUSE".to_owned(),
            message: None,
        });

        let mut conditions = make_conditions();
        conditions.refusal = Some("E_DIFFUSE".to_owned());
        assert!(matches_tool(&conditions, &report));
    }

    #[test]
    fn refusal_code_mismatch() {
        let mut report = make_report(None);
        report.refusal = Some(ArtifactRefusal {
            code: "E_DIFFUSE".to_owned(),
            message: None,
        });

        let mut conditions = make_conditions();
        conditions.refusal = Some("E_MISSINGNESS".to_owned());
        assert!(!matches_tool(&conditions, &report));
    }

    #[test]
    fn refusal_expected_but_absent() {
        let report = make_report(Some("PASS"));
        let mut conditions = make_conditions();
        conditions.refusal = Some("E_DIFFUSE".to_owned());
        assert!(!matches_tool(&conditions, &report));
    }

    #[test]
    fn signals_exact_match() {
        let mut report = make_report(Some("COMPATIBLE"));
        report
            .policy_signals
            .insert("compatibility_band".to_owned(), json!("FULL"));

        let mut conditions = make_conditions();
        conditions.outcome = Some("COMPATIBLE".to_owned());
        conditions
            .signals
            .insert("compatibility_band".to_owned(), json!("FULL"));
        assert!(matches_tool(&conditions, &report));
    }

    #[test]
    fn signals_value_mismatch() {
        let mut report = make_report(Some("COMPATIBLE"));
        report
            .policy_signals
            .insert("compatibility_band".to_owned(), json!("PARTIAL"));

        let mut conditions = make_conditions();
        conditions
            .signals
            .insert("compatibility_band".to_owned(), json!("FULL"));
        assert!(!matches_tool(&conditions, &report));
    }

    #[test]
    fn signals_key_missing() {
        let report = make_report(Some("COMPATIBLE"));
        let mut conditions = make_conditions();
        conditions
            .signals
            .insert("nonexistent_key".to_owned(), json!("something"));
        assert!(!matches_tool(&conditions, &report));
    }

    #[test]
    fn empty_conditions_always_match() {
        let report = make_report(Some("PASS"));
        let conditions = make_conditions();
        assert!(matches_tool(&conditions, &report));
    }

    #[test]
    fn combined_outcome_and_refusal_and_signals() {
        let report = ArtifactReport {
            tool: Some("rvl".to_owned()),
            version: "rvl.v0".to_owned(),
            outcome: None,
            refusal: Some(ArtifactRefusal {
                code: "E_MISSINGNESS".to_owned(),
                message: None,
            }),
            policy_signals: BTreeMap::from([("missingness_band".to_owned(), json!("TOLERABLE"))]),
        };

        let conditions = ToolMatcher {
            outcome: None,
            outcome_in: None,
            refusal: Some("E_MISSINGNESS".to_owned()),
            signals: BTreeMap::from([("missingness_band".to_owned(), json!("TOLERABLE"))]),
        };
        assert!(matches_tool(&conditions, &report));
    }

    #[test]
    fn when_clause_with_tool_not_in_bundle() {
        let when: WhenClause = BTreeMap::from([(
            "shape".to_owned(),
            ToolMatcher {
                outcome: Some("COMPATIBLE".to_owned()),
                outcome_in: None,
                refusal: None,
                signals: BTreeMap::<String, Value>::new(),
            },
        )]);

        let bundle = {
            let dir =
                std::env::temp_dir().join(format!("assess-matcher-{}-notools", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();

            let path = dir.join("verify.json");
            std::fs::write(
                &path,
                serde_json::to_string(&json!({
                    "tool": "verify",
                    "version": "verify.report.v1",
                    "outcome": "PASS"
                }))
                .unwrap(),
            )
            .unwrap();

            let bundle = crate::bundle::load(&[path]).unwrap();
            std::fs::remove_dir_all(&dir).ok();
            bundle
        };

        let observed = bundle.observed_tools();
        assert!(!matches_bundle(&when, &bundle, &observed));
    }
}
