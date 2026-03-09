pub mod args;
pub mod exit;

use std::path::PathBuf;

use clap::{CommandFactory, error::ErrorKind};
use serde_json::json;

use crate::refusal::{RefusalCode, RefusalEnvelope};

pub use args::{
    Cli, Command, WitnessArgs, WitnessCommand, WitnessCount, WitnessLast, WitnessQuery,
};
pub use exit::AssessExit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Describe,
    Schema,
    Version,
    Run(RunCommand),
    Witness(WitnessInvocation),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCommand {
    pub artifacts: Vec<PathBuf>,
    pub policy_selector: PolicySelector,
    pub json: bool,
    pub no_witness: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicySelector {
    Path(String),
    Id(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessInvocation {
    pub command: WitnessInvocationCommand,
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessInvocationCommand {
    Query { filters: Vec<String> },
    Last,
    Count { filters: Vec<String> },
}

#[derive(Debug)]
pub enum RouteError {
    Usage(Box<clap::Error>),
    Refusal(Box<RefusalEnvelope>),
}

pub fn route(cli: Cli) -> Result<Route, RouteError> {
    let Cli {
        artifacts,
        policy,
        policy_id,
        json,
        no_witness,
        describe,
        schema,
        version,
        command,
    } = cli;

    if describe {
        return Ok(Route::Describe);
    }

    if schema {
        return Ok(Route::Schema);
    }

    if version {
        return Ok(Route::Version);
    }

    match command {
        Some(Command::Witness(witness)) => {
            route_witness(artifacts, policy, policy_id, no_witness, json, witness)
        }
        None => route_run(artifacts, policy, policy_id, json, no_witness),
    }
}

fn route_run(
    artifacts: Vec<PathBuf>,
    policy: Option<String>,
    policy_id: Option<String>,
    json: bool,
    no_witness: bool,
) -> Result<Route, RouteError> {
    if artifacts.is_empty() {
        return Err(RouteError::Usage(Box::new(missing_required_argument(
            "the following required arguments were not provided:\n  <ARTIFACT>...",
        ))));
    }

    let policy_selector = match (policy, policy_id) {
        (Some(policy), Some(policy_id)) => {
            return Err(RouteError::Refusal(Box::new(
                RefusalEnvelope::new(
                    RefusalCode::AmbiguousPolicy,
                    "ambiguous policy selector: provide either --policy or --policy-id, not both",
                )
                .with_detail(json!({
                    "policy": policy,
                    "policy_id": policy_id,
                })),
            )));
        }
        (Some(policy), None) => PolicySelector::Path(policy),
        (None, Some(policy_id)) => PolicySelector::Id(policy_id),
        (None, None) => {
            return Err(RouteError::Usage(Box::new(missing_required_argument(
                "the following required arguments were not provided:\n  --policy <POLICY>\n\nor:\n  --policy-id <POLICY_ID>",
            ))));
        }
    };

    Ok(Route::Run(RunCommand {
        artifacts,
        policy_selector,
        json,
        no_witness,
    }))
}

fn route_witness(
    artifacts: Vec<PathBuf>,
    policy: Option<String>,
    policy_id: Option<String>,
    no_witness: bool,
    json: bool,
    witness: WitnessArgs,
) -> Result<Route, RouteError> {
    if !artifacts.is_empty() {
        return Err(RouteError::Usage(Box::new(argument_conflict(
            "artifact arguments are not accepted with `assess witness`",
        ))));
    }

    if policy.is_some() || policy_id.is_some() {
        return Err(RouteError::Usage(Box::new(argument_conflict(
            "policy selectors are not accepted with `assess witness`",
        ))));
    }

    if no_witness {
        return Err(RouteError::Usage(Box::new(argument_conflict(
            "`--no-witness` cannot be used with `assess witness`",
        ))));
    }

    let command = match witness.command {
        WitnessCommand::Query(query) => WitnessInvocationCommand::Query {
            filters: query.filters,
        },
        WitnessCommand::Last(_) => WitnessInvocationCommand::Last,
        WitnessCommand::Count(count) => WitnessInvocationCommand::Count {
            filters: count.filters,
        },
    };

    Ok(Route::Witness(WitnessInvocation { command, json }))
}

fn missing_required_argument(message: &str) -> clap::Error {
    Cli::command().error(ErrorKind::MissingRequiredArgument, message)
}

fn argument_conflict(message: &str) -> clap::Error {
    Cli::command().error(ErrorKind::ArgumentConflict, message)
}
