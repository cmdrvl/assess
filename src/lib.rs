#![forbid(unsafe_code)]

pub mod bundle;
pub mod cli;
pub mod evaluate;
pub mod output;
pub mod policy;
pub mod refusal;
pub mod witness;

use cli::{Cli, Command};
use thiserror::Error;

pub const TOOL: &str = "assess";
pub const VERSION: &str = "assess.v0";
pub const OPERATOR_JSON: &str = include_str!("../operator.json");
pub const ASSESS_SCHEMA_JSON: &str = include_str!("../schemas/assess.v0.schema.json");
pub const POLICY_SCHEMA_JSON: &str = include_str!("../schemas/policy.v0.schema.json");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Execution {
    pub exit_code: u8,
    pub stdout: String,
}

#[derive(Debug, Error)]
pub enum AssessError {
    #[error("{0}")]
    Scaffold(&'static str),
}

pub fn execute(cli: Cli) -> Result<Execution, AssessError> {
    if cli.describe {
        return Ok(Execution {
            exit_code: 0,
            stdout: with_trailing_newline(OPERATOR_JSON),
        });
    }

    if cli.schema {
        return Ok(Execution {
            exit_code: 0,
            stdout: with_trailing_newline(ASSESS_SCHEMA_JSON),
        });
    }

    if cli.version {
        return Ok(Execution {
            exit_code: 0,
            stdout: format!("assess {}\n", env!("CARGO_PKG_VERSION")),
        });
    }

    match cli.command {
        Some(Command::Witness(_)) => Err(AssessError::Scaffold(
            "assess scaffold: witness execution is not implemented yet",
        )),
        None => Err(AssessError::Scaffold(
            "assess scaffold: decision execution is not implemented yet",
        )),
    }
}

fn with_trailing_newline(value: &str) -> String {
    if value.ends_with('\n') {
        value.to_owned()
    } else {
        format!("{value}\n")
    }
}
