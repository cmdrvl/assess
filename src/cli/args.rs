use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "assess",
    about = "Deterministic decision classification over a complete spine evidence bundle",
    disable_version_flag = true,
    subcommand_precedence_over_arg = true,
    subcommand_negates_reqs = true
)]
pub struct Cli {
    #[arg(
        value_name = "ARTIFACT",
        required_unless_present_any = ["describe", "schema", "version"]
    )]
    pub artifacts: Vec<PathBuf>,

    #[arg(
        long,
        required_unless_present_any = ["policy_id", "describe", "schema", "version"]
    )]
    pub policy: Option<String>,

    #[arg(
        long = "policy-id",
        required_unless_present_any = ["policy", "describe", "schema", "version"]
    )]
    pub policy_id: Option<String>,

    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long = "no-witness")]
    pub no_witness: bool,

    #[arg(long, global = true)]
    pub describe: bool,

    #[arg(long, global = true)]
    pub schema: bool,

    #[arg(long, global = true)]
    pub version: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Witness(WitnessArgs),
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
