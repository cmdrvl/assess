use assess::policy::DecisionBand;

#[test]
fn decision_band_strings_match_plan_contract() {
    assert_eq!(DecisionBand::Proceed.as_str(), "PROCEED");
    assert_eq!(DecisionBand::ProceedWithRisk.as_str(), "PROCEED_WITH_RISK");
    assert_eq!(DecisionBand::Escalate.as_str(), "ESCALATE");
    assert_eq!(DecisionBand::Block.as_str(), "BLOCK");
}
