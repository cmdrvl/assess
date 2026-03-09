#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn repo_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

pub fn fixture_path(relative: &str) -> PathBuf {
    repo_path().join("fixtures").join(relative)
}

pub fn rule_path(relative: &str) -> PathBuf {
    repo_path().join("rules").join(relative)
}
