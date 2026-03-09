pub mod loader;
pub mod schema;
pub mod validate;

use std::path::PathBuf;

use thiserror::Error;

use crate::refusal::RefusalCode;

pub use loader::{
    LoadedPolicy, PolicySearchPaths, PolicySource, load_path, load_policy_id, load_policy_id_with,
    policy_sha256, resolution_order,
};
pub use schema::{DecisionBand, PolicyFile, Rule, ThenClause, ToolMatcher, WhenClause};
pub use validate::{default_rule_is_last, validate};

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("failed to read policy `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse policy YAML from `{location}`: {source}")]
    YamlParse {
        location: String,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("{0}")]
    SchemaViolation(String),

    #[error(
        "policy `{id}` was not found in ASSESS_POLICY_PATH, builtin policies, or ~/.epistemic/policies"
    )]
    NotFound { id: String },

    #[error("ambiguous policy selector: --policy and --policy-id are mutually exclusive")]
    AmbiguousSelector,
}

impl PolicyError {
    pub const fn refusal_code(&self) -> RefusalCode {
        match self {
            Self::Io { .. } | Self::YamlParse { .. } | Self::SchemaViolation(_) => {
                RefusalCode::BadPolicy
            }
            Self::NotFound { .. } => RefusalCode::UnknownPolicy,
            Self::AmbiguousSelector => RefusalCode::AmbiguousPolicy,
        }
    }
}

/// Load and validate a policy from file path.
pub fn load_and_validate(path: &std::path::Path) -> Result<LoadedPolicy, PolicyError> {
    loader::load_path(path)
}

/// Load and validate a policy by ID.
pub fn load_and_validate_by_id(id: &str) -> Result<LoadedPolicy, PolicyError> {
    loader::load_policy_id(id)
}
