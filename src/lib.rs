#![forbid(unsafe_code)]

pub mod bundle;
pub mod cli;
pub mod evaluate;
pub mod output;
pub mod policy;
pub mod refusal;
pub mod witness;

use std::path::Path;
use std::time::Instant;

use cli::{AssessExit, Cli, Route, RouteError};
use output::{RenderContext, RenderMode, WitnessStatus};
use refusal::RefusalEnvelope;
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

impl Execution {
    pub fn new(exit: AssessExit, stdout: impl AsRef<str>) -> Self {
        Self {
            exit_code: exit.code(),
            stdout: with_trailing_newline(stdout.as_ref()),
        }
    }

    pub fn refusal(
        refusal: RefusalEnvelope,
        render_mode: RenderMode,
        witness_status: WitnessStatus,
    ) -> Self {
        let stdout = output::render_with_context(
            &output::AssessResult::Refusal(refusal),
            render_mode,
            RenderContext::with_witness_status(witness_status),
        );
        Self::new(AssessExit::Stop, stdout)
    }
}

#[derive(Debug, Error)]
pub enum AssessError {
    #[error(transparent)]
    Usage(#[from] clap::Error),
    #[error("{0}")]
    Witness(String),
}

pub fn execute(cli: Cli) -> Result<Execution, AssessError> {
    match cli::route(cli) {
        Ok(Route::Describe) => Ok(Execution::new(AssessExit::Proceed, OPERATOR_JSON)),
        Ok(Route::Schema) => Ok(Execution::new(AssessExit::Proceed, ASSESS_SCHEMA_JSON)),
        Ok(Route::Version) => Ok(Execution::new(
            AssessExit::Proceed,
            format!("assess {}", env!("CARGO_PKG_VERSION")),
        )),
        Ok(Route::Witness(invocation)) => execute_witness(invocation),
        Ok(Route::Run(command)) => execute_run(command),
        Err(RouteError::Usage(error)) => Err(AssessError::Usage(*error)),
        Err(RouteError::Refusal {
            refusal,
            render_mode,
            witness_status,
        }) => Ok(Execution::refusal(*refusal, render_mode, witness_status)),
    }
}

fn with_trailing_newline(value: &str) -> String {
    if value.ends_with('\n') {
        value.to_owned()
    } else {
        format!("{value}\n")
    }
}

fn execute_run(command: cli::RunCommand) -> Result<Execution, AssessError> {
    let start = Instant::now();
    let refusal_context =
        RenderContext::with_witness_status(refusal_witness_status(command.no_witness));

    let loaded_policy = match &command.policy_selector {
        cli::PolicySelector::Path(path) => policy::load_path(Path::new(path)),
        cli::PolicySelector::Id(id) => policy::load_policy_id(id),
    };

    let loaded_policy = match loaded_policy {
        Ok(lp) => lp,
        Err(error) => {
            let refusal = RefusalEnvelope::new(error.refusal_code(), error.to_string());
            let stdout = output::render_with_context(
                &output::AssessResult::Refusal(refusal.clone()),
                command.render_mode,
                refusal_context,
            );
            return Ok(Execution::new(AssessExit::Stop, stdout));
        }
    };

    let artifact_bundle = match bundle::load(&command.artifacts) {
        Ok(b) => b,
        Err(error) => {
            let refusal = RefusalEnvelope::new(error.refusal_code(), error.to_string());
            let stdout = output::render_with_context(
                &output::AssessResult::Refusal(refusal.clone()),
                command.render_mode,
                refusal_context,
            );
            return Ok(Execution::new(AssessExit::Stop, stdout));
        }
    };

    let decision = match evaluate::evaluate(&loaded_policy.policy, &artifact_bundle) {
        Ok(d) => d,
        Err(error) => {
            let refusal = RefusalEnvelope::new(error.refusal_code(), error.to_string());
            let stdout = output::render_with_context(
                &output::AssessResult::Refusal(refusal.clone()),
                command.render_mode,
                refusal_context,
            );
            return Ok(Execution::new(AssessExit::Stop, stdout));
        }
    };

    let exit = AssessExit::from_decision_band(decision.decision_band);
    let assess_output = output::build_output(&decision, &artifact_bundle, &loaded_policy);
    let witness_status = if command.no_witness {
        WitnessStatus::Disabled
    } else {
        let elapsed = start.elapsed();
        let inputs: Vec<String> = command
            .artifacts
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        let record = witness::WitnessRecord::scaffold(inputs)
            .with_policy_id(&loaded_policy.policy.policy_id)
            .with_decision_band(decision.decision_band.as_str())
            .with_duration_ms(elapsed.as_millis() as u64)
            .with_timestamp(unix_seconds_now());

        witness::ledger::append(&record)
            .map_err(|e| AssessError::Witness(format!("failed to append witness: {e}")))?;
        WitnessStatus::Written
    };

    let stdout = output::render_with_context(
        &output::AssessResult::Decision(assess_output),
        command.render_mode,
        RenderContext::with_witness_status(witness_status),
    );

    Ok(Execution::new(exit, stdout))
}

fn unix_seconds_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn execute_witness(invocation: cli::WitnessInvocation) -> Result<Execution, AssessError> {
    let output = match invocation.command {
        cli::WitnessInvocationCommand::Query { filters } => {
            witness::query::render_query(&filters, invocation.json)
        }
        cli::WitnessInvocationCommand::Last => witness::query::render_last(invocation.json),
        cli::WitnessInvocationCommand::Count { filters } => {
            witness::query::render_count(&filters, invocation.json)
        }
    }
    .map_err(AssessError::Witness)?;

    Ok(Execution {
        exit_code: output.exit_code,
        stdout: with_trailing_newline(&output.stdout),
    })
}

fn refusal_witness_status(no_witness: bool) -> WitnessStatus {
    if no_witness {
        WitnessStatus::Disabled
    } else {
        WitnessStatus::NotWritten
    }
}
