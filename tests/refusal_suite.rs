use assess::refusal::RefusalCode;

#[test]
fn refusal_code_set_matches_plan_count() {
    let labels: Vec<&str> = RefusalCode::ALL
        .into_iter()
        .map(RefusalCode::as_str)
        .collect();
    assert_eq!(labels.len(), 7);
    assert!(labels.contains(&"E_BAD_POLICY"));
    assert!(labels.contains(&"E_MISSING_RULE"));
}
