use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Recursively sort all JSON object keys for deterministic serialization.
fn canonicalize_value(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let sorted: serde_json::Map<_, _> = map
                .into_iter()
                .map(|(k, v)| (k, canonicalize_value(v)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect();
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(canonicalize_value).collect())
        }
        other => other,
    }
}

pub fn compute_config_hash(config: &Config, workspace_root: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::os::unix::ffi::OsStrExt;

    let canonical_root = std::fs::canonicalize(workspace_root)
        .context("Failed to canonicalize workspace root path")?;

    let json_value = serde_json::to_value(config)?;
    let canonical_json = canonicalize_value(json_value);
    let config_bytes = serde_json::to_vec(&canonical_json)?;

    let mut hasher = Sha256::new();
    hasher.update(canonical_root.as_os_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(&config_bytes);

    Ok(hex::encode(hasher.finalize()))
}

pub fn load_approval_store() -> Result<ApprovalStore> {
    todo!()
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
    pub fn is_approved(&self, hash: &str) -> bool {
        self.entries.contains_key(hash)
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
        let mut c2 = Config::default();
        c2.memory = "2048m".to_string();

        let h1 = compute_config_hash(&c1, path).unwrap();
        let h2 = compute_config_hash(&c2, path).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_map_ordering_invariance() {
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
}
