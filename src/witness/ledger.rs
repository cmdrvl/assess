use std::{
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use super::record::WitnessRecord;
use crate::paths;

pub fn append(record: &WitnessRecord) -> Result<(), io::Error> {
    append_to_path(&paths::prepare_witness_path_for_append()?, record)
}

pub fn append_to_path(path: &Path, record: &WitnessRecord) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let encoded = serde_json::to_string(record)
        .map_err(|error| io::Error::other(format!("failed to encode witness record: {error}")))?;
    writeln!(file, "{encoded}")?;
    Ok(())
}

pub fn load() -> Result<Vec<WitnessRecord>, String> {
    let path = witness_ledger_path_for_query()
        .map_err(|error| format!("assess: witness: failed to prepare ledger path: {error}"))?;
    load_from_path(&path)
}

pub fn load_from_path(path: &Path) -> Result<Vec<WitnessRecord>, String> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("assess: witness: failed to read ledger: {error}")),
    };

    let mut records = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = match line {
            Ok(line) if !line.trim().is_empty() => line,
            Ok(_) => continue,
            Err(_) => continue,
        };

        let Ok(record) = serde_json::from_str::<WitnessRecord>(&line) else {
            continue;
        };

        if record.tool == "assess" {
            records.push(record);
        }
    }

    Ok(records)
}

pub fn witness_ledger_path() -> PathBuf {
    paths::default_witness_path()
}

pub fn witness_ledger_path_for_query() -> Result<PathBuf, io::Error> {
    paths::prepare_witness_path_for_query()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::paths;

    use super::{WitnessRecord, append_to_path, load_from_path};

    #[test]
    fn explicit_epistemic_witness_path_wins() {
        let path = PathBuf::from("/tmp/custom-ledger.jsonl");

        assert_eq!(path, PathBuf::from("/tmp/custom-ledger.jsonl"));
    }

    #[test]
    fn home_fallback_uses_standard_ledger_location() {
        let path = PathBuf::from("/tmp/home/.cmdrvl/state/witness/witness.jsonl");

        assert_eq!(
            path,
            PathBuf::from(paths::CANONICAL_WITNESS.replace('~', "/tmp/home"))
        );
    }

    #[test]
    fn malformed_ledger_lines_are_ignored() {
        let path = std::env::temp_dir().join(format!(
            "assess-witness-test-{}-malformed.jsonl",
            std::process::id()
        ));
        std::fs::write(
            &path,
            format!(
                "{}\n{}\nnot-json\n",
                serde_json::json!({"tool": "assess", "command": "run", "inputs": [], "duration_ms": 0, "ts": "1"}),
                serde_json::json!({"tool": "verify", "command": "run", "inputs": [], "duration_ms": 0, "ts": "2"})
            ),
        )
        .expect("ledger file should write");

        let records = load_from_path(&path).expect("ledger should load");
        std::fs::remove_file(&path).expect("temporary ledger should be removed");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].tool, "assess");
    }

    #[test]
    fn append_round_trips_record() {
        let path = std::env::temp_dir().join(format!(
            "assess-witness-test-{}-append.jsonl",
            std::process::id()
        ));
        let record = WitnessRecord::scaffold(vec!["shape.json".to_owned()])
            .with_policy_id("loan_tape.monthly.v1")
            .with_decision_band("PROCEED")
            .with_timestamp("123");

        append_to_path(&path, &record).expect("append should succeed");
        let records = load_from_path(&path).expect("records should load");
        std::fs::remove_file(&path).expect("temporary ledger should be removed");

        assert_eq!(records, vec![record]);
    }
}
