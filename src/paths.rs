use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

const TOOL_NAME: &str = "assess";
const WITNESS_ENV: &str = "EPISTEMIC_WITNESS";
const POLICY_ENV: &str = "ASSESS_POLICY_PATH";

pub const CANONICAL_ROOT: &str = "~/.cmdrvl";
pub const CANONICAL_POLICY_DIR: &str = "~/.cmdrvl/config/assess/policies/";
pub const CANONICAL_WITNESS: &str = "~/.cmdrvl/state/witness/witness.jsonl";
pub const MIGRATION_LOG: &str = "~/.cmdrvl/migrations/applied.jsonl";
pub const DEPRECATION_NOTICES: &str = "~/.cmdrvl/notices/deprecated-paths.jsonl";
pub const LEGACY_POLICY_DIR: &str = "~/.epistemic/policies/";
pub const LEGACY_WITNESS: &str = "~/.epistemic/witness.jsonl";

pub fn home_dir_from_process() -> Option<PathBuf> {
    home_dir_from_env(env_value)
}

pub fn canonical_policy_dir() -> PathBuf {
    canonical_policy_dir_from_env(env_value)
}

pub fn canonical_policy_dir_from_home(home: &Path) -> PathBuf {
    home.join(".cmdrvl")
        .join("config")
        .join("assess")
        .join("policies")
}

pub fn ensure_policy_dir_migrated() -> io::Result<()> {
    ensure_policy_dir_migrated_from_env(env_value)
}

pub fn default_witness_path() -> PathBuf {
    default_witness_path_from_env(env_value)
}

pub fn prepare_witness_path_for_append() -> io::Result<PathBuf> {
    ensure_witness_migrated_from_env(env_value)?;
    let path = default_witness_path();
    if non_empty_env(env_value, WITNESS_ENV).is_none() {
        prepare_canonical_witness_tree_from_env(env_value)?;
    }
    Ok(path)
}

pub fn prepare_witness_path_for_query() -> io::Result<PathBuf> {
    ensure_witness_migrated_from_env(env_value)?;
    Ok(default_witness_path())
}

pub fn config_footprint() -> Value {
    json!({
        "schema": "cmdrvl.config_footprint.v1",
        "tool": TOOL_NAME,
        "canonical_root": CANONICAL_ROOT,
        "managed_config_paths": [CANONICAL_POLICY_DIR],
        "managed_state_paths": [CANONICAL_WITNESS],
        "managed_cache_paths": [],
        "managed_lock_paths": [],
        "env_overrides": [
            {
                "name": POLICY_ENV,
                "path_class": "policy_search_path",
                "behavior": "explicit operator search path; canonical policy dir remains the implicit user fallback"
            },
            {
                "name": WITNESS_ENV,
                "path_class": "witness_ledger",
                "behavior": "explicit operator ledger override; no implicit witness migration is performed for override paths"
            }
        ],
        "legacy_paths": [LEGACY_POLICY_DIR, LEGACY_WITNESS],
        "migration_log": MIGRATION_LOG,
        "deprecation_notices": DEPRECATION_NOTICES,
        "legacy_migration_required": true,
        "migration_policy": "copy-only legacy policy and witness migration; never delete or move legacy files; never record file contents or secret values",
        "self_contained": true
    })
}

fn default_witness_path_from_env<F>(get_env: F) -> PathBuf
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    if let Some(path) = non_empty_env(get_env, WITNESS_ENV) {
        return PathBuf::from(path);
    }

    canonical_witness_path_from_env(get_env)
}

