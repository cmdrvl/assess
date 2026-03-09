use crate::policy::DecisionBand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessExit {
    Proceed,
    Attention,
    Stop,
}

impl AssessExit {
    pub const fn code(self) -> u8 {
        match self {
            Self::Proceed => 0,
            Self::Attention => 1,
            Self::Stop => 2,
        }
    }

    pub const fn from_decision_band(decision_band: DecisionBand) -> Self {
        match decision_band {
            DecisionBand::Proceed => Self::Proceed,
            DecisionBand::ProceedWithRisk | DecisionBand::Escalate => Self::Attention,
            DecisionBand::Block => Self::Stop,
        }
    }
}
