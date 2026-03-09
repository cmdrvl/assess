use std::{
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use super::record::WitnessRecord;

pub fn append(record: &WitnessRecord) -> Result<(), io::Error> {
    append_to_path(&witness_ledger_path(), record)
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
    load_from_path(&witness_ledger_path())
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
    witness_ledger_path_from_env(|key| std::env::var(key).ok())
}

fn witness_ledger_path_from_env<F>(get_env: F) -> PathBuf
where
    F: Fn(&str) -> Option<String>,
{
    if let Some(path) = get_env("EPISTEMIC_WITNESS")
        && !path.trim().is_empty()
    {
        return PathBuf::from(path);
    }

    let home = home_from_env(&get_env).unwrap_or_else(|| PathBuf::from("."));
    home.join(".epistemic").join("witness.jsonl")
}

fn home_from_env<F>(get_env: &F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<String>,
{
    #[cfg(unix)]
    {
        get_env("HOME")
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
    }

    #[cfg(windows)]
    {
        get_env("USERPROFILE")
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
    }

    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{WitnessRecord, append_to_path, load_from_path, witness_ledger_path_from_env};

    #[test]
    fn explicit_epistemic_witness_path_wins() {
        let path = witness_ledger_path_from_env(|key| match key {
            "EPISTEMIC_WITNESS" => Some("/tmp/custom-ledger.jsonl".to_owned()),
            "HOME" => Some("/tmp/home".to_owned()),
            _ => None,
        });

        assert_eq!(path, PathBuf::from("/tmp/custom-ledger.jsonl"));
    }

    #[test]
    fn home_fallback_uses_standard_ledger_location() {
        let path = witness_ledger_path_from_env(|key| match key {
            "EPISTEMIC_WITNESS" => Some(String::new()),
            "HOME" => Some("/tmp/home".to_owned()),
            _ => None,
        });

        assert_eq!(path, PathBuf::from("/tmp/home/.epistemic/witness.jsonl"));
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
