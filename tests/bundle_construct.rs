use assess::bundle::derive::canonical_tool;

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
