pub mod args;
pub mod exit;

use std::path::PathBuf;

use clap::{CommandFactory, error::ErrorKind};
use serde_json::json;

use crate::output::{RenderMode, WitnessStatus};
use crate::refusal::{RefusalCode, RefusalEnvelope};

pub use args::{
    CapabilitiesArgs, Cli, Command, DoctorArgs, DoctorCommand, RobotDocsArgs, RobotDocsCommand,
    RunRenderMode, WitnessArgs, WitnessCommand, WitnessCount, WitnessLast, WitnessQuery,
};
pub use exit::AssessExit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Help,
    Describe,
    Schema,
    Version,
    Doctor(DoctorInvocation),
    Run(RunCommand),
    Witness(WitnessInvocation),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCommand {
    pub artifacts: Vec<PathBuf>,
    pub policy_selector: PolicySelector,
    pub render_mode: RenderMode,
    pub no_witness: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicySelector {
    Path(String),
    Id(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorInvocation {
    pub command: DoctorInvocationCommand,
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoctorInvocationCommand {
    Health,
    /// Top-level `capabilities`: the operator.v1 document (same bytes as `--describe`).
    Capabilities,
    /// `doctor capabilities`: cmdrvl.doctor.v1 report embedding that document.
    DoctorCapabilities,
    RobotDocs,
    RobotTriage,
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
    Refusal {
        refusal: Box<RefusalEnvelope>,
        render_mode: RenderMode,
        witness_status: WitnessStatus,
    },
}

pub fn route(cli: Cli) -> Result<Route, RouteError> {
    let Cli {
        artifacts,
        policy,
        policy_id,
        json,
        render,
        no_witness,
        out: _,
        describe,
        schema,
        version,
        robot_triage,
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

    if robot_triage {
        reject_read_only_alias_args(&artifacts, &policy, &policy_id, no_witness, render)?;
        return Ok(Route::Doctor(DoctorInvocation {
            command: DoctorInvocationCommand::RobotTriage,
            json: true,
        }));
    }

    match command {
        Some(Command::Capabilities(capabilities)) => route_capabilities(
            artifacts,
            policy,
            policy_id,
            no_witness,
            json,
            render,
            capabilities,
        ),
        Some(Command::RobotDocs(robot_docs)) => route_robot_docs(
            artifacts, policy, policy_id, no_witness, json, render, robot_docs,
        ),
        Some(Command::Doctor(doctor)) => route_doctor(
            artifacts, policy, policy_id, no_witness, json, render, doctor,
        ),
        Some(Command::Witness(witness)) => route_witness(
            artifacts, policy, policy_id, no_witness, json, render, witness,
        ),
        None => route_run(artifacts, policy, policy_id, json, render, no_witness),
    }
}

fn reject_read_only_alias_args(
    artifacts: &[PathBuf],
    policy: &Option<String>,
    policy_id: &Option<String>,
    no_witness: bool,
    render: Option<RunRenderMode>,
) -> Result<(), RouteError> {
    if !artifacts.is_empty() {
        return Err(RouteError::Usage(Box::new(argument_conflict(
            "artifact arguments are not accepted with read-only assess introspection commands",
        ))));
    }

    if policy.is_some() || policy_id.is_some() {
        return Err(RouteError::Usage(Box::new(argument_conflict(
            "policy selectors are not accepted with read-only assess introspection commands",
        ))));
    }

    if no_witness {
        return Err(RouteError::Usage(Box::new(argument_conflict(
            "`--no-witness` cannot be used with read-only assess introspection commands",
        ))));
    }

    if render.is_some() {
        return Err(RouteError::Usage(Box::new(argument_conflict(
            "`--render` is not supported with read-only assess introspection commands",
        ))));
    }

    Ok(())
}

fn route_capabilities(
    artifacts: Vec<PathBuf>,
    policy: Option<String>,
    policy_id: Option<String>,
    no_witness: bool,
    json: bool,
    render: Option<RunRenderMode>,
    _capabilities: CapabilitiesArgs,
) -> Result<Route, RouteError> {
    reject_read_only_alias_args(&artifacts, &policy, &policy_id, no_witness, render)?;
    Ok(Route::Doctor(DoctorInvocation {
        command: DoctorInvocationCommand::Capabilities,
        json,
    }))
}

fn route_robot_docs(
    artifacts: Vec<PathBuf>,
    policy: Option<String>,
    policy_id: Option<String>,
    no_witness: bool,
    json: bool,
    render: Option<RunRenderMode>,
    robot_docs: RobotDocsArgs,
) -> Result<Route, RouteError> {
    reject_read_only_alias_args(&artifacts, &policy, &policy_id, no_witness, render)?;
    match robot_docs.command {
        None | Some(RobotDocsCommand::Guide) => Ok(Route::Doctor(DoctorInvocation {
            command: DoctorInvocationCommand::RobotDocs,
            json,
        })),
    }
}

fn route_doctor(
    artifacts: Vec<PathBuf>,
    policy: Option<String>,
    policy_id: Option<String>,
    no_witness: bool,
    json: bool,
    render: Option<RunRenderMode>,
    doctor: DoctorArgs,
) -> Result<Route, RouteError> {
    reject_read_only_alias_args(&artifacts, &policy, &policy_id, no_witness, render)?;

    let command = if doctor.robot_triage {
        DoctorInvocationCommand::RobotTriage
    } else {
        match doctor.command.unwrap_or(DoctorCommand::Health) {
            DoctorCommand::Health => DoctorInvocationCommand::Health,
            DoctorCommand::Capabilities => DoctorInvocationCommand::DoctorCapabilities,
            DoctorCommand::Triage => DoctorInvocationCommand::RobotTriage,
            DoctorCommand::RobotDocs => DoctorInvocationCommand::RobotDocs,
        }
    };

    Ok(Route::Doctor(DoctorInvocation { command, json }))
}

fn route_run(
    artifacts: Vec<PathBuf>,
    policy: Option<String>,
    policy_id: Option<String>,
    json: bool,
    render: Option<RunRenderMode>,
    no_witness: bool,
) -> Result<Route, RouteError> {
    if artifacts.is_empty() && policy.is_none() && policy_id.is_none() {
        if no_witness {
            return Err(RouteError::Usage(Box::new(argument_conflict(
                "`--no-witness` requires an assess decision run",
            ))));
        }

        if render.is_some() {
            return Err(RouteError::Usage(Box::new(argument_conflict(
                "`--render` requires an assess decision run",
            ))));
        }

        if json {
            return Ok(Route::Doctor(DoctorInvocation {
                command: DoctorInvocationCommand::Capabilities,
                json: true,
            }));
        }

        return Ok(Route::Help);
    }

    if artifacts.is_empty() {
        return Err(RouteError::Usage(Box::new(missing_required_argument(
            "the following required arguments were not provided:\n  <ARTIFACT>...",
        ))));
    }

    let render_mode = render_mode(json, render);
    let witness_status = refusal_witness_status(no_witness);
    let policy_selector = match (policy, policy_id) {
        (Some(policy), Some(policy_id)) => {
            return Err(RouteError::Refusal {
                refusal: Box::new(
                    RefusalEnvelope::new(
                    RefusalCode::AmbiguousPolicy,
                    "ambiguous policy selector: provide either --policy or --policy-id, not both",
                )
                    .with_detail(json!({
                        "policy": policy,
                        "policy_id": policy_id,
                    })),
                ),
                render_mode,
                witness_status,
            });
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
        render_mode,
        no_witness,
    }))
}

fn route_witness(
    artifacts: Vec<PathBuf>,
    policy: Option<String>,
    policy_id: Option<String>,
    no_witness: bool,
    json: bool,
    render: Option<RunRenderMode>,
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

    if render.is_some() {
        return Err(RouteError::Usage(Box::new(argument_conflict(
            "`--render` is not supported with `assess witness`",
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

pub fn long_help() -> String {
    let mut command = Cli::command();
    command.render_long_help().to_string()
}

pub fn format_cli_error(error: &clap::Error) -> String {
    let mut rendered = error.to_string();
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    if let Some(candidate) = did_you_mean(error) {
        rendered.push_str(&format!("hint: did you mean `{candidate}`?\n"));
    }
    match error.kind() {
        ErrorKind::UnknownArgument
        | ErrorKind::InvalidSubcommand
        | ErrorKind::MissingRequiredArgument
        | ErrorKind::ArgumentConflict => {
            rendered.push_str("next: assess capabilities --json\nhelp: assess robot-docs guide\n")
        }
        _ => {}
    }

    rendered
}

/// Optimal-string-alignment distance: insert, delete, substitute or swap two
/// adjacent characters each cost 1 (so `--jsno` is one edit from `--json`).
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    d[0] = (0..=b.len()).collect();
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

/// The one candidate exactly one edit away from an unknown long flag or
/// subcommand name, drawn from the command the user was actually inside.
/// Two or more equally close candidates (or none) yield no hint: never guess.
fn did_you_mean(error: &clap::Error) -> Option<String> {
    use clap::error::{ContextKind, ContextValue};

    let context = match error.kind() {
        ErrorKind::UnknownArgument => ContextKind::InvalidArg,
        ErrorKind::InvalidSubcommand => ContextKind::InvalidSubcommand,
        _ => return None,
    };
    let unknown_name = match error.get(context)? {
        ContextValue::String(value) => value.split_whitespace().next()?.to_owned(),
        _ => return None,
    };

    // Resolve the command path from clap's usage line ("Usage: assess doctor [OPTIONS]").
    let usage = error
        .get(ContextKind::Usage)
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut command = Cli::command();
    command.build();
    let mut scope = command.clone();
    for word in usage.split_whitespace().skip(2) {
        match scope.find_subcommand(word) {
            Some(sub) => scope = sub.clone(),
            None => break,
        }
    }

    let mut candidates = std::collections::BTreeSet::new();
    if unknown_name.starts_with("--") {
        candidates.insert("--help".to_owned());
        for arg in scope.get_arguments() {
            if let Some(long) = arg.get_long() {
                candidates.insert(format!("--{long}"));
            }
            for alias in arg.get_visible_aliases().into_iter().flatten() {
                candidates.insert(format!("--{alias}"));
            }
        }
    } else if !unknown_name.starts_with('-') {
        candidates.insert("help".to_owned());
        for sub in scope.get_subcommands().filter(|sub| !sub.is_hide_set()) {
            candidates.insert(sub.get_name().to_owned());
        }
    } else {
        return None;
    }

    unique_one_edit_away(&unknown_name, candidates)
}

/// `Some(candidate)` only when exactly one candidate is one edit from the unknown name.
/// A tie or no match returns `None`: a hint must never be a guess.
fn unique_one_edit_away(
    unknown_name: &str,
    candidates: impl IntoIterator<Item = String>,
) -> Option<String> {
    let mut close = candidates
        .into_iter()
        .filter(|candidate| edit_distance(unknown_name, candidate) == 1);
    match (close.next(), close.next()) {
        (Some(only), None) => Some(only),
        _ => None,
    }
}

fn render_mode(json: bool, render: Option<RunRenderMode>) -> RenderMode {
    if json {
        return RenderMode::Json;
    }

    match render {
        Some(RunRenderMode::Summary) => RenderMode::Summary,
        Some(RunRenderMode::SummaryTsv) => RenderMode::SummaryTsv,
        None => RenderMode::Human,
    }
}

fn refusal_witness_status(no_witness: bool) -> WitnessStatus {
    if no_witness {
        WitnessStatus::Disabled
    } else {
        WitnessStatus::NotWritten
    }
}

#[cfg(test)]
mod typo_tests {
    use super::*;

    fn set(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn distance_counts_swaps_as_one_edit() {
        assert_eq!(edit_distance("--jsno", "--json"), 1);
        assert_eq!(edit_distance("--josn", "--json"), 1);
        assert_eq!(edit_distance("--jsn", "--json"), 1);
        assert_eq!(edit_distance("--jsson", "--json"), 1);
        assert_eq!(edit_distance("--jxon", "--json"), 1);
        assert_eq!(edit_distance("--json", "--json"), 0);
        assert_eq!(edit_distance("--js", "--json"), 2);
    }

    #[test]
    fn single_candidate_is_suggested() {
        assert_eq!(
            unique_one_edit_away("--polcy", set(&["--policy", "--policy-id", "--json"])),
            Some("--policy".to_owned())
        );
    }

    #[test]
    fn tie_yields_no_hint() {
        // `--jon` is one edit from both --json and --join: never guess.
        assert_eq!(
            unique_one_edit_away("--jon", set(&["--json", "--join", "--policy"])),
            None
        );
    }

    #[test]
    fn nothing_close_yields_no_hint() {
        assert_eq!(
            unique_one_edit_away("--completely-off", set(&["--json", "--policy"])),
            None
        );
    }
}
