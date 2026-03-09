pub mod matcher;

use crate::policy::DecisionBand;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub decision_band: DecisionBand,
    pub matched_rule: Option<String>,
    pub risk_code: Option<String>,
}
