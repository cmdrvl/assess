//! Generalized typo recovery through the real binary: one edit away from a flag or
//! subcommand of the command the user is inside gets exactly one `did you mean` hint;
//! far typos and ties get none; every usage error exits 2 with an empty stdout.
//! Also pins the documented `--json` / `--render` interaction.

use std::process::{Command, Output};

fn assess(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_assess"))
        .args(args)
        .output()
        .expect("run assess")
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn hints(o: &Output) -> Vec<String> {
    stderr(o)
        .lines()
        .filter(|l| l.starts_with("hint: did you mean"))
        .map(str::to_owned)
        .collect()
}

fn assert_usage_error(o: &Output) {
    assert_eq!(o.status.code(), Some(2), "stderr: {}", stderr(o));
    assert!(o.stdout.is_empty(), "usage errors never write stdout");
}

#[test]
fn flag_typos_one_edit_away_get_one_hint() {
    for (typo, want) in [
        ("--jsno", "--json"),
        ("--josn", "--json"),
        ("--jso", "--json"),
        ("--polcy", "--policy"),
        ("--no-witnes", "--no-witness"),
        ("--rendr", "--render"),
        ("--descibe", "--describe"),
    ] {
        let o = assess(&[typo]);
        assert_usage_error(&o);
        assert_eq!(
            hints(&o),
            vec![format!("hint: did you mean `{want}`?")],
            "typo {typo}"
        );
    }
}

#[test]
fn subcommand_typos_inside_a_group_get_one_hint() {
    for (args, want) in [
        (&["witness", "qurey"][..], "query"),
        (&["witness", "cuont"][..], "count"),
        (&["doctor", "triag"][..], "triage"),
        (&["doctor", "helth"][..], "health"),
    ] {
        let o = assess(args);
        assert_usage_error(&o);
        assert_eq!(
            hints(&o),
            vec![format!("hint: did you mean `{want}`?")],
            "{args:?}"
        );
    }
}

#[test]
fn candidates_come_from_the_command_the_user_is_in() {
    // `--polcy` is one edit from the top-level --policy, but `doctor` has no such flag:
    // suggesting it would send the user to an invalid command.
    let o = assess(&["doctor", "--polcy"]);
    assert_usage_error(&o);
    assert!(hints(&o).is_empty(), "stderr: {}", stderr(&o));

    // A global flag is valid everywhere, so it is still suggested inside a group.
    let o = assess(&["witness", "last", "--josn"]);
    assert_usage_error(&o);
    assert_eq!(hints(&o), vec!["hint: did you mean `--json`?".to_owned()]);
}

#[test]
fn far_typos_and_non_typos_get_no_hint() {
    for args in [
        &["--completely-unrelated"][..],
        &["--js"][..],
        &["doctor", "--fix"][..],
        &["witness", "everything"][..],
    ] {
        let o = assess(args);
        assert_usage_error(&o);
        assert!(hints(&o).is_empty(), "{args:?}: {}", stderr(&o));
    }
}

#[test]
fn hints_always_keep_the_next_step_lines() {
    let o = assess(&["--jsno"]);
    let e = stderr(&o);
    assert!(e.contains("next: assess capabilities --json"), "{e}");
    assert!(e.contains("help: assess robot-docs guide"), "{e}");
}

// ── documented --json / --render interaction ─────────────────────────────

#[test]
fn json_and_render_are_mutually_exclusive() {
    let o = assess(&[
        "fixtures/artifacts/shape_compatible.json",
        "--policy-id",
        "default.v0",
        "--json",
        "--render",
        "summary",
    ]);
    assert_usage_error(&o);
    assert!(stderr(&o).contains("--json") && stderr(&o).contains("--render"));
}

#[test]
fn render_values_are_summary_and_summary_tsv_only() {
    let o = assess(&[
        "fixtures/artifacts/shape_compatible.json",
        "--policy-id",
        "default.v0",
        "--render",
        "summry",
    ]);
    assert_usage_error(&o);
    let e = stderr(&o);
    assert!(e.contains("summary") && e.contains("summary-tsv"), "{e}");
}

#[test]
fn render_alone_is_one_line_and_tsv_has_a_header() {
    let base = [
        "fixtures/artifacts/shape_compatible.json",
        "fixtures/artifacts/rvl_real_change.json",
        "fixtures/artifacts/verify_pass.json",
        "--policy-id",
        "loan_tape.monthly.v1",
        "--no-witness",
    ];
    let mut summary: Vec<&str> = base.to_vec();
    summary.extend(["--render", "summary"]);
    let o = assess(&summary);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&o.stdout).lines().count(), 1);

    let mut tsv: Vec<&str> = base.to_vec();
    tsv.extend(["--render", "summary-tsv"]);
    let o = assess(&tsv);
    assert_eq!(o.status.code(), Some(0));
    let out = String::from_utf8_lossy(&o.stdout);
    assert_eq!(out.lines().count(), 2, "header + one row: {out}");
    assert!(out.lines().next().unwrap().contains('\t'));
}
