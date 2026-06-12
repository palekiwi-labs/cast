use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::NamedTempFile;

/// The approval status of a configuration for a given workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStatus {
    /// Hash matches — config is approved for this workspace.
    Approved,
    /// Workspace has a prior approval, but the current hash doesn't match (config changed).
    Changed,
    /// No approval entry exists for this workspace at all.
    Unapproved,
}

/// Compute a config hash from an already-canonicalized workspace root.
fn compute_config_hash_canonical(config: &Config, canonical_root: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::os::unix::ffi::OsStrExt;

    let config_bytes = serde_json::to_vec(config)?;

    let mut hasher = Sha256::new();
    hasher.update(canonical_root.as_os_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(&config_bytes);

    Ok(hex::encode(hasher.finalize()))
}

pub fn compute_config_hash(config: &Config, workspace_root: &Path) -> Result<String> {
    let canonical_root = std::fs::canonicalize(workspace_root)
        .context("Failed to canonicalize workspace root path")?;
    compute_config_hash_canonical(config, &canonical_root)
}

/// A validated configuration that has been explicitly approved by the user.
///
/// This type can only be constructed by the `ApprovalStore::verify` method,
/// creating a compiler-enforced "gate" for functions that require an approved
/// environment (like starting an agent session).
#[derive(Debug, Clone)]
pub struct ApprovedConfig(Config);

impl std::ops::Deref for ApprovedConfig {
    type Target = Config;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ApprovedConfig {
    /// Unwrap the approved configuration.
    pub fn into_inner(self) -> Config {
        self.0
    }

    /// UNSAFE: Wrap a raw configuration without user approval.
    /// ONLY for use in tests to bypass the approval gate.
    #[cfg(test)]
    pub fn assume_approved_for_test(config: Config) -> Self {
        Self(config)
    }
}

pub fn approval_store_path() -> PathBuf {
    if let Ok(dir) = std::env::var("CAST_DATA_DIR") {
        return PathBuf::from(dir).join("approved_configs.json");
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from(".local").join("share"))
        .join("cast")
        .join("approved_configs.json")
}

pub fn load_approval_store() -> Result<ApprovalStore> {
    ApprovalStore::load_from(&approval_store_path())
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ApprovalStore {
    pub entries: BTreeMap<String, ApprovalEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApprovalEntry {
    pub workspace: String,
    pub approved_at: u64,
    pub approved_config: serde_json::Value,
}

impl ApprovalStore {
    pub fn load_from(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(raw) => Ok(serde_json::from_str(&raw).context(
                "Failed to parse approval store. The file may be corrupted. \
                 Try manually fixing or deleting ~/.local/share/cast/approved_configs.json",
            )?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ApprovalStore::default()),
            Err(e) => Err(e).context("Failed to read approval store"),
        }
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&approval_store_path())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let parent = path.parent().context("Invalid approval store path")?;

        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;

        let json = serde_json::to_string(self).context("Failed to serialize approval store")?;

        let mut temp = NamedTempFile::new_in(parent).context("Failed to create temporary file")?;

        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o600))?;

        temp.write_all(json.as_bytes())
            .context("Failed to write to temporary file")?;
        temp.persist(path)
            .context("Failed to persist approval store")?;

        Ok(())
    }

    pub fn is_approved(&self, hash: &str) -> bool {
        self.entries.contains_key(hash)
    }

    /// Find the approval entry for a given canonical workspace path.
    pub fn find_by_workspace(&self, canonical_path: &str) -> Option<&ApprovalEntry> {
        self.entries
            .values()
            .find(|entry| entry.workspace == canonical_path)
    }

    /// Return the approval status for a given hash and canonical workspace path.
    ///
    /// The workspace is cross-checked against the matched entry to guard against
    /// hash collisions and hand-edited approval files.
    pub fn check_status(&self, hash: &str, canonical_workspace: &str) -> ApprovalStatus {
        if let Some(entry) = self.entries.get(hash)
            && entry.workspace == canonical_workspace
        {
            return ApprovalStatus::Approved;
        }
        if self.find_by_workspace(canonical_workspace).is_some() {
            ApprovalStatus::Changed
        } else {
            ApprovalStatus::Unapproved
        }
    }

    /// Verify that the given configuration and workspace are approved.
    /// Returns an `ApprovedConfig` on success, or an error if not approved.
    pub fn verify(&self, config: Config, workspace_root: &Path) -> Result<ApprovedConfig> {
        let canonical = std::fs::canonicalize(workspace_root)
            .context("Failed to canonicalize workspace root path")?;
        let canonical_str = canonical.to_string_lossy();
        let hash = compute_config_hash_canonical(&config, &canonical)?;

        match self.check_status(&hash, &canonical_str) {
            ApprovalStatus::Approved => Ok(ApprovedConfig(config)),
            ApprovalStatus::Changed => anyhow::bail!(
                "Configuration has changed since last approval.\n\
                 Note: env-var overrides (CAST_*) affect the hash.\n\
                 Run `cast config diff` to see what changed, then `cast config allow` to approve."
            ),
            ApprovalStatus::Unapproved => anyhow::bail!(
                "Configuration has not been approved for this project.\n\
                 Run `cast config allow` to approve the current configuration."
            ),
        }
    }

    pub fn add_entry(&mut self, hash: String, workspace: String, config: serde_json::Value) {
        // Ensure exactly 1 approved version per workspace by removing any existing entries
        self.remove_workspace_entries(&workspace);

        let approved_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.entries.insert(
            hash,
            ApprovalEntry {
                workspace,
                approved_at,
                approved_config: config,
            },
        );
    }

    pub fn remove_entry(&mut self, hash: &str) {
        self.entries.remove(hash);
    }

    /// Remove all approval entries associated with a specific workspace path.
    pub fn remove_workspace_entries(&mut self, workspace_path: &str) {
        self.entries
            .retain(|_, entry| entry.workspace != workspace_path);
    }
}

