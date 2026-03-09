use assess::{ASSESS_SCHEMA_JSON, POLICY_SCHEMA_JSON};

#[test]
fn embedded_schemas_are_present() {
    assert!(ASSESS_SCHEMA_JSON.contains("\"title\": \"assess.v0\""));
    assert!(POLICY_SCHEMA_JSON.contains("\"title\": \"policy.v0\""));
}
