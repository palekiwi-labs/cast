use anyhow::{bail, Result};
use std::path::Path;

use crate::opencode::version::cache;
use crate::opencode::version::github::VersionFetcher;

/// Resolve a raw version string to a concrete semver string.
///
/// - If `version` is already a valid semver, normalize and return it.
/// - If `version` is `"latest"`, check the cache first; fall back to the
///   fetcher if the cache is missing or expired.
/// - Network errors are non-fatal when a stale cache entry is present.
pub fn resolve_version<F: VersionFetcher>(
    version: &str,
    ttl_hours: u32,
    cache_path: &Path,
    fetcher: &F,
) -> Result<String> {
    let normalized = normalize_version(version);

    if normalized != "latest" {
        if !validate_semver(&normalized) {
            bail!(
                "Invalid version '{}': must be 'latest' or MAJOR.MINOR.PATCH",
                version
            );
        }
        return Ok(normalized);
    }

    // Try fresh cache first
    if let Some(entry) = cache::read_cache(cache_path, ttl_hours) {
        return Ok(entry.version);
    }

    // Attempt network fetch
    match fetcher.fetch_latest_version() {
        Ok(fetched) => {
            let resolved = normalize_version(&fetched);
            if let Err(e) = cache::write_cache(cache_path, &resolved) {
                eprintln!("Warning: Failed to write version cache: {}", e);
            }
            Ok(resolved)
        }
        Err(fetch_err) => {
            // Soft fallback: read stale cache entry ignoring TTL
            if let Ok(raw) = std::fs::read_to_string(cache_path)
                && let Ok(entry) = serde_json::from_str::<cache::CacheEntry>(&raw)
            {
                eprintln!(
                    "Warning: Failed to reach GitHub ({}). Falling back to cached version '{}'.",
                    fetch_err, entry.version
                );
                return Ok(entry.version);
            }
            bail!(
                "Failed to resolve latest version: {}. No cached version available.",
                fetch_err
            )
        }
    }
}

/// Normalize a raw version string: strip a leading `v` prefix.
/// `"latest"` is returned unchanged.
pub fn normalize_version(version: &str) -> String {
    let trimmed = version.trim();
    if trimmed == "latest" {
        return "latest".to_string();
    }
    trimmed.strip_prefix('v').unwrap_or(trimmed).to_string()
}

