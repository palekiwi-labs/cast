use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheEntry {
    pub version: String,
    /// Nanoseconds since Unix epoch
    pub fetched_at: u64,
}

/// Returns the standard path to the version cache file
pub fn get_cache_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("ocx")
        .join("version-cache.json")
}

/// Read a valid (non-expired) cache entry from the given path.
/// Returns `None` if the file does not exist, cannot be parsed, or is older
/// than `ttl_hours`.
pub fn read_cache(path: &Path, ttl_hours: u32) -> Option<CacheEntry> {
    let raw = std::fs::read_to_string(path).ok()?;
    let entry: CacheEntry = serde_json::from_str(&raw).ok()?;
    let age_nanos = now_nanos().saturating_sub(entry.fetched_at);
    let ttl_nanos = ttl_hours as u64 * 3600 * 1_000_000_000;
    if age_nanos >= ttl_nanos {
        return None;
    }
    Some(entry)
}

/// Write a cache entry to the given path, creating parent directories as needed.
/// Performs an atomic write using a temporary file.
pub fn write_cache(path: &Path, version: &str) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;

    let entry = CacheEntry {
        version: version.to_string(),
        fetched_at: now_nanos(),
    };
    let json = serde_json::to_string(&entry)?;

    let mut temp = NamedTempFile::new_in(parent)?;
    temp.write_all(json.as_bytes())?;
    temp.persist(path)?;

    Ok(())
}

/// Return the current time as nanoseconds since the Unix epoch.
pub fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn cache_path(dir: &TempDir) -> PathBuf {
        dir.path().join("version-cache.json")
    }

    // --- write_cache ---

    #[test]
    fn test_write_cache_creates_file() {
        let dir = TempDir::new().unwrap();
        let path = cache_path(&dir);

        write_cache(&path, "1.4.7").unwrap();

        assert!(path.exists());
    }

    #[test]
    fn test_write_cache_stores_version() {
        let dir = TempDir::new().unwrap();
        let path = cache_path(&dir);

        write_cache(&path, "1.4.7").unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        let entry: CacheEntry = serde_json::from_str(&raw).unwrap();
        assert_eq!(entry.version, "1.4.7");
    }

    #[test]
    fn test_write_cache_stores_nanosecond_timestamp() {
        let dir = TempDir::new().unwrap();
        let path = cache_path(&dir);
        let before = now_nanos();

        write_cache(&path, "1.4.7").unwrap();

        let after = now_nanos();
        let raw = fs::read_to_string(&path).unwrap();
        let entry: CacheEntry = serde_json::from_str(&raw).unwrap();
        assert!(entry.fetched_at >= before);
        assert!(entry.fetched_at <= after);
    }

    #[test]
    fn test_write_cache_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir
            .path()
            .join("nested")
            .join("dir")
            .join("version-cache.json");

        write_cache(&path, "1.0.0").unwrap();

        assert!(path.exists());
    }

    // --- read_cache ---

    #[test]
    fn test_read_cache_returns_none_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let path = cache_path(&dir);

        let result = read_cache(&path, 24);

        assert!(result.is_none());
    }

    #[test]
    fn test_read_cache_returns_entry_within_ttl() {
        let dir = TempDir::new().unwrap();
        let path = cache_path(&dir);
        write_cache(&path, "1.4.7").unwrap();

        let result = read_cache(&path, 24);

        assert!(result.is_some());
        assert_eq!(result.unwrap().version, "1.4.7");
    }

    #[test]
    fn test_read_cache_returns_none_when_expired() {
        let dir = TempDir::new().unwrap();
        let path = cache_path(&dir);

        // Write a stale entry: fetched_at = 48 hours ago in nanoseconds
        let stale_nanos = now_nanos() - (48u64 * 3600 * 1_000_000_000);
        let entry = CacheEntry {
            version: "1.0.0".to_string(),
            fetched_at: stale_nanos,
        };
        fs::write(&path, serde_json::to_string(&entry).unwrap()).unwrap();

        let result = read_cache(&path, 24);

        assert!(result.is_none());
    }

    #[test]
    fn test_read_cache_returns_none_on_corrupt_file() {
        let dir = TempDir::new().unwrap();
        let path = cache_path(&dir);
        fs::write(&path, "not valid json").unwrap();

        let result = read_cache(&path, 24);

        assert!(result.is_none());
    }
}
