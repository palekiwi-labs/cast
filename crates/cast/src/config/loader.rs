use super::Config;
use crate::paths::home_config_dir;
use anyhow::{Context, Result};
use figment::{
    providers::{Env, Format, Json, Serialized},
    Figment,
};
use std::path::PathBuf;
use tracing::info;

/// Load configuration from all sources with proper precedence:
/// 1. Environment variables (CAST_*)
/// 2. MCP-specific project config (./cast-mcp.json)
/// 3. Project config (./cast.json)
/// 4. Global config (~/.config/cast/cast.json)
/// 5. Defaults
///
/// Environment variable format:
/// - Use single underscore for field names: CAST_NIX_VOLUME_NAME → nix_volume_name
/// - Use double underscore for nesting: CAST_EXTRA_DATA_VOLUMES__CARGO__TARGET → extra_data_volumes.cargo.target
pub fn load_config() -> Result<Config> {
    let cwd = std::env::current_dir().context("Failed to get current working directory")?;
    load_config_from(&cwd)
}

pub fn load_config_from(base_dir: &std::path::Path) -> Result<Config> {
    let mut figment = Figment::new().merge(Serialized::defaults(Config::default()));

    if let Some(global_path) = global_config_path() {
        figment = figment.merge(Json::file(global_path));
    }

    // Load cast-mcp.json into an intermediate Value.
    // This allows the file to have a flat structure (no root "mcp" key).
    let mcp_json: figment::value::Value = Figment::from(Json::file(base_dir.join("cast-mcp.json")))
        .extract()
        .unwrap_or_else(|_| figment::value::Value::from(figment::value::Dict::new()));

    let config: Config = figment
        .merge(Json::file(base_dir.join("cast.json")))
        .merge(Serialized::defaults(mcp_json).key("mcp"))
        .merge(Env::prefixed("CAST_").split("__"))
        .extract()
        .context("Failed to load configuration")?;

    info!(
        memory = %config.memory,
        cpus = config.cpus,
        pids_limit = config.pids_limit,
        network = %config.network,
        "config loaded"
    );

    Ok(config)
}

/// Resolve the global config path (~/.config/cast/cast.json)
/// Returns None if the home directory cannot be determined
fn global_config_path() -> Option<PathBuf> {
    home_config_dir().map(|p| p.join("cast").join("cast.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_config_path_is_under_home_config() {
        let path = global_config_path().expect("global config path should resolve");
        let home = dirs::home_dir().expect("home dir should resolve");
        assert!(
            path.starts_with(home.join(".config")),
            "expected path under $HOME/.config, got: {}",
            path.display()
        );
        assert!(path.ends_with("cast/cast.json"));
    }

    #[test]
    fn test_load_config_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        // Just verify it loads without error
        // Don't test specific values since they depend on user's environment
        let config = load_config_from(dir.path()).unwrap();

        // Basic sanity checks - these fields should always have some value
        assert!(!config.memory.is_empty());
        assert!(config.cpus > 0.0);
    }

    #[test]
    fn test_config_default_has_expected_values() {
        // Test the defaults in isolation
        let config = Config::default();

        assert!(config.agent_versions.is_empty());
        assert_eq!(config.memory, "1024m");
        assert_eq!(config.cpus, 1.0);
    }

    #[test]
    fn test_merge_cast_and_mcp_json() {
        use std::fs::File;
        use std::io::Write;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();

        // Create cast.json
        let mut cast_json = File::create(dir.path().join("cast.json")).unwrap();
        writeln!(
            cast_json,
            r#"{{ "memory": "2048m", "mcp": {{ "port": 3000 }} }}"#
        )
        .unwrap();

        // Create cast-mcp.json (flat structure, no root "mcp" key)
        let mut mcp_json = File::create(dir.path().join("cast-mcp.json")).unwrap();
        writeln!(mcp_json, r#"{{ "hostname": "0.0.0.0", "port": 4000 }}"#).unwrap();

        let config = load_config_from(dir.path()).unwrap();

        // Should have memory from cast.json
        assert_eq!(config.memory, "2048m");
        // Should have hostname from cast-mcp.json
        assert_eq!(config.mcp.hostname, "0.0.0.0");
        // Should have port from cast-mcp.json (precedence)
        assert_eq!(config.mcp.port, 4000);
    }

    #[test]
    fn test_load_config_without_mcp_json() {
        use std::fs::File;
        use std::io::Write;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();

        // Create cast.json only
        let mut cast_json = File::create(dir.path().join("cast.json")).unwrap();
        writeln!(cast_json, r#"{{ "memory": "2048m" }}"#).unwrap();

        let config = load_config_from(dir.path()).unwrap();

        assert_eq!(config.memory, "2048m");
        // Should have defaults for MCP
        assert_eq!(config.mcp.port, 8080);
    }
}