fn ensure_policy_dir_migrated_from_env<F>(get_env: F) -> io::Result<()>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    let canonical = canonical_policy_dir_from_env(get_env);
    let Some(legacy) = legacy_policy_dir_from_env(get_env) else {
        return Ok(());
    };
    if !legacy.is_dir() {
        return Ok(());
    }

    let canonical_preexisting = canonical.exists();
    prepare_canonical_config_tree_from_env(get_env)?;
    let root = cmdrvl_root_from_env(get_env);
    let migration_log = root.join("migrations").join("applied.jsonl");
    let deprecation_notices = root.join("notices").join("deprecated-paths.jsonl");

    if canonical_preexisting {
        append_record_once(
            &deprecation_notices,
            deprecation_record(
                "policy_directory",
                &legacy,
                &canonical,
                "legacy_path_present",
                "canonical_preferred",
            ),
        )?;
        return Ok(());
    }

    copy_directory_contents(&legacy, &canonical)?;

    append_record_once(
        &migration_log,
        migration_record(
            "policy_directory",
            &legacy,
            &canonical,
            "copied_legacy_to_canonical",
        ),
    )?;
    append_record_once(
        &deprecation_notices,
        deprecation_record(
            "policy_directory",
            &legacy,
            &canonical,
            "legacy_path_migrated",
            "canonical_created",
        ),
    )?;

    Ok(())
}

fn ensure_witness_migrated_from_env<F>(get_env: F) -> io::Result<()>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    if non_empty_env(get_env, WITNESS_ENV).is_some() {
        return Ok(());
    }

    let canonical = canonical_witness_path_from_env(get_env);
    let Some(legacy) = legacy_witness_path_from_env(get_env) else {
        return Ok(());
    };
    if !legacy.is_file() || legacy == canonical {
        return Ok(());
    }

    prepare_canonical_witness_tree_from_env(get_env)?;
    let root = cmdrvl_root_from_env(get_env);
    let migration_log = root.join("migrations").join("applied.jsonl");
    let deprecation_notices = root.join("notices").join("deprecated-paths.jsonl");

    if canonical.exists() {
        append_record_once(
            &deprecation_notices,
            deprecation_record(
                "witness_ledger",
                &legacy,
                &canonical,
                "legacy_path_present",
                "canonical_preferred",
            ),
        )?;
        return Ok(());
    }

    if let Some(parent) = canonical.parent() {
        fs::create_dir_all(parent)?;
        harden_directory(parent)?;
    }

    fs::copy(&legacy, &canonical)?;
    fs::set_permissions(&canonical, fs::metadata(&legacy)?.permissions())?;

    append_record_once(
        &migration_log,
        migration_record(
            "witness_ledger",
            &legacy,
            &canonical,
            "copied_legacy_to_canonical",
        ),
    )?;
    append_record_once(
        &deprecation_notices,
        deprecation_record(
            "witness_ledger",
            &legacy,
            &canonical,
            "legacy_path_migrated",
            "canonical_created",
        ),
    )?;

    Ok(())
}

fn canonical_policy_dir_from_env<F>(get_env: F) -> PathBuf
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    cmdrvl_root_from_env(get_env)
        .join("config")
        .join("assess")
        .join("policies")
}

fn canonical_witness_path_from_env<F>(get_env: F) -> PathBuf
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    cmdrvl_root_from_env(get_env)
        .join("state")
        .join("witness")
        .join("witness.jsonl")
}

fn legacy_policy_dir_from_env<F>(get_env: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    home_dir_from_env(get_env).map(|home| home.join(".epistemic").join("policies"))
}

fn legacy_witness_path_from_env<F>(get_env: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    home_dir_from_env(get_env).map(|home| home.join(".epistemic").join("witness.jsonl"))
}

fn prepare_canonical_config_tree_from_env<F>(get_env: F) -> io::Result<()>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    let root = cmdrvl_root_from_env(get_env);
    for dir in [
        root.clone(),
        root.join("config"),
        root.join("config").join("assess"),
        root.join("config").join("assess").join("policies"),
        root.join("migrations"),
        root.join("notices"),
    ] {
        fs::create_dir_all(&dir)?;
        harden_directory(&dir)?;
    }
    Ok(())
}

