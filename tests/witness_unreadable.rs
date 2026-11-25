//! A ledger path that cannot be read (a directory) must produce a typed exit-2 error quickly.
//! Before the fix the reader ignored the read error and spun at 100% CPU forever. Each case runs
//! the binary as a child under a hard timeout and kills it if it does not exit.
//! Empty and absent ledgers still exit 0 (family ADR).

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("assess-unreadable-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `None` means the child did not exit within 5 seconds and was killed.
fn assess_with_timeout(ledger: &Path, args: &[&str]) -> Option<Output> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_assess"))
        .args(args)
        .env("EPISTEMIC_WITNESS", ledger)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn assess");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().expect("try_wait").is_some() {
            return Some(child.wait_with_output().expect("collect output"));
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill hung child");
            child.wait().expect("reap child");
            return None;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn directory_ledger_refuses_fast_for_query_last_and_count() {
    let directory = scratch("directory").join("a_directory");
    std::fs::create_dir_all(&directory).unwrap();

    for args in [
        &["witness", "query", "policy=none.v1", "--json"][..],
        &["witness", "last", "--json"][..],
        &["witness", "count", "--json"][..],
    ] {
        let o = assess_with_timeout(&directory, args);
        assert!(
            o.is_some(),
            "assess {} hung on a directory ledger",
            args.join(" ")
        );
        let o = o.expect("checked above");
        assert_eq!(o.status.code(), Some(2), "assess {}", args.join(" "));
        assert!(o.stdout.is_empty(), "stdout must stay empty");
        let stderr = String::from_utf8_lossy(&o.stderr);
        assert!(
            stderr.contains(&directory.display().to_string()),
            "stderr must name the path: {stderr}"
        );
    }
}

#[test]
fn empty_and_absent_ledgers_still_exit_zero() {
    let root = scratch("empty");
    let absent = root.join("absent.jsonl");
    let empty = root.join("empty.jsonl");
    std::fs::write(&empty, "").unwrap();

    for ledger in [&absent, &empty] {
        let q = assess_with_timeout(ledger, &["witness", "query", "policy=none.v1", "--json"])
            .expect("query must not hang");
        assert_eq!(q.status.code(), Some(0));
        assert_eq!(String::from_utf8_lossy(&q.stdout).trim(), "[]");
        let c = assess_with_timeout(ledger, &["witness", "count", "--json"])
            .expect("count must not hang");
        assert_eq!(c.status.code(), Some(0));
        assert_eq!(String::from_utf8_lossy(&c.stdout).trim(), r#"{"count":0}"#);
        let l = assess_with_timeout(ledger, &["witness", "last", "--json"])
            .expect("last must not hang");
        assert_eq!(l.status.code(), Some(0));
        assert_eq!(String::from_utf8_lossy(&l.stdout).trim(), "null");
    }
}

#[test]
fn a_non_utf8_line_is_skipped_and_reading_continues() {
    let ledger = scratch("non-utf8").join("witness.jsonl");
    let good = |ts: &str| {
        format!(
            r#"{{"tool":"assess","command":"assess","ts":"{ts}","policy_id":"p.one","decision_band":"PROCEED","inputs":[],"duration_ms":1}}"#
        )
    };
    let mut bytes = Vec::new();
    bytes.extend_from_slice(good("1").as_bytes());
    bytes.push(b'\n');
    bytes.extend_from_slice(&[0xff, 0xfe, b'\n']);
    bytes.extend_from_slice(good("2").as_bytes());
    bytes.push(b'\n');
    std::fs::write(&ledger, bytes).unwrap();

    let o = assess_with_timeout(&ledger, &["witness", "count", "--json"]).expect("no hang");
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), r#"{"count":2}"#);
}
