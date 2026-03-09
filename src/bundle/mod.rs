pub mod artifact;
pub mod derive;

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::refusal::RefusalCode;

pub use artifact::{ArtifactBasisEntry, ArtifactRefusal, ArtifactReport};

#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactBundle {
    basis: Vec<ArtifactBasisEntry>,
    reports: BTreeMap<String, ArtifactReport>,
}

impl ArtifactBundle {
    pub fn basis(&self) -> &[ArtifactBasisEntry] {
        &self.basis
    }

    pub fn observed_tools(&self) -> Vec<String> {
        self.reports.keys().cloned().collect()
    }

    pub fn get(&self, canonical_tool: &str) -> Option<&ArtifactReport> {
        self.reports.get(canonical_tool)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BundleError {
    #[error("invalid artifact {path}: {message}")]
    BadArtifact { path: String, message: String },
    #[error("duplicate canonical tool `{tool}` for artifacts {first} and {second}")]
    DuplicateTool {
        tool: String,
        first: String,
        second: String,
    },
}

impl BundleError {
    pub const fn refusal_code(&self) -> RefusalCode {
        match self {
            Self::BadArtifact { .. } => RefusalCode::BadArtifact,
            Self::DuplicateTool { .. } => RefusalCode::DuplicateTool,
        }
    }
}

pub fn load(paths: &[PathBuf]) -> Result<ArtifactBundle, BundleError> {
    let mut basis = Vec::with_capacity(paths.len());
    let mut reports = BTreeMap::new();
    let mut first_seen = BTreeMap::new();

    for path in paths {
        let artifact = load_one(path)?;
        let artifact_name = path.display().to_string();

        if let Some(existing) =
            first_seen.insert(artifact.canonical_tool.clone(), artifact_name.clone())
        {
            return Err(BundleError::DuplicateTool {
                tool: artifact.canonical_tool,
                first: existing,
                second: artifact_name,
            });
        }

        basis.push(ArtifactBasisEntry {
            artifact: artifact_name,
            tool: artifact.canonical_tool.clone(),
            version: artifact.report.version.clone(),
            outcome: artifact.report.outcome.clone(),
            policy_signals: artifact.report.policy_signals.clone(),
            refusal: artifact.report.refusal.clone(),
        });
        reports.insert(artifact.canonical_tool, artifact.report);
    }

    basis.sort_by(|left, right| {
        left.tool
            .cmp(&right.tool)
            .then_with(|| left.artifact.cmp(&right.artifact))
    });

    Ok(ArtifactBundle { basis, reports })
}

struct LoadedArtifact {
    canonical_tool: String,
    report: ArtifactReport,
}

fn load_one(path: &Path) -> Result<LoadedArtifact, BundleError> {
    let raw = fs::read(path).map_err(|error| BundleError::BadArtifact {
        path: path.display().to_string(),
        message: format!("failed to read artifact: {error}"),
    })?;

    let report: ArtifactReport =
        serde_json::from_slice(&raw).map_err(|error| BundleError::BadArtifact {
            path: path.display().to_string(),
            message: format!("failed to parse artifact JSON: {error}"),
        })?;

    let canonical_tool = derive::canonical_tool(report.tool.as_deref(), &report.version)
        .ok_or_else(|| BundleError::BadArtifact {
            path: path.display().to_string(),
            message: format!(
                "cannot derive canonical tool from tool={:?} version={}",
                report.tool, report.version
            ),
        })?;

    Ok(LoadedArtifact {
        canonical_tool,
        report,
    })
}
