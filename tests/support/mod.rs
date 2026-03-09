#![allow(dead_code)]

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde::Serialize;
use serde_json::Value;

const TEMP_WORKSPACE_ROOT: &str = "target/test-support";
static TEMP_WORKSPACE_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn repo_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

pub fn fixture_root() -> PathBuf {
    repo_path().join("fixtures")
}

pub fn fixture_path(relative: impl AsRef<Path>) -> PathBuf {
    fixture_root().join(relative)
}

pub fn policy_path(relative: impl AsRef<Path>) -> PathBuf {
    fixture_path(Path::new("policies").join(relative))
}

pub fn artifact_path(relative: impl AsRef<Path>) -> PathBuf {
    fixture_path(Path::new("artifacts").join(relative))
}

pub fn golden_path(relative: impl AsRef<Path>) -> PathBuf {
    fixture_path(Path::new("golden").join(relative))
}

pub fn schema_path(relative: impl AsRef<Path>) -> PathBuf {
    repo_path().join("schemas").join(relative)
}

pub fn rule_path(relative: impl AsRef<Path>) -> PathBuf {
    repo_path().join("rules").join(relative)
}

pub fn read_text(path: impl AsRef<Path>) -> io::Result<String> {
    fs::read_to_string(path)
}

pub fn read_fixture(relative: impl AsRef<Path>) -> io::Result<String> {
    read_text(fixture_path(relative))
}

pub fn read_json(path: impl AsRef<Path>) -> Result<Value, Box<dyn std::error::Error>> {
    let raw = read_text(path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub fn read_fixture_json(relative: impl AsRef<Path>) -> Result<Value, Box<dyn std::error::Error>> {
    read_json(fixture_path(relative))
}

pub fn normalized_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn assert_human_lines(actual: &str, expected_lines: &[&str]) {
    let actual = normalized_lines(actual);
    let expected = expected_lines
        .iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    let mut scan_index = 0usize;
    for expected_line in &expected {
        let found_at = actual[scan_index..]
            .iter()
            .position(|line| line == expected_line);
        assert!(
            found_at.is_some(),
            "missing human line `{expected_line}` in output lines {actual:?}"
        );
        scan_index += found_at.unwrap_or(0) + 1;
    }
}

pub fn assert_json_pointer(actual: &Value, pointer: &str, expected: &Value) {
    let found = actual.pointer(pointer);
    assert!(
        found.is_some(),
        "missing json pointer `{pointer}` in value {actual}"
    );
    assert_eq!(
        found.unwrap_or(expected),
        expected,
        "json pointer `{pointer}` mismatch in value {actual}"
    );
}

#[derive(Debug, Clone)]
pub struct TempWorkspace {
    root: PathBuf,
}

impl TempWorkspace {
    pub fn new(prefix: &str) -> io::Result<Self> {
        let root = repo_path().join(TEMP_WORKSPACE_ROOT).join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            TEMP_WORKSPACE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn child(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root.join(relative)
    }

    pub fn write(&self, relative: impl AsRef<Path>, contents: &str) -> io::Result<PathBuf> {
        let path = self.child(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents)?;
        Ok(path)
    }

    pub fn write_json(
        &self,
        relative: impl AsRef<Path>,
        value: &impl Serialize,
    ) -> io::Result<PathBuf> {
        let payload = serde_json::to_string_pretty(value)
            .map_err(|error| io::Error::other(format!("json encode failed: {error}")))?;
        self.write(relative, &payload)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        TempWorkspace, artifact_path, assert_human_lines, assert_json_pointer, golden_path,
        normalized_lines, policy_path, read_fixture_json, read_text, schema_path,
    };

    #[test]
    fn fixture_family_helpers_point_at_real_files() {
        assert!(policy_path("loan_tape_monthly_v1.yaml").exists());
        assert!(artifact_path("shape_compatible.json").exists());
        assert!(golden_path("proceed.json").exists());
        assert!(schema_path("assess.v0.schema.json").exists());
    }

    #[test]
    fn read_fixture_json_parses_assess_fixture_payloads() {
        let parsed = read_fixture_json("artifacts/verify_pass.json").expect("fixture should parse");
        assert_json_pointer(&parsed, "/tool", &json!("verify"));
        assert_json_pointer(&parsed, "/outcome", &json!("PASS"));
    }

    #[test]
    fn human_line_assertion_ignores_blank_lines_but_preserves_order() {
        assert_human_lines(
            "VERIFY ESCALATE\n\nmatched_rule: diffuse_requires_review\nrisk: DIFFUSE_CHANGE\n",
            &[
                "VERIFY ESCALATE",
                "matched_rule: diffuse_requires_review",
                "risk: DIFFUSE_CHANGE",
            ],
        );
    }

    #[test]
    fn normalized_lines_trims_blank_lines() {
        assert_eq!(
            normalized_lines("alpha\r\n\r\nbeta\n"),
            vec!["alpha".to_owned(), "beta".to_owned()]
        );
    }

    #[test]
    fn temp_workspace_writes_nested_files() {
        let workspace = TempWorkspace::new("support-harness").expect("temp workspace should exist");
        let path = workspace
            .write("nested/output.txt", "hello assess")
            .expect("write should succeed");

        assert!(path.exists());
        assert_eq!(read_text(path).expect("file should read"), "hello assess");
        assert!(workspace.path().exists());
    }

    #[test]
    fn temp_workspace_writes_json_payloads() {
        let workspace = TempWorkspace::new("json-support").expect("temp workspace should exist");
        let path = workspace
            .write_json("decision.json", &json!({"decision_band": "PROCEED"}))
            .expect("json write should succeed");

        let payload = read_text(path).expect("json file should read");
        let parsed: serde_json::Value =
            serde_json::from_str(&payload).expect("written json should parse");
        assert_eq!(parsed["decision_band"], "PROCEED");
    }
}
