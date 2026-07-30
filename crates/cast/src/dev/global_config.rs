use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// The embedded default global cast config. Written to a new user's
/// `~/.config/cast/cast.json` when none exists, so harnesses fetch from the
/// numtide binary cache instead of building from source.
///
/// Under the hardened nix daemon (`trusted-users = root`), the dev container
/// user is non-trusted, so the global flake's `nixConfig` cache declarations
/// are no longer forwarded to the daemon. Caches now reach the daemon only via
/// `cast.json` -> `generate_nix_conf` -> daemon server-side config. Seeding a
/// default with numtide preserves zero-config first-run cache access.
pub(crate) const DEFAULT_CAST_JSON: &str = r#"{
  "nix_extra_substituters": ["https://cache.numtide.com"],
  "nix_extra_trusted_public_keys": [
    "niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g="
  ]
}
"#;

/// Directory (relative to the host home) holding the global cast config.
const GLOBAL_CONFIG_SUBDIR: &str = ".config/cast";

/// Scaffold the global cast config from the embedded default if it is absent.
///
/// Given the host home directory, ensures `<home>/.config/cast/cast.json`
/// exists. If it is missing, the directory tree is created and the embedded
/// default is written. Returns `Ok(true)` when a new config was scaffolded and
/// `Ok(false)` when one was already present (left untouched).
pub fn scaffold_if_missing(home_dir: &Path) -> Result<bool> {
    let config_dir = home_dir.join(GLOBAL_CONFIG_SUBDIR);
    let config_path = config_dir.join("cast.json");

    if config_path.exists() {
        return Ok(false);
    }

    fs::create_dir_all(&config_dir)
        .with_context(|| format!("creating global cast config dir {}", config_dir.display()))?;
    fs::write(&config_path, DEFAULT_CAST_JSON)
        .with_context(|| format!("writing global cast config {}", config_path.display()))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn scaffolds_when_missing() {
        let home = TempDir::new().unwrap();
        let created = scaffold_if_missing(home.path()).unwrap();

        assert!(created, "expected a fresh scaffold to report true");
        let config = home.path().join(".config/cast/cast.json");
        assert!(config.exists(), "cast.json should have been written");

        let contents = fs::read_to_string(&config).unwrap();
        assert_eq!(contents, DEFAULT_CAST_JSON);
    }

    #[test]
    fn leaves_existing_untouched() {
        let home = TempDir::new().unwrap();
        let config_dir = home.path().join(".config/cast");
        fs::create_dir_all(&config_dir).unwrap();
        let config = config_dir.join("cast.json");
        fs::write(&config, "{}").unwrap();

        let created = scaffold_if_missing(home.path()).unwrap();

        assert!(!created, "expected an existing config to report false");
        let contents = fs::read_to_string(&config).unwrap();
        assert_eq!(contents, "{}", "existing cast.json must not be overwritten");
    }

    #[test]
    fn contains_numtide_cache() {
        assert!(DEFAULT_CAST_JSON.contains("cache.numtide.com"));
        assert!(DEFAULT_CAST_JSON.contains("niks3.numtide.com-1:"));
    }

    #[test]
    fn default_is_valid_json() {
        let parsed: serde_json::Value =
            serde_json::from_str(DEFAULT_CAST_JSON).expect("DEFAULT_CAST_JSON must be valid JSON");
        assert!(parsed.get("nix_extra_substituters").is_some());
        assert!(parsed.get("nix_extra_trusted_public_keys").is_some());
    }
}
