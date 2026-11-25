//! NO_COLOR / TERM=dumb / non-TTY: no ANSI escapes on any surface a script reads,
//! including clap usage errors, even when color is forced on by the environment.

use std::process::{Command, Output};

fn run(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_assess"));
    cmd.args(args).env_remove("NO_COLOR").env_remove("TERM");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("run assess")
}

fn has_ansi(o: &Output) -> bool {
    let esc = |b: &[u8]| b.windows(2).any(|w| w == b"\x1b[");
    esc(&o.stdout) || esc(&o.stderr)
}

const SURFACES: &[&[&str]] = &[
    &["--help"],
    &["--describe"],
    &["--robot-triage"],
    &["doctor", "--json"],
    &["doctor", "health"],
    &["--definitely-not-a-flag"],
    &["--jsno"],
    &["doctor", "--fix"],
    &["witness", "query", "bogus-filter-without-equals"],
];

#[test]
fn no_ansi_when_piped_even_with_color_forced() {
    for args in SURFACES {
        for envs in [
            &[("CLICOLOR_FORCE", "1")][..],
            &[("CLICOLOR_FORCE", "1"), ("TERM", "xterm-256color")][..],
            &[("NO_COLOR", "1"), ("CLICOLOR_FORCE", "1")][..],
            &[("NO_COLOR", "1"), ("TERM", "dumb")][..],
            &[][..],
        ] {
            let o = run(args, envs);
            assert!(
                !has_ansi(&o),
                "`assess {}` emitted ANSI with env {envs:?}\nstdout: {}\nstderr: {}",
                args.join(" "),
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
        }
    }
}

#[test]
fn the_detector_itself_sees_escapes() {
    // Guard against a vacuous pass: the check must fire on real ANSI bytes.
    let o = Command::new("printf")
        .arg("\\033[31mred\\033[0m")
        .output()
        .expect("printf");
    assert!(has_ansi(&o));
}
