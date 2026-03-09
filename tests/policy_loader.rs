mod support;

use assess::policy::{
    PolicyError, PolicySearchPaths, PolicySource, load_path, load_policy_id_with,
};
use assess::refusal::RefusalCode;
use sha2::{Digest, Sha256};

#[test]
fn explicit_policy_path_loads_and_hashes_raw_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let path = support::policy_path("loan_tape_monthly_v1.yaml");
    let raw = std::fs::read(&path)?;
    let loaded = load_path(&path)?;

    assert_eq!(loaded.policy.policy_id, "loan_tape.monthly.v1");
    assert_eq!(
        loaded.sha256,
        format!("sha256:{:x}", Sha256::digest(raw.as_slice()))
    );
    assert_eq!(loaded.source, PolicySource::Path(path));
    Ok(())
}

#[test]
fn builtin_policy_resolution_works_without_external_search_paths()
-> Result<(), Box<dyn std::error::Error>> {
    let loaded = load_policy_id_with("default.v0", &PolicySearchPaths::default())?;

    assert_eq!(loaded.policy.policy_id, "default.v0");
    assert_eq!(loaded.source, PolicySource::Builtin("default.v0"));
    Ok(())
}

#[test]
fn env_search_paths_override_builtin_policies() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("policy-loader-env")?;
    let env_dir = workspace.child("env-policies");
    std::fs::create_dir_all(&env_dir)?;
    workspace.write(
        "env-policies/override.yaml",
        r#"schema_version: 1
policy_id: loan_tape.monthly.v1
policy_version: 9
description: env override policy
rules:
  - name: default_block
    default: true
    then:
      decision_band: BLOCK
      risk_code: OVERRIDE
"#,
    )?;

    let search_paths = PolicySearchPaths::new(vec![env_dir], None);
    let loaded = load_policy_id_with("loan_tape.monthly.v1", &search_paths)?;

    assert_eq!(loaded.policy.policy_version, 9);
    assert_eq!(
        loaded.policy.description.as_deref(),
        Some("env override policy")
    );
    assert!(matches!(loaded.source, PolicySource::SearchPath(_)));
    Ok(())
}

#[test]
fn user_policy_dir_is_checked_after_builtins() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = support::TempWorkspace::new("policy-loader-home")?;
    let home_dir = workspace.child("home");
    std::fs::create_dir_all(home_dir.join(".epistemic").join("policies"))?;
    workspace.write(
        "home/.epistemic/policies/custom.yaml",
        r#"schema_version: 1
policy_id: custom.policy.v1
policy_version: 3
rules:
  - name: default_block
    default: true
    then:
      decision_band: BLOCK
      risk_code: CUSTOM_HOME
"#,
    )?;

    let search_paths = PolicySearchPaths::new(Vec::new(), Some(home_dir));
    let loaded = load_policy_id_with("custom.policy.v1", &search_paths)?;

    assert_eq!(loaded.policy.policy_id, "custom.policy.v1");
    assert!(matches!(loaded.source, PolicySource::UserDir(_)));
    Ok(())
}

#[test]
fn unknown_policy_id_maps_to_unknown_policy_refusal() {
    let error = load_policy_id_with("missing.policy.v1", &PolicySearchPaths::default())
        .expect_err("unknown policy id should fail");

    assert!(matches!(
        error,
        PolicyError::NotFound { ref id } if id == "missing.policy.v1"
    ));
    assert_eq!(error.refusal_code(), RefusalCode::UnknownPolicy);
}

#[test]
fn invalid_policy_structure_is_rejected_on_explicit_load() -> Result<(), Box<dyn std::error::Error>>
{
    let workspace = support::TempWorkspace::new("policy-loader-invalid")?;
    let path = workspace.write(
        "invalid.yaml",
        r#"schema_version: 1
policy_id: invalid.policy.v1
policy_version: 1
rules:
  - name: default_block
    default: true
    then:
      decision_band: BLOCK
      risk_code: FIRST
  - name: another_rule
    then:
      decision_band: PROCEED
"#,
    )?;

    let error = load_path(&path).expect_err("default-not-last policy should fail");
    assert!(matches!(error, PolicyError::SchemaViolation(_)));
    assert_eq!(error.refusal_code(), RefusalCode::BadPolicy);
    assert_eq!(error.to_string(), "policy default rule must be last");
    Ok(())
}
