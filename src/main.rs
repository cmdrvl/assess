#![forbid(unsafe_code)]

use std::process::ExitCode;

use assess::cli::Cli;
use clap::{Parser, error::ErrorKind};

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            print!("{error}");
            return ExitCode::from(0);
        }
        Err(error) if error.kind() == ErrorKind::DisplayVersion => {
            print!("{error}");
            return ExitCode::from(0);
        }
        Err(error) => {
            eprint!("{}", assess::cli::format_cli_error(&error));
            return ExitCode::from(2);
        }
    };

    match assess::execute(cli) {
        Ok(execution) => {
            print!("{}", execution.stdout);
            ExitCode::from(execution.exit_code)
        }
        Err(assess::AssessError::Usage(error)) => {
            eprint!("{}", assess::cli::format_cli_error(&error));
            ExitCode::from(2)
        }
        Err(assess::AssessError::Witness(error)) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
