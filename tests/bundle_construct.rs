mod support;

use serde_json::json;

use assess::bundle::{BundleError, derive::canonical_tool, load};

#[test]
fn canonical_tool_prefers_explicit_tool_then_version_fallback() {
    assert_eq!(
        canonical_tool(Some("verify"), "verify.report.v1"),
        Some("verify".to_owned())
    );
    assert_eq!(
        canonical_tool(None, "benchmark.v0"),
        Some("benchmark".to_owned())
    );
}

#[test]
fn canonical_tool_rejects_malformed_explicit_or_fallback_values() {
    assert_eq!(canonical_tool(Some(""), "shape.v0"), None);
    assert_eq!(
        canonical_tool(Some("verify.report"), "verify.report.v1"),
        None
    );
    assert_eq!(canonical_tool(None, "verify.report.v1"), None);
    assert_eq!(canonical_tool(None, "shape.latest"), None);
}

#[test]
fn load_builds_full_basis_and_sorted_observed_tools() -> Result<(), Box<dyn std::error::Error>> {
    let bundle = load(&[
        support::artifact_path("verify_pass.json"),
        support::artifact_path("shape_compatible.json"),
        support::artifact_path("rvl_refusal_diffuse.json"),
    ])?;

    assert_eq!(
        bundle.observed_tools(),
        vec!["rvl".to_owned(), "shape".to_owned(), "verify".to_owned()]
    );
    assert_eq!(bundle.basis().len(), 3);

    let verify = bundle.get("verify").expect("verify artifact should exist");
    assert_eq!(verify.outcome.as_deref(), Some("PASS"));
    assert_eq!(
        verify.policy_signals.get("severity_band"),
        Some(&json!("CLEAN"))
    );

    let rvl = bundle
        .basis()
        .iter()
        .find(|entry| entry.tool == "rvl")
        .expect("rvl basis entry should exist");
    assert_eq!(
        rvl.artifact,
        support::artifact_path("rvl_refusal_diffuse.json")
            .display()
            .to_string()
    );
    assert_eq!(rvl.outcome, None);
    assert_eq!(
        rvl.refusal.as_ref().map(|refusal| refusal.code.as_str()),
        Some("E_DIFFUSE")
    );

    Ok(())
}

#[test]
fn load_rejects_duplicate_canonical_tool_identities() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("bundle-duplicate")?;
    let explicit = workspace.write_json(
        "verify-report.json",
        &json!({
            "tool": "verify",
            "version": "verify.report.v1",
            "outcome": "PASS",
            "policy_signals": {}
        }),
    )?;
    let fallback = workspace.write_json(
        "verify-fallback.json",
        &json!({
            "version": "verify.v0",
            "outcome": "PASS",
            "policy_signals": {}
        }),
    )?;

    let error = load(&[explicit, fallback]).expect_err("duplicate tool identities should fail");
    assert!(matches!(
        error,
        BundleError::DuplicateTool { ref tool, .. } if tool == "verify"
    ));
    assert_eq!(error.refusal_code().as_str(), "E_DUPLICATE_TOOL");
    Ok(())
}

#[test]
fn load_rejects_bad_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("bundle-bad-artifact")?;
    let bad = workspace.write_json(
        "bad-artifact.json",
        &json!({
            "tool": "",
            "version": "verify.report.v1",
            "outcome": "PASS"
        }),
    )?;

    let error = load(&[bad]).expect_err("malformed tool should fail");
    assert!(matches!(error, BundleError::BadArtifact { .. }));
    assert_eq!(error.refusal_code().as_str(), "E_BAD_ARTIFACT");
    Ok(())
}

#[test]
fn load_rejects_compiled_verify_constraints_with_specific_guidance()
-> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("bundle-verify-constraints")?;
    let constraints = workspace.write_json(
        "constraints.verify.json",
        &json!({
            "version": "verify.constraint.v1",
            "constraint_set_id": "example.not_null",
            "bindings": [
                {
                    "name": "input",
                    "kind": "relation",
                    "key_fields": ["loan_id"]
                }
            ],
            "rules": [
                {
                    "id": "INPUT_LOAN_ID_PRESENT",
                    "severity": "error",
                    "portability": "portable",
                    "check": {
                        "op": "not_null",
                        "binding": "input",
                        "columns": ["loan_id"]
                    }
                }
            ]
        }),
    )?;

    let error = load(std::slice::from_ref(&constraints))
        .expect_err("compiled constraints should not be accepted as assess evidence");
    assert!(matches!(
        error,
        BundleError::VerifyConstraintsArtifact { ref path, ref version }
            if path == &constraints.display().to_string() && version == "verify.constraint.v1"
    ));
    assert_eq!(error.refusal_code().as_str(), "E_BAD_ARTIFACT");
    assert_eq!(
        error.refusal_detail(),
        Some(json!({
            "artifact": constraints.display().to_string(),
            "received_version": "verify.constraint.v1",
            "expected_version": "verify.report.v1",
        }))
    );

    let next_command = error
        .refusal_next_command()
        .expect("compiled constraints should surface a next command");
    assert!(next_command.starts_with("verify run "));
    assert!(next_command.contains("constraints.verify.json"));
    assert!(next_command.contains("&& assess "));

    Ok(())
}