/// Loads the store, approves the config for the given workspace, and saves it.
pub fn approve_workspace_config(config: &Config, workspace_root: &Path) -> Result<()> {
    let canonical_root =
        std::fs::canonicalize(workspace_root).context("Failed to canonicalize workspace root")?;
    let hash = compute_config_hash_canonical(config, &canonical_root)?;
    let snapshot = serde_json::to_value(config).context("Failed to serialize config snapshot")?;
    let mut store = load_approval_store()?;
    store.add_entry(
        hash,
        canonical_root.to_string_lossy().into_owned(),
        snapshot,
    );
    store.save()
}

/// Loads the store, revokes all approvals for the given workspace, and saves it.
pub fn deny_workspace_config(workspace_root: &Path) -> Result<()> {
    let canonical_root =
        std::fs::canonicalize(workspace_root).context("Failed to canonicalize workspace root")?;
    let mut store = load_approval_store()?;
    store.remove_workspace_entries(&canonical_root.to_string_lossy());
    store.save()
}

/// Helper to load the store and verify a config in one step.
pub fn check_approved(config: Config, workspace_root: &Path) -> Result<ApprovedConfig> {
    let store = load_approval_store()?;
    store.verify(config, workspace_root)
}

/// Return the approval status of the current config for a workspace.
///
/// Convenience wrapper used by `cast config show` to decide which hint to emit.
pub fn get_approval_status(config: &Config, workspace_root: &Path) -> Result<ApprovalStatus> {
    let store = load_approval_store()?;
    get_approval_status_with(config, workspace_root, &store)
}

fn get_approval_status_with(
    config: &Config,
    workspace_root: &Path,
    store: &ApprovalStore,
) -> Result<ApprovalStatus> {
    let canonical = std::fs::canonicalize(workspace_root)
        .context("Failed to canonicalize workspace root path")?;
    let canonical_str = canonical.to_string_lossy();
    let hash = compute_config_hash_canonical(config, &canonical)?;
    Ok(store.check_status(&hash, &canonical_str))
}

