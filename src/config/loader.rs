use super::Config;
use anyhow::{Context, Result};
use figment::{
    providers::{Env, Format, Json, Serialized},
    Figment,
};
use std::path::PathBuf;

/// Load configuration from all sources with proper precedence:
/// 1. Environment variables (OCX_*)
/// 2. Project config (./ocx.json)
/// 3. Global config (~/.config/ocx/ocx.json)
/// 4. Defaults
///
/// Environment variable format:
/// - Use single underscore for field names: OCX_NIX_VOLUME_NAME → nix_volume_name
/// - Use double underscore for nesting: OCX_EXTRA_DATA_VOLUMES__CARGO__TARGET → extra_data_volumes.cargo.target
pub fn load_config() -> Result<Config> {
    let mut figment = Figment::new().merge(Serialized::defaults(Config::default()));

    if let Some(global_path) = global_config_path() {
        figment = figment.merge(Json::file(global_path));
    }

    figment
        .merge(Json::file("ocx.json"))
        .merge(Env::prefixed("OCX_").split("__"))
        .extract()
        .context("Failed to load configuration")
}

/// Resolve the global config path (~/.config/ocx/ocx.json)
/// Returns None if the system config directory cannot be determined
fn global_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("ocx").join("ocx.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_succeeds() {
        // Just verify it loads without error
        // Don't test specific values since they depend on user's environment
        let config = load_config().unwrap();

        // Basic sanity checks - these fields should always have some value
        assert!(!config.opencode_version.is_empty());
        assert!(!config.memory.is_empty());
        assert!(config.cpus > 0.0);
    }

    #[test]
    fn test_config_default_has_expected_values() {
        // Test the defaults in isolation
        let config = Config::default();

        assert_eq!(config.opencode_version, "latest");
        assert_eq!(config.memory, "1024m");
        assert_eq!(config.cpus, 1.0);
    }
}
