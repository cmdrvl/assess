use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefusalCode {
    #[serde(rename = "E_BAD_POLICY")]
    BadPolicy,
    #[serde(rename = "E_AMBIGUOUS_POLICY")]
    AmbiguousPolicy,
    #[serde(rename = "E_UNKNOWN_POLICY")]
    UnknownPolicy,
    #[serde(rename = "E_BAD_ARTIFACT")]
    BadArtifact,
    #[serde(rename = "E_DUPLICATE_TOOL")]
    DuplicateTool,
    #[serde(rename = "E_INCOMPLETE_BASIS")]
    IncompleteBasis,
    #[serde(rename = "E_MISSING_RULE")]
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

    pub const fn next_command(self) -> &'static str {
        match self {
            Self::BadPolicy => "assess <ARTIFACT>... --policy <PATH>",
            Self::AmbiguousPolicy => "assess <ARTIFACT>... --policy <PATH>",
            Self::UnknownPolicy => "assess <ARTIFACT>... --policy <PATH>",
            Self::BadArtifact => "assess <ARTIFACT>... --policy <PATH>",
            Self::DuplicateTool => "assess <ARTIFACT>... --policy <PATH>",
            Self::IncompleteBasis => "assess <ARTIFACT>... --policy <PATH>",
            Self::MissingRule => "assess <ARTIFACT>... --policy <PATH>",
        }
    }
}
