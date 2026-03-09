use assess::cli::Cli;
use assess::{OPERATOR_JSON, execute};
use clap::Parser;

#[test]
fn describe_surface_works_before_runtime_semantics() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse_from(["assess", "--describe"]);
    let execution = execute(cli)?;
    assert_eq!(execution.exit_code, 0);
    let expected = if OPERATOR_JSON.ends_with('\n') {
        OPERATOR_JSON.to_owned()
    } else {
        format!("{OPERATOR_JSON}\n")
    };
    assert_eq!(execution.stdout, expected);
    Ok(())
}
