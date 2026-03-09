use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "assess",
    about = "Deterministic decision classification over a complete spine evidence bundle",
    disable_version_flag = true,
    subcommand_precedence_over_arg = true
)]
pub struct Cli {
    #[arg(value_name = "ARTIFACT")]
    pub artifacts: Vec<PathBuf>,

    #[arg(long)]
    pub policy: Option<String>,

    #[arg(long = "policy-id")]
    pub policy_id: Option<String>,

    #[arg(long)]
    pub json: bool,

    #[arg(long = "no-witness")]
    pub no_witness: bool,

    #[arg(long)]
    pub describe: bool,

    #[arg(long)]
    pub schema: bool,

    #[arg(long)]
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

    #[arg(long)]
    pub json: bool,
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