/// Validate that a string is a three-part semver
/// (`MAJOR.MINOR.PATCH`, each part a non-empty integer).
pub fn validate_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts
        .iter()
        .all(|p| !p.is_empty() && p.parse::<u64>().is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opencode::version::cache::{write_cache, CacheEntry};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // --- test doubles ---

    struct OkFetcher(String);
    impl VersionFetcher for OkFetcher {
        fn fetch_latest_version(&self) -> anyhow::Result<String> {
            Ok(self.0.clone())
        }
    }

    struct FailFetcher;
    impl VersionFetcher for FailFetcher {
        fn fetch_latest_version(&self) -> anyhow::Result<String> {
            anyhow::bail!("network error")
        }
    }

    fn tmp_cache(dir: &TempDir) -> PathBuf {
        dir.path().join("version-cache.json")
    }

    // --- resolve_version ---

    #[test]
    fn test_resolve_explicit_version_is_returned_normalized() {
        let dir = TempDir::new().unwrap();
        let path = tmp_cache(&dir);
        let fetcher = FailFetcher;

        let result = resolve_version("v1.4.7", 24, &path, &fetcher).unwrap();
        assert_eq!(result, "1.4.7");
    }

    #[test]
    fn test_resolve_explicit_version_does_not_touch_cache() {
        let dir = TempDir::new().unwrap();
        let path = tmp_cache(&dir);
        let fetcher = FailFetcher;

        resolve_version("1.0.0", 24, &path, &fetcher).unwrap();

        assert!(
            !path.exists(),
            "cache must not be written for explicit versions"
        );
    }

    #[test]
    fn test_resolve_invalid_explicit_version_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = tmp_cache(&dir);
        let fetcher = FailFetcher;

        let result = resolve_version("not-a-version", 24, &path, &fetcher);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_latest_fetches_and_caches_when_no_cache() {
        let dir = TempDir::new().unwrap();
        let path = tmp_cache(&dir);
        let fetcher = OkFetcher("1.9.0".to_string());

        let result = resolve_version("latest", 24, &path, &fetcher).unwrap();
        assert_eq!(result, "1.9.0");
        assert!(path.exists(), "cache must be written after fetch");
    }

    #[test]
    fn test_resolve_latest_strips_v_prefix_from_fetched_version() {
        let dir = TempDir::new().unwrap();
        let path = tmp_cache(&dir);
        let fetcher = OkFetcher("v2.0.1".to_string());

        let result = resolve_version("latest", 24, &path, &fetcher).unwrap();
        assert_eq!(result, "2.0.1");
    }

    #[test]
    fn test_resolve_latest_uses_cache_when_fresh() {
        let dir = TempDir::new().unwrap();
        let path = tmp_cache(&dir);
        write_cache(&path, "1.2.3").unwrap();

        // fetcher would return a different version if called
        let fetcher = OkFetcher("9.9.9".to_string());

        let result = resolve_version("latest", 24, &path, &fetcher).unwrap();
        assert_eq!(result, "1.2.3");
    }

    #[test]
    fn test_resolve_latest_fetches_when_cache_expired() {
        let dir = TempDir::new().unwrap();
        let path = tmp_cache(&dir);

        // Write a stale entry
        let stale_nanos = crate::opencode::version::cache::now_nanos() - (48u64 * 3600 * 1_000_000_000);
        let stale = CacheEntry {
            version: "0.0.1".to_string(),
            fetched_at: stale_nanos,
        };
        fs::write(&path, serde_json::to_string(&stale).unwrap()).unwrap();

        let fetcher = OkFetcher("3.0.0".to_string());
        let result = resolve_version("latest", 24, &path, &fetcher).unwrap();
        assert_eq!(result, "3.0.0");
    }

    #[test]
    fn test_resolve_latest_falls_back_to_stale_cache_on_network_error() {
        let dir = TempDir::new().unwrap();
        let path = tmp_cache(&dir);

        // Write a stale (expired) entry
        let stale_nanos = crate::opencode::version::cache::now_nanos() - (48u64 * 3600 * 1_000_000_000);
        let stale = CacheEntry {
            version: "1.1.1".to_string(),
            fetched_at: stale_nanos,
        };
        fs::write(&path, serde_json::to_string(&stale).unwrap()).unwrap();

        let result = resolve_version("latest", 24, &path, &FailFetcher).unwrap();
        assert_eq!(result, "1.1.1");
    }

    #[test]
    fn test_resolve_latest_errors_when_no_cache_and_network_fails() {
        let dir = TempDir::new().unwrap();
        let path = tmp_cache(&dir);

        let result = resolve_version("latest", 24, &path, &FailFetcher);
        assert!(result.is_err());
    }

    // --- validate_semver ---

    #[test]
    fn test_validate_semver_rejects_latest() {
        assert!(!validate_semver("latest"));
    }

    #[test]
    fn test_validate_semver_accepts_three_part_version() {
        assert!(validate_semver("1.4.7"));
    }

    #[test]
    fn test_validate_semver_accepts_zero_versions() {
        assert!(validate_semver("0.0.0"));
    }

    #[test]
    fn test_validate_semver_rejects_v_prefix() {
        assert!(!validate_semver("v1.4.7"));
    }

    #[test]
    fn test_validate_semver_rejects_two_parts() {
        assert!(!validate_semver("1.4"));
    }

    #[test]
    fn test_validate_semver_rejects_four_parts() {
        assert!(!validate_semver("1.4.7.1"));
    }

    #[test]
    fn test_validate_semver_rejects_non_numeric() {
        assert!(!validate_semver("1.4.x"));
    }

    #[test]
    fn test_validate_semver_rejects_empty_parts() {
        assert!(!validate_semver("1..7"));
    }

    // --- normalize_version ---

    #[test]
    fn test_normalize_strips_v_prefix() {
        assert_eq!(normalize_version("v1.4.7"), "1.4.7");
    }

    #[test]
    fn test_normalize_leaves_bare_version_unchanged() {
        assert_eq!(normalize_version("1.4.7"), "1.4.7");
    }

    #[test]
    fn test_normalize_leaves_latest_unchanged() {
        assert_eq!(normalize_version("latest"), "latest");
    }

    #[test]
    fn test_normalize_trims_whitespace() {
        assert_eq!(normalize_version("  v1.2.3  "), "1.2.3");
    }
}
