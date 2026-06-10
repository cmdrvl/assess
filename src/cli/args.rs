use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

const AFTER_LONG_HELP: &str = "\
Agent entrypoints:
  assess --robot-triage
  assess capabilities --json
  assess robot-docs guide

Decision examples:
  assess shape.json rvl.json verify.json --policy policy.yaml --json
  assess shape.json rvl.json verify.json --policy-id loan_tape.monthly.v1 --render summary

Witness examples:
  assess witness last --json
  assess witness query policy=loan_tape.monthly.v1 --json
";

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RunRenderMode {
    Summary,
    SummaryTsv,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "assess",
    about = "Deterministic decision classification over a complete spine evidence bundle",
    long_about = "Classify a complete spine evidence bundle into PROCEED, PROCEED_WITH_RISK, ESCALATE, or BLOCK using exact-match policy rules.",
    after_long_help = AFTER_LONG_HELP,
    disable_version_flag = true,
    subcommand_precedence_over_arg = true,
    subcommand_negates_reqs = true
)]
pub struct Cli {
    #[arg(
        value_name = "ARTIFACT",
        help = "Spine report artifact included in the epistemic basis"
    )]
    pub artifacts: Vec<PathBuf>,

    #[arg(long, value_name = "PATH", help = "Decision policy YAML path")]
    pub policy: Option<String>,

    #[arg(
        long = "policy-id",
        value_name = "ID",
        help = "Resolve a policy by ID from ASSESS_POLICY_PATH, builtins, or ~/.cmdrvl/config/assess/policies"
    )]
    pub policy_id: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Emit canonical assess.v0 JSON or structured refusal JSON"
    )]
    pub json: bool,

    #[arg(
        long,
        value_enum,
        conflicts_with = "json",
        help = "Emit compact operator summary output"
    )]
    pub render: Option<RunRenderMode>,

    #[arg(
        long = "no-witness",
        help = "Suppress witness ledger recording for successful decisions"
    )]
    pub no_witness: bool,

    #[arg(long, global = true, help = "Print embedded operator.json and exit")]
    pub describe: bool,

    #[arg(long, global = true, help = "Print the assess.v0 JSON Schema and exit")]
    pub schema: bool,

    #[arg(long, global = true, help = "Print assess version and exit")]
    pub version: bool,

    #[arg(
        long = "robot-triage",
        global = true,
        help = "Emit read-only machine triage JSON and exit"
    )]
    pub robot_triage: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    #[command(about = "Read-only alias for `assess doctor capabilities`")]
    Capabilities(CapabilitiesArgs),
    #[command(
        name = "robot-docs",
        about = "Read-only alias for `assess doctor robot-docs`"
    )]
    RobotDocs(RobotDocsArgs),
    Doctor(DoctorArgs),
    Witness(WitnessArgs),
}

#[derive(Debug, Clone, Default, Args)]
pub struct CapabilitiesArgs {}

#[derive(Debug, Clone, Default, Args)]
pub struct RobotDocsArgs {
    #[command(subcommand)]
    pub command: Option<RobotDocsCommand>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum RobotDocsCommand {
    Guide,
}

#[derive(Debug, Clone, Args)]
pub struct DoctorArgs {
    #[arg(long = "robot-triage")]
    pub robot_triage: bool,

    #[command(subcommand)]
    pub command: Option<DoctorCommand>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum DoctorCommand {
    Health,
    Capabilities,
    #[command(name = "robot-docs")]
    RobotDocs,
}

#[derive(Debug, Clone, Args)]
pub struct WitnessArgs {
    #[command(subcommand)]
    pub command: WitnessCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum WitnessCommand {
    Query(WitnessQuery),
    Last(WitnessLast),
    Count(WitnessCount),
}

#[derive(Debug, Clone, Default, Args)]
pub struct WitnessQuery {
    #[arg(value_name = "FILTER")]
    pub filters: Vec<String>,
}

#[derive(Debug, Clone, Default, Args)]
pub struct WitnessLast {}

#[derive(Debug, Clone, Default, Args)]
pub struct WitnessCount {
    #[arg(value_name = "FILTER")]
    pub filters: Vec<String>,
}