fn prepare_canonical_witness_tree_from_env<F>(get_env: F) -> io::Result<()>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    let root = cmdrvl_root_from_env(get_env);
    for dir in [
        root.clone(),
        root.join("state"),
        root.join("state").join("witness"),
        root.join("migrations"),
        root.join("notices"),
    ] {
        fs::create_dir_all(&dir)?;
        harden_directory(&dir)?;
    }
    Ok(())
}

fn copy_directory_contents(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    harden_directory(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_directory_contents(&source_path, &destination_path)?;
        } else if file_type.is_file() && !destination_path.exists() {
            fs::copy(&source_path, &destination_path)?;
            fs::set_permissions(&destination_path, fs::metadata(&source_path)?.permissions())?;
        }
    }

    Ok(())
}

fn home_dir_from_env<F>(get_env: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    non_empty_env(get_env, "HOME")
        .or_else(|| non_empty_env(get_env, "USERPROFILE"))
        .map(PathBuf::from)
}

fn cmdrvl_root_from_env<F>(get_env: F) -> PathBuf
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    home_dir_from_env(get_env)
        .map(|home| home.join(".cmdrvl"))
        .unwrap_or_else(|| PathBuf::from(".cmdrvl"))
}

fn non_empty_env<F>(get_env: F, key: &str) -> Option<OsString>
where
    F: Fn(&str) -> Option<OsString> + Copy,
{
    let value = get_env(key)?;
    if value.is_empty() {
        return None;
    }
    if value.to_str().is_some_and(|value| value.trim().is_empty()) {
        return None;
    }
    Some(value)
}

fn env_value(key: &str) -> Option<OsString> {
    env::var_os(key)
}

fn migration_record(path_class: &str, source: &Path, destination: &Path, action: &str) -> Value {
    json!({
        "version": "cmdrvl.migration.v1",
        "tool": TOOL_NAME,
        "path_class": path_class,
        "source_path": source.display().to_string(),
        "destination_path": destination.display().to_string(),
        "action": action,
        "outcome": "ok",
        "secret_values_recorded": false
    })
}

fn deprecation_record(
    path_class: &str,
    source: &Path,
    destination: &Path,
    action: &str,
    outcome: &str,
) -> Value {
    json!({
        "version": "cmdrvl.deprecated_path_notice.v1",
        "tool": TOOL_NAME,
        "path_class": path_class,
        "source_path": source.display().to_string(),
        "destination_path": destination.display().to_string(),
        "action": action,
        "outcome": outcome,
        "secret_values_recorded": false
    })
}

fn append_record_once(path: &Path, record: Value) -> io::Result<()> {
    if record_already_exists(path, &record)? {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        harden_directory(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{record}")?;
    Ok(())
}

fn record_already_exists(path: &Path, record: &Value) -> io::Result<bool> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Ok(false);
    };

    Ok(contents.lines().any(|line| {
        let Ok(existing) = serde_json::from_str::<Value>(line) else {
            return false;
        };

        existing.get("tool") == record.get("tool")
            && existing.get("path_class") == record.get("path_class")
            && existing.get("source_path") == record.get("source_path")
            && existing.get("destination_path") == record.get("destination_path")
            && existing.get("action") == record.get("action")
    }))
}

