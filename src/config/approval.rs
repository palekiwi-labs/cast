use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::NamedTempFile;

pub fn compute_config_hash(config: &Config, workspace_root: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::os::unix::ffi::OsStrExt;

    let canonical_root = std::fs::canonicalize(workspace_root)
        .context("Failed to canonicalize workspace root path")?;

    let config_bytes = serde_json::to_vec(config)?;

    let mut hasher = Sha256::new();
    hasher.update(canonical_root.as_os_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(&config_bytes);

    Ok(hex::encode(hasher.finalize()))
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

        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(parent)?;
        }
        #[cfg(not(unix))]
        std::fs::create_dir_all(parent)?;

        let json =
            serde_json::to_string_pretty(self).context("Failed to serialize approval store")?;

        let mut temp = NamedTempFile::new_in(parent).context("Failed to create temporary file")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o600))?;
        }

        temp.write_all(json.as_bytes())
            .context("Failed to write to temporary file")?;
        temp.persist(path)
            .context("Failed to persist approval store")?;

        Ok(())
    }

    pub fn is_approved(&self, hash: &str) -> bool {
        self.entries.contains_key(hash)
    }

    /// Verify that the given configuration and workspace are approved.
    /// Returns an `ApprovedConfig` on success, or an error if not approved.
    pub fn verify(&self, config: Config, workspace_root: &Path) -> Result<ApprovedConfig> {
        let hash = compute_config_hash(&config, workspace_root)?;
        if self.is_approved(&hash) {
            Ok(ApprovedConfig(config))
        } else {
            anyhow::bail!(
                "Configuration has not been approved for this project.\n\
                 Note: env-var overrides (CAST_*) affect the hash.\n\
                 Review with `cast config show`, then run `cast config allow` to approve."
            );
        }
    }

    pub fn add_entry(&mut self, hash: String, workspace: String) {
        let approved_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.entries.insert(
            hash,
            ApprovalEntry {
                workspace,
                approved_at,
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

        store.add_entry(hash.clone(), workspace);
        assert!(store.is_approved(&hash));

        store.remove_entry(&hash);
        assert!(!store.is_approved(&hash));
    }

    #[test]
    fn test_approval_store_persistence_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("approvals.json");

        let mut store = ApprovalStore::default();
        store.add_entry("hash1".into(), "/project1".into());
        store.save_to(&path).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let loaded: ApprovalStore = serde_json::from_str(&raw).unwrap();
        assert!(loaded.is_approved("hash1"));

        // Test restricted permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&path).unwrap();
            assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        }
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
        store.add_entry(hash, path.display().to_string());

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
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Configuration has not been approved"));
    }
}
