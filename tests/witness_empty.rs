//! Read/query commands never exit 1 for an empty result (spine-rules empty-query rule):
//! `witness query` prints `[]`, `count` prints a zero count, `last` prints null; all exit 0.
//! A populated ledger still returns its records with exit 0.

use std::path::PathBuf;
use std::process::{Command, Output};

fn ledger(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("assess-wit-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d.join("witness.jsonl")
}

fn assess(ledger: &PathBuf, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_assess"))
        .args(args)
        .env("EPISTEMIC_WITNESS", ledger)
        .output()
        .expect("run assess")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).trim().to_owned()
}

#[test]
fn empty_json_queries_exit_zero_with_empty_values() {
    let l = ledger("empty-json"); // file does not exist yet
    let q = assess(&l, &["witness", "query", "policy=none.v1", "--json"]);
    assert_eq!((q.status.code(), stdout(&q).as_str()), (Some(0), "[]"));

    let c = assess(&l, &["witness", "count", "policy=none.v1", "--json"]);
    assert_eq!(
        (c.status.code(), stdout(&c).as_str()),
        (Some(0), r#"{"count":0}"#)
    );

    let last = assess(&l, &["witness", "last", "--json"]);
    assert_eq!(
        (last.status.code(), stdout(&last).as_str()),
        (Some(0), "null")
    );
}

#[test]
fn empty_human_queries_exit_zero() {
    let l = ledger("empty-human");
    for args in [
        &["witness", "query", "policy=none.v1"][..],
        &["witness", "count"][..],
        &["witness", "last"][..],
    ] {
        let o = assess(&l, args);
        assert_eq!(o.status.code(), Some(0), "assess {}", args.join(" "));
    }
}

#[test]
fn query_with_matches_still_exits_zero_and_filters_correctly() {
    let l = ledger("populated");
    std::fs::write(
        &l,
        concat!(
            r#"{"tool":"assess","command":"assess","ts":"1","policy_id":"p.one","decision_band":"PROCEED","inputs":["a.json"],"duration_ms":1}"#,
            "\n",
            r#"{"tool":"assess","command":"assess","ts":"2","policy_id":"p.two","decision_band":"BLOCK","inputs":["b.json"],"duration_ms":2}"#,
            "\n"
        ),
    )
    .unwrap();
    let hit = assess(&l, &["witness", "query", "policy=p.two", "--json"]);
    assert_eq!(hit.status.code(), Some(0));
    let rows: serde_json::Value = serde_json::from_str(&stdout(&hit)).unwrap();
    assert_eq!(rows.as_array().map(Vec::len), Some(1), "{rows}");
    assert_eq!(rows[0]["decision_band"], "BLOCK");

    let miss = assess(&l, &["witness", "query", "policy=p.three", "--json"]);
    assert_eq!(
        (miss.status.code(), stdout(&miss).as_str()),
        (Some(0), "[]")
    );
}

#[test]
fn a_bad_filter_is_still_an_error_not_an_empty_result() {
    // Filters are validated per record, so the ledger needs one record to reach them.
    let l = ledger("bad-filter");
    std::fs::write(
        &l,
        concat!(
            r#"{"tool":"assess","command":"assess","ts":"1","policy_id":"p.one","decision_band":"PROCEED","inputs":[],"duration_ms":1}"#,
            "\n"
        ),
    )
    .unwrap();
    let o = assess(&l, &["witness", "query", "no-equals-sign", "--json"]);
    assert_eq!(o.status.code(), Some(2));
}