#[cfg(unix)]
fn harden_directory(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn harden_directory(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempHome {
        path: PathBuf,
    }

    impl TempHome {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos();
            let path = env::temp_dir().join(format!(
                "assess-paths-{name}-{}-{nanos}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("temp home should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn env_map<'a>(
        values: &'a [(&'a str, OsString)],
    ) -> impl Fn(&str) -> Option<OsString> + Copy + 'a {
        move |key| {
            values
                .iter()
                .find(|(candidate, _)| *candidate == key)
                .map(|(_, value)| value.clone())
        }
    }

    #[test]
    fn default_witness_path_prefers_explicit_env_override() {
        let values = [
            (WITNESS_ENV, OsString::from("/tmp/custom-witness.jsonl")),
            ("HOME", OsString::from("/tmp/home")),
        ];
        let path = default_witness_path_from_env(env_map(&values));

        assert_eq!(path, PathBuf::from("/tmp/custom-witness.jsonl"));
    }

    #[test]
    fn default_witness_path_uses_cmdrvl_root_when_env_missing() {
        let values = [("HOME", OsString::from("/tmp/home"))];
        let path = default_witness_path_from_env(env_map(&values));

        assert_eq!(
            path,
            PathBuf::from("/tmp/home/.cmdrvl/state/witness/witness.jsonl")
        );
    }

    #[test]
    fn policy_dir_migration_copies_legacy_files_without_deleting_source() {
        let dir = TempHome::new("policy");
        let home = dir.path();
        let legacy = home.join(".epistemic").join("policies");
        fs::create_dir_all(legacy.join("nested")).unwrap();
        fs::write(legacy.join("custom.yaml"), "policy_id: custom\n").unwrap();
        fs::write(
            legacy.join("nested").join("extra.yml"),
            "policy_id: nested\n",
        )
        .unwrap();
        let values = [("HOME", home.as_os_str().to_os_string())];

        ensure_policy_dir_migrated_from_env(env_map(&values)).unwrap();

        let canonical = home.join(".cmdrvl/config/assess/policies");
        assert_eq!(
            fs::read_to_string(canonical.join("custom.yaml")).unwrap(),
            "policy_id: custom\n"
        );
        assert_eq!(
            fs::read_to_string(canonical.join("nested").join("extra.yml")).unwrap(),
            "policy_id: nested\n"
        );
        assert!(legacy.join("custom.yaml").exists());

        let migration_log = fs::read_to_string(home.join(".cmdrvl/migrations/applied.jsonl"))
            .expect("migration log should be written");
        assert!(migration_log.contains("\"path_class\":\"policy_directory\""));
        assert!(migration_log.contains("\"secret_values_recorded\":false"));
    }

    #[test]
    fn witness_migration_copies_legacy_file_without_deleting_source() {
        let dir = TempHome::new("witness");
        let home = dir.path();
        let legacy = home.join(".epistemic").join("witness.jsonl");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(&legacy, "{\"tool\":\"assess\"}\n").unwrap();
        let values = [("HOME", home.as_os_str().to_os_string())];

        ensure_witness_migrated_from_env(env_map(&values)).unwrap();

        let canonical = home.join(".cmdrvl/state/witness/witness.jsonl");
        assert_eq!(
            fs::read_to_string(canonical).unwrap(),
            "{\"tool\":\"assess\"}\n"
        );
        assert!(legacy.exists());

        let notices = fs::read_to_string(home.join(".cmdrvl/notices/deprecated-paths.jsonl"))
            .expect("deprecation notices should be written");
        assert!(notices.contains("\"path_class\":\"witness_ledger\""));
    }

    #[test]
    fn witness_migration_is_skipped_for_explicit_override() {
        let dir = TempHome::new("override");
        let home = dir.path();
        let override_path = home.join("operator-witness.jsonl");
        let values = [
            ("HOME", home.as_os_str().to_os_string()),
            (WITNESS_ENV, override_path.as_os_str().to_os_string()),
        ];

        ensure_witness_migrated_from_env(env_map(&values)).unwrap();

        assert_eq!(
            default_witness_path_from_env(env_map(&values)),
            override_path
        );
        assert!(!home.join(".cmdrvl").exists());
    }

    #[test]
    fn config_footprint_declares_policy_and_witness_paths() {
        let footprint = config_footprint();

        assert_eq!(footprint["tool"], TOOL_NAME);
        assert_eq!(footprint["managed_config_paths"][0], CANONICAL_POLICY_DIR);
        assert_eq!(footprint["managed_state_paths"][0], CANONICAL_WITNESS);
        assert_eq!(footprint["legacy_migration_required"], true);
        assert_eq!(footprint["self_contained"], true);
    }
}
