use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefusalCode {
    BadPolicy,
    AmbiguousPolicy,
    UnknownPolicy,
    BadArtifact,
    DuplicateTool,
    IncompleteBasis,
    MissingRule,
}

impl RefusalCode {
    pub const ALL: [Self; 7] = [
        Self::BadPolicy,
        Self::AmbiguousPolicy,
        Self::UnknownPolicy,
        Self::BadArtifact,
        Self::DuplicateTool,
        Self::IncompleteBasis,
        Self::MissingRule,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BadPolicy => "E_BAD_POLICY",
            Self::AmbiguousPolicy => "E_AMBIGUOUS_POLICY",
            Self::UnknownPolicy => "E_UNKNOWN_POLICY",
            Self::BadArtifact => "E_BAD_ARTIFACT",
            Self::DuplicateTool => "E_DUPLICATE_TOOL",
            Self::IncompleteBasis => "E_INCOMPLETE_BASIS",
            Self::MissingRule => "E_MISSING_RULE",
        }
    }
}
