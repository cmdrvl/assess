use crate::policy::DecisionBand;

/// Exit-code contract (spine-rules `docs/exit-codes.md`):
///
/// | band / outcome        | code | domain   |
/// |-----------------------|------|----------|
/// | PROCEED               | 0    | positive |
/// | PROCEED_WITH_RISK     | 0    | positive |
/// | ESCALATE              | 1    | negative |
/// | REFUSAL / CLI error   | 2    | error    |
/// | BLOCK                 | 3    | policy   |
///
/// BLOCK is a valid, deterministic policy verdict and must never share a code
/// with a refusal or a malformed invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessExit {
    Proceed,
    Escalate,
    Refusal,
    Block,
}

impl AssessExit {
    pub const fn code(self) -> u8 {
        match self {
            Self::Proceed => 0,
            Self::Escalate => 1,
            Self::Refusal => 2,
            Self::Block => 3,
        }
    }

    pub const fn from_decision_band(decision_band: DecisionBand) -> Self {
        match decision_band {
            DecisionBand::Proceed | DecisionBand::ProceedWithRisk => Self::Proceed,
            DecisionBand::Escalate => Self::Escalate,
            DecisionBand::Block => Self::Block,
        }
    }
}
