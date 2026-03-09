use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use super::{PolicyError, PolicyFile, validate};

pub const RESOLUTION_ORDER: [&str; 3] = [
    "ASSESS_POLICY_PATH",
    "builtin-policies",
    "~/.epistemic/policies/",
];

const BUILTIN_POLICIES: [BuiltinPolicy; 2] = [
    BuiltinPolicy {
        id: "loan_tape.monthly.v1",
        bytes: include_bytes!("../../fixtures/policies/loan_tape_monthly_v1.yaml"),
    },
    BuiltinPolicy {
        id: "default.v0",
        bytes: include_bytes!("../../fixtures/policies/minimal_default_only.yaml"),
    },
];

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedPolicy {
    pub policy: PolicyFile,
    pub sha256: String,
    pub source: PolicySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicySource {
    Path(PathBuf),
    SearchPath(PathBuf),
    Builtin(&'static str),
    UserDir(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PolicySearchPaths {
    pub env_paths: Vec<PathBuf>,
    pub home_dir: Option<PathBuf>,
}

impl PolicySearchPaths {
    pub fn new(env_paths: Vec<PathBuf>, home_dir: Option<PathBuf>) -> Self {
        Self {
            env_paths,
            home_dir,
        }
    }

    pub fn from_process() -> Self {
        let env_paths = env::var_os("ASSESS_POLICY_PATH")
            .map(|paths| env::split_paths(&paths).collect())
            .unwrap_or_default();
        let home_dir = env::var_os("HOME").map(PathBuf::from);

        Self {
            env_paths,
            home_dir,
        }
    }

    pub fn user_policy_dir(&self) -> Option<PathBuf> {
        self.home_dir
            .as_ref()
            .map(|home_dir| home_dir.join(".epistemic").join("policies"))
    }
}

#[derive(Debug, Clone, Copy)]
struct BuiltinPolicy {
    id: &'static str,
    bytes: &'static [u8],
}

#[derive(Debug, Clone, Copy)]
enum CandidateSource {
    SearchPath,
    UserDir,
}

pub fn resolution_order() -> &'static [&'static str; 3] {
    &RESOLUTION_ORDER
}

pub fn load_path(path: impl AsRef<Path>) -> Result<LoadedPolicy, PolicyError> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| PolicyError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    parse_loaded_policy(
        &bytes,
        format!("path `{}`", path.display()),
        PolicySource::Path(path.to_path_buf()),
    )
}

pub fn load_policy_id(id: &str) -> Result<LoadedPolicy, PolicyError> {
    let search_paths = PolicySearchPaths::from_process();
    load_policy_id_with(id, &search_paths)
}

pub fn load_policy_id_with(
    id: &str,
    search_paths: &PolicySearchPaths,
) -> Result<LoadedPolicy, PolicyError> {
    if let Some(loaded) = find_in_dirs(id, &search_paths.env_paths, CandidateSource::SearchPath)? {
        return Ok(loaded);
    }

    if let Some(loaded) = find_builtin(id)? {
        return Ok(loaded);
    }

    if let Some(user_dir) = search_paths.user_policy_dir()
        && let Some(loaded) = find_in_dirs(id, &[user_dir], CandidateSource::UserDir)?
    {
        return Ok(loaded);
    }

    Err(PolicyError::NotFound { id: id.to_owned() })
}

pub fn policy_sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn find_in_dirs(
    id: &str,
    directories: &[PathBuf],
    source: CandidateSource,
) -> Result<Option<LoadedPolicy>, PolicyError> {
    for directory in directories {
        let candidates = yaml_candidates(directory)?;
        for candidate in candidates {
            let bytes = fs::read(&candidate).map_err(|source_error| PolicyError::Io {
                path: candidate.clone(),
                source: source_error,
            })?;
            let loaded = parse_loaded_policy(
                &bytes,
                format!("path `{}`", candidate.display()),
                match source {
                    CandidateSource::SearchPath => PolicySource::SearchPath(candidate.clone()),
                    CandidateSource::UserDir => PolicySource::UserDir(candidate.clone()),
                },
            )?;
            if loaded.policy.policy_id == id {
                return Ok(Some(loaded));
            }
        }
    }

    Ok(None)
}

fn find_builtin(id: &str) -> Result<Option<LoadedPolicy>, PolicyError> {
    for builtin in BUILTIN_POLICIES {
        if builtin.id == id {
            return parse_loaded_policy(
                builtin.bytes,
                format!("builtin policy `{}`", builtin.id),
                PolicySource::Builtin(builtin.id),
            )
            .map(Some);
        }
    }

    Ok(None)
}

fn parse_loaded_policy(
    bytes: &[u8],
    location: String,
    source: PolicySource,
) -> Result<LoadedPolicy, PolicyError> {
    let policy: PolicyFile =
        serde_yaml::from_slice(bytes).map_err(|source_error| PolicyError::YamlParse {
            location,
            source: source_error,
        })?;
    validate(&policy)?;

    Ok(LoadedPolicy {
        policy,
        sha256: policy_sha256(bytes),
        source,
    })
}

fn yaml_candidates(directory: &Path) -> Result<Vec<PathBuf>, PolicyError> {
    let mut candidates = Vec::new();
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(candidates),
        Err(source) => {
            return Err(PolicyError::Io {
                path: directory.to_path_buf(),
                source,
            });
        }
    };

    for entry in entries {
        let entry = entry.map_err(|source| PolicyError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("yaml" | "yml")
        ) {
            candidates.push(path);
        }
    }

    candidates.sort();
    Ok(candidates)
}