/// The result of comparing the current configuration against the last approved snapshot.
#[derive(Debug, PartialEq)]
pub enum ConfigDiffOutput {
    /// No approval entry exists for this workspace.
    Unapproved,
    /// Config matches the approved snapshot — nothing to show.
    Unchanged,
    /// Config has changed; the plain unified diff string is enclosed.
    Changed(String),
}

/// Compute the diff between the current config and the last approved snapshot.
///
/// Convenience wrapper used by `cast config diff`.
pub fn compute_workspace_diff(config: &Config, workspace_root: &Path) -> Result<ConfigDiffOutput> {
    let store = load_approval_store()?;
    compute_workspace_diff_with(config, workspace_root, &store)
}

fn compute_workspace_diff_with(
    config: &Config,
    workspace_root: &Path,
    store: &ApprovalStore,
) -> Result<ConfigDiffOutput> {
    use crate::config::diff::format_config_diff;

    let canonical = std::fs::canonicalize(workspace_root)
        .context("Failed to canonicalize workspace root path")?;
    let canonical_str = canonical.to_string_lossy();

    let Some(entry) = store.find_by_workspace(&canonical_str) else {
        return Ok(ConfigDiffOutput::Unapproved);
    };

    let current_hash = compute_config_hash_canonical(config, &canonical)?;
    let is_unchanged = store
        .entries
        .get(&current_hash)
        .map(|e| e.workspace == canonical_str)
        .unwrap_or(false);

    if is_unchanged {
        return Ok(ConfigDiffOutput::Unchanged);
    }

    let current_value = serde_json::to_value(config)?;
    let diff = format_config_diff(&entry.approved_config, &current_value);

    if diff.is_empty() {
        Ok(ConfigDiffOutput::Unchanged)
    } else {
        Ok(ConfigDiffOutput::Changed(diff))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VolumeConfig;
    use tempfile::TempDir;

    #[test]
    fn test_hash_stability() {
        let config = Config::default();
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();

        let h1 = compute_config_hash(&config, path).unwrap();
        let h2 = compute_config_hash(&config, path).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_path_sensitivity() {
        let config = Config::default();
        let tmp1 = TempDir::new().unwrap();
        let tmp2 = TempDir::new().unwrap();

        let h1 = compute_config_hash(&config, tmp1.path()).unwrap();
        let h2 = compute_config_hash(&config, tmp2.path()).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_config_sensitivity() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();

        let c1 = Config::default();
        let c2 = Config {
            memory: "2048m".to_string(),
            ..Config::default()
        };

        let h1 = compute_config_hash(&c1, path).unwrap();
        let h2 = compute_config_hash(&c2, path).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_config_hash_determinism() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();

        let mut c1 = Config::default();
        let mut c2 = Config::default();

        let vol1 = VolumeConfig {
            target: "/a".into(),
            source: None,
            mode: "rw".into(),
            volume_type: "volume".into(),
        };
        let vol2 = VolumeConfig {
            target: "/b".into(),
            source: None,
            mode: "rw".into(),
            volume_type: "volume".into(),
        };

        // Insert in different order
        c1.extra_data_volumes.insert("vol_a".into(), vol1.clone());
        c1.extra_data_volumes.insert("vol_b".into(), vol2.clone());

        c2.extra_data_volumes.insert("vol_b".into(), vol2);
        c2.extra_data_volumes.insert("vol_a".into(), vol1);

        let h1 = compute_config_hash(&c1, path).unwrap();
        let h2 = compute_config_hash(&c2, path).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_approval_store_in_memory() {
        let mut store = ApprovalStore::default();
        let hash = "abc123".to_string();
        let workspace = "/home/user/project".to_string();

        assert!(!store.is_approved(&hash));

        store.add_entry(hash.clone(), workspace, serde_json::Value::Null);
        assert!(store.is_approved(&hash));

        store.remove_entry(&hash);
        assert!(!store.is_approved(&hash));
    }

    #[test]
    fn test_approval_store_persistence_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("approvals.json");

        let mut store = ApprovalStore::default();
        store.add_entry("hash1".into(), "/project1".into(), serde_json::Value::Null);
        store.save_to(&path).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let loaded: ApprovalStore = serde_json::from_str(&raw).unwrap();
        assert!(loaded.is_approved("hash1"));

        // Test restricted permissions on Unix
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(&path).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn test_load_approval_store_missing_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("missing.json");
        // load_approval_store usually uses global path, so I'll test the internal loader helper if I make one,
        // or just mock it. Let's add a `load_from` for testing.
        let store = ApprovalStore::load_from(&path).unwrap();
        assert!(store.entries.is_empty());
    }

    #[test]
    fn test_load_approval_store_corrupt_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("corrupt.json");
        std::fs::write(&path, "not json").unwrap();
        let result = ApprovalStore::load_from(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_approved_config() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        let config = Config::default();
        let hash = compute_config_hash(&config, path).unwrap();

        let mut store = ApprovalStore::default();
        store.add_entry(hash, path.display().to_string(), serde_json::Value::Null);

        let result = store.verify(config, path);
        assert!(result.is_ok());
        let approved = result.unwrap();
        assert_eq!(approved.memory, Config::default().memory);
    }

    #[test]
    fn test_verify_unapproved_config_fails() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        let config = Config::default();

        let store = ApprovalStore::default();
        let result = store.verify(config, path);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Configuration has not been approved"),
            "unexpected message: {}",
            msg
        );
        // Should NOT suggest `config diff` when there's no prior approval
        assert!(
            !msg.contains("config diff"),
            "should not mention config diff for never-approved workspace: {}",
            msg
        );
    }

    #[test]
    fn test_verify_changed_config_fails_with_diff_hint() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();

        // Approve config A
        let config_a = Config::default();
        let hash_a = compute_config_hash(&config_a, path).unwrap();
        let canonical = std::fs::canonicalize(path).unwrap();
        let workspace = canonical.to_string_lossy().into_owned();

        let mut store = ApprovalStore::default();
        store.add_entry(hash_a, workspace, serde_json::Value::Null);

        // Try to verify config B (different from A)
        let config_b = Config {
            memory: "2048m".to_string(),
            ..Config::default()
        };
        let result = store.verify(config_b, path);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Configuration has changed"),
            "unexpected message: {}",
            msg
        );
        assert!(
            msg.contains("config diff"),
            "should suggest config diff for changed config: {}",
            msg
        );
    }

    #[test]
    fn test_check_status_approved() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        let config = Config::default();
        let canonical = std::fs::canonicalize(path).unwrap();
        let workspace = canonical.to_string_lossy().into_owned();
        let hash = compute_config_hash(&config, path).unwrap();

        let mut store = ApprovalStore::default();
        store.add_entry(hash.clone(), workspace.clone(), serde_json::Value::Null);

        assert_eq!(
            store.check_status(&hash, &workspace),
            ApprovalStatus::Approved
        );
    }

    #[test]
    fn test_check_status_changed() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        let canonical = std::fs::canonicalize(path).unwrap();
        let workspace = canonical.to_string_lossy().into_owned();

        let config_a = Config::default();
        let hash_a = compute_config_hash(&config_a, path).unwrap();

        let mut store = ApprovalStore::default();
        store.add_entry(hash_a, workspace.clone(), serde_json::Value::Null);

        // Different hash, same workspace
        let hash_b = "deadbeef".to_string();
        assert_eq!(
            store.check_status(&hash_b, &workspace),
            ApprovalStatus::Changed
        );
    }

    #[test]
    fn test_check_status_unapproved() {
        let store = ApprovalStore::default();
        assert_eq!(
            store.check_status("anyhash", "/some/workspace"),
            ApprovalStatus::Unapproved
        );
    }

    #[test]
    fn test_check_status_hash_collision_guard() {
        // Hash matches an entry but workspace differs — should NOT return Approved
        let mut store = ApprovalStore::default();
        store.add_entry(
            "collidehash".into(),
            "/workspace/a".into(),
            serde_json::Value::Null,
        );

        // Same hash, different workspace — falls through to Unapproved (no entry for /workspace/b)
        assert_eq!(
            store.check_status("collidehash", "/workspace/b"),
            ApprovalStatus::Unapproved
        );
    }

    #[test]
    fn test_approval_overwrite() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        let workspace = path.display().to_string();

        let mut store = ApprovalStore::default();

        // First approval
        let c1 = Config::default();
        let h1 = compute_config_hash(&c1, path).unwrap();
        store.add_entry(h1.clone(), workspace.clone(), serde_json::Value::Null);
        assert_eq!(store.entries.len(), 1);

        // Second approval for same workspace with different config
        let c2 = Config {
            memory: "2048m".to_string(),
            ..Config::default()
        };
        let h2 = compute_config_hash(&c2, path).unwrap();
        store.add_entry(h2.clone(), workspace.clone(), serde_json::Value::Null);

        assert_eq!(
            store.entries.len(),
            1,
            "Should have exactly 1 entry per workspace"
        );
        assert!(store.is_approved(&h2));
        assert!(!store.is_approved(&h1), "Old hash should have been removed");
    }

    #[test]
    fn test_deny_removes_all_entries() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();
        let workspace = path.display().to_string();

        let mut store = ApprovalStore::default();

        let config = Config::default();
        let hash = compute_config_hash(&config, path).unwrap();
        store.add_entry(hash.clone(), workspace.clone(), serde_json::Value::Null);

        assert!(store.is_approved(&hash));

        store.remove_workspace_entries(&workspace);
        assert!(!store.is_approved(&hash));
        assert_eq!(store.entries.len(), 0);
    }

    #[test]
    fn test_symlink_path_matching() {
        use std::os::unix::fs::symlink;

        // Use a persistent temp dir for this test
        let tmp = TempDir::new().unwrap();
        let real_path = tmp.path().join("real");
        std::fs::create_dir(&real_path).unwrap();
        let sym_path = tmp.path().join("sym");
        symlink(&real_path, &sym_path).unwrap();

        // Mock the approval store path to avoid polluting real user data
        let store_path = tmp.path().join("approvals.json");

        let config = Config::default();

        // Helper to simulate approve_workspace_config with a custom path
        let approve = |cfg: &Config, p: &Path| -> Result<()> {
            let canonical_root = std::fs::canonicalize(p)?;
            let hash = compute_config_hash_canonical(cfg, &canonical_root)?;
            let mut store = ApprovalStore::load_from(&store_path)?;
            store.add_entry(
                hash,
                canonical_root.to_string_lossy().into_owned(),
                serde_json::Value::Null,
            );
            store.save_to(&store_path)
        };

        // 1. Approve via real path
        approve(&config, &real_path).unwrap();

        // 2. Approve via symlink path
        approve(&config, &sym_path).unwrap();

        let config2 = Config {
            memory: "2048m".to_string(),
            ..Config::default()
        };
        approve(&config2, &sym_path).unwrap();

        let store = ApprovalStore::load_from(&store_path).unwrap();
        assert_eq!(
            store.entries.len(),
            1,
            "Should have exactly 1 entry even when accessed via symlinks"
        );
    }

    #[test]
    fn test_add_entry_stores_config_snapshot() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("approvals.json");

        let config = Config::default();
        let snapshot = serde_json::to_value(&config).unwrap();
        let workspace = "/home/user/project".to_string();

        let mut store = ApprovalStore::default();
        store.add_entry("hash1".into(), workspace.clone(), snapshot.clone());
        store.save_to(&path).unwrap();

        let loaded = ApprovalStore::load_from(&path).unwrap();
        let entry = loaded.entries.get("hash1").unwrap();
        assert_eq!(entry.approved_config, snapshot);
    }

    #[test]
    fn test_find_by_workspace_returns_entry() {
        let config = Config::default();
        let snapshot = serde_json::to_value(&config).unwrap();

        let mut store = ApprovalStore::default();
        store.add_entry("hash1".into(), "/workspace/a".into(), snapshot.clone());
        store.add_entry("hash2".into(), "/workspace/b".into(), snapshot);

        assert!(store.find_by_workspace("/workspace/a").is_some());
        assert!(store.find_by_workspace("/workspace/b").is_some());
        assert!(store.find_by_workspace("/workspace/c").is_none());
    }

    // --- get_approval_status ---
    //
    // Tests use the private `get_approval_status_with` helper to pass an
    // in-memory store, avoiding I/O to the global `CAST_DATA_DIR` path.

    #[test]
    fn test_get_approval_status_approved() {
        let tmp = TempDir::new().unwrap();
        let config = Config::default();
        let canonical = std::fs::canonicalize(tmp.path()).unwrap();
        let workspace = canonical.to_string_lossy().into_owned();
        let hash = compute_config_hash(&config, tmp.path()).unwrap();
        let snapshot = serde_json::to_value(&config).unwrap();

        let mut store = ApprovalStore::default();
        store.add_entry(hash, workspace, snapshot);

        assert_eq!(
            get_approval_status_with(&config, tmp.path(), &store).unwrap(),
            ApprovalStatus::Approved
        );
    }

    #[test]
    fn test_get_approval_status_changed() {
        let tmp = TempDir::new().unwrap();
        let config_a = Config::default();
        let canonical = std::fs::canonicalize(tmp.path()).unwrap();
        let workspace = canonical.to_string_lossy().into_owned();
        let hash_a = compute_config_hash(&config_a, tmp.path()).unwrap();
        let snapshot = serde_json::to_value(&config_a).unwrap();

        let mut store = ApprovalStore::default();
        store.add_entry(hash_a, workspace, snapshot);

        let config_b = Config {
            memory: "2048m".to_string(),
            ..Config::default()
        };
        assert_eq!(
            get_approval_status_with(&config_b, tmp.path(), &store).unwrap(),
            ApprovalStatus::Changed
        );
    }

    #[test]
    fn test_get_approval_status_unapproved() {
        let tmp = TempDir::new().unwrap();
        let config = Config::default();
        let store = ApprovalStore::default();
        assert_eq!(
            get_approval_status_with(&config, tmp.path(), &store).unwrap(),
            ApprovalStatus::Unapproved
        );
    }

    // --- compute_workspace_diff ---
    //
    // Tests use the private `compute_workspace_diff_with` helper for the
    // same reason.

    #[test]
    fn test_compute_workspace_diff_unapproved() {
        let tmp = TempDir::new().unwrap();
        let config = Config::default();
        let store = ApprovalStore::default();
        assert_eq!(
            compute_workspace_diff_with(&config, tmp.path(), &store).unwrap(),
            ConfigDiffOutput::Unapproved
        );
    }

    #[test]
    fn test_compute_workspace_diff_unchanged() {
        let tmp = TempDir::new().unwrap();
        let config = Config::default();
        let canonical = std::fs::canonicalize(tmp.path()).unwrap();
        let workspace = canonical.to_string_lossy().into_owned();
        let hash = compute_config_hash(&config, tmp.path()).unwrap();
        let snapshot = serde_json::to_value(&config).unwrap();

        let mut store = ApprovalStore::default();
        store.add_entry(hash, workspace, snapshot);

        assert_eq!(
            compute_workspace_diff_with(&config, tmp.path(), &store).unwrap(),
            ConfigDiffOutput::Unchanged
        );
    }

    #[test]
    fn test_compute_workspace_diff_changed() {
        let tmp = TempDir::new().unwrap();
        let config_a = Config::default();
        let canonical = std::fs::canonicalize(tmp.path()).unwrap();
        let workspace = canonical.to_string_lossy().into_owned();
        let hash_a = compute_config_hash(&config_a, tmp.path()).unwrap();
        let snapshot = serde_json::to_value(&config_a).unwrap();

        let mut store = ApprovalStore::default();
        store.add_entry(hash_a, workspace, snapshot);

        let config_b = Config {
            memory: "4096m".to_string(),
            ..Config::default()
        };
        match compute_workspace_diff_with(&config_b, tmp.path(), &store).unwrap() {
            ConfigDiffOutput::Changed(diff) => {
                assert!(diff.contains("1024m"), "diff should mention old value");
                assert!(diff.contains("4096m"), "diff should mention new value");
            }
            other => panic!("expected Changed, got {:?}", other),
        }
    }
}
