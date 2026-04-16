use assert_cmd::Command;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_has_defaults() {
    let config = ocx::config::Config::default();

    // Should have sensible defaults
    assert_eq!(config.opencode_version, "latest");
    assert_eq!(config.memory, "1024m");
    assert_eq!(config.cpus, 1.0);
    assert_eq!(config.pids_limit, 100);
}

#[test]
fn test_config_load_with_partial_json() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("ocx.json");

    let json = json!({
        "memory": "4g",
        "cpus": 2.5,
    });

    fs::write(&config_path, json.to_string()).unwrap();

    // Load config from the temp directory
    std::env::set_current_dir(temp_dir.path()).unwrap();
    let config = ocx::config::load_config().unwrap();

    // Should merge with defaults
    assert_eq!(config.memory, "4g");
    assert_eq!(config.cpus, 2.5);
    assert_eq!(config.opencode_version, "latest"); // from defaults
}

#[test]
fn test_config_env_vars_override() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("ocx.json");

    let json = json!({
        "memory": "2g",
        "cpus": 1.5,
        "nix_volume_name": "from-file"
    });

    fs::write(&config_path, json.to_string()).unwrap();

    // Run ocx config show with env vars set via subprocess
    // Note: Use single underscore for field names (OCX_NIX_VOLUME_NAME)
    let output = Command::cargo_bin("ocx")
        .unwrap()
        .current_dir(temp_dir.path())
        .env("OCX_MEMORY", "8g")
        .env("OCX_CPUS", "4.0")
        .env("OCX_NIX_VOLUME_NAME", "from-env")
        .args(["config", "show"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let config: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Env vars should override file config
    assert_eq!(config["memory"], "8g");
    assert_eq!(config["cpus"], 4.0);
    // Test that fields with underscores work correctly
    assert_eq!(config["nix_volume_name"], "from-env");
}

#[test]
fn test_config_serialize_to_json() {
    let config = ocx::config::Config::default();

    let json = serde_json::to_string_pretty(&config).unwrap();

    // Should be valid JSON
    assert!(json.contains("opencode_version"));
    assert!(json.contains("memory"));
}
