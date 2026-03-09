mod support;

#[test]
fn golden_rule_files_exist() {
    assert!(support::rule_path("exit-code-range.yml").exists());
    assert!(support::rule_path("no-hashmap-in-output.yml").exists());
    assert!(support::rule_path("witness-must-append.yml").exists());
}
