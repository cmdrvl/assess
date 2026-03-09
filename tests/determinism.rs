use assess::cli::Cli;
use assess::{ASSESS_SCHEMA_JSON, execute};
use clap::Parser;

#[test]
fn schema_output_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    let first = execute(Cli::parse_from(["assess", "--schema"]))?;
    let second = execute(Cli::parse_from(["assess", "--schema"]))?;

    let expected = if ASSESS_SCHEMA_JSON.ends_with('\n') {
        ASSESS_SCHEMA_JSON.to_owned()
    } else {
        format!("{ASSESS_SCHEMA_JSON}\n")
    };

    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stdout, expected);
    Ok(())
}
