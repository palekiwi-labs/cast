use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_has_defaults() {
    let config = cast::config::Config::default();

    // Should have sensible defaults
    assert_eq!(config.memory, "1024m");
    assert_eq!(config.cpus, 1.0);
    assert_eq!(config.pids_limit, 512);
    assert!(config.use_global_flake);
    assert!(config.use_project_flake);
}

#[test]
fn test_config_load_with_partial_json() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("cast.json");

    let json = json!({
        "memory": "4g",
        "cpus": 2.5,
    });

    fs::write(&config_path, json.to_string()).unwrap();

    // Load config from the temp directory
    let config = cast::config::load_config_from(temp_dir.path()).unwrap();

    // Should merge with defaults
    assert_eq!(config.memory, "4g");
    assert_eq!(config.cpus, 2.5);
}

#[test]
fn test_config_env_vars_override() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("cast.json");

    let json = json!({
        "memory": "2g",
        "cpus": 1.5,
        "nix_volume_name": "from-file"
    });

    fs::write(&config_path, json.to_string()).unwrap();

    let data_dir = TempDir::new().unwrap();
    // Run cast config show with env vars set via subprocess
    // Note: Use single underscore for field names (CAST_NIX_VOLUME_NAME)
    let output = Command::cargo_bin("cast")
        .unwrap()
        .current_dir(temp_dir.path())
        .env("CAST_LOG_DIR", std::env::temp_dir().join("cast-test-logs"))
        .env("CAST_DATA_DIR", data_dir.path())
        .env("CAST_MEMORY", "8g")
        .env("CAST_CPUS", "4.0")
        .env("CAST_NIX_VOLUME_NAME", "from-env")
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
fn test_config_local_file_overrides_project_but_not_env() {
    let workspace = TempDir::new().unwrap();
    let data_dir = TempDir::new().unwrap();

    fs::write(
        workspace.path().join("cast.json"),
        json!({ "memory": "2g", "cpus": 1.5 }).to_string(),
    )
    .unwrap();
    fs::write(
        workspace.path().join("cast.local.json"),
        json!({ "memory": "4g", "cpus": 2.5 }).to_string(),
    )
    .unwrap();

    let output = Command::cargo_bin("cast")
        .unwrap()
        .current_dir(workspace.path())
        .env("CAST_LOG_DIR", data_dir.path().join("logs"))
        .env("CAST_DATA_DIR", data_dir.path())
        .env("CAST_MEMORY", "8g")
        .args(["config", "show"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let config: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(config["memory"], "8g");
    assert_eq!(config["cpus"], 2.5);
}

#[test]
fn test_config_serialize_to_json() {
    let config = cast::config::Config::default();

    let json = serde_json::to_string_pretty(&config).unwrap();

    // Should be valid JSON
    assert!(json.contains("memory"));
}

fn cast_with_data_dir(data_dir: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("cast").unwrap();
    cmd.env("CAST_LOG_DIR", std::env::temp_dir().join("cast-test-logs"))
        .env("CAST_DATA_DIR", data_dir);
    cmd
}

#[test]
fn test_config_init_creates_global_config_and_flake() {
    let workspace = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let data_dir = TempDir::new().unwrap();

    cast_with_data_dir(data_dir.path())
        .current_dir(workspace.path())
        .env("HOME", home.path())
        .args(["config", "init"])
        .assert()
        .success();

    let config_path = home.path().join(".config/cast/cast.json");
    let config: serde_json::Value =
        serde_json::from_slice(&fs::read(config_path).unwrap()).unwrap();
    assert_eq!(config["global_shell"], "~/.config/cast/nix#default");
    assert_eq!(
        config["nix_extra_substituters"],
        json!(["https://cache.numtide.com"])
    );
    assert_eq!(
        config["nix_extra_trusted_public_keys"],
        json!(["niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g="])
    );

    let flake = fs::read_to_string(home.path().join(".config/cast/nix/flake.nix")).unwrap();
    assert!(flake.contains("llm-agents"));
}

#[test]
fn test_config_init_preserves_existing_config_and_creates_missing_flake() {
    let workspace = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let data_dir = TempDir::new().unwrap();
    let cast_dir = home.path().join(".config/cast");
    fs::create_dir_all(&cast_dir).unwrap();
    fs::write(cast_dir.join("cast.json"), "{\"memory\":\"8g\"}").unwrap();

    let output = cast_with_data_dir(data_dir.path())
        .current_dir(workspace.path())
        .env("HOME", home.path())
        .args(["config", "init"])
        .assert()
        .success()
        .get_output()
        .clone();

    assert_eq!(
        fs::read_to_string(cast_dir.join("cast.json")).unwrap(),
        "{\"memory\":\"8g\"}"
    );
    assert!(cast_dir.join("nix/flake.nix").exists());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Skipped existing global cast config"));
    assert!(stderr.contains("Created global nix flake"));
}

#[test]
fn test_config_diff_no_approval() {
    let workspace = TempDir::new().unwrap();
    let data_dir = TempDir::new().unwrap();

    cast_with_data_dir(data_dir.path())
        .current_dir(workspace.path())
        .args(["config", "diff"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cast config allow"));
}

#[test]
fn test_config_diff_no_changes_when_approved() {
    let workspace = TempDir::new().unwrap();
    let data_dir = TempDir::new().unwrap();

    // Approve first
    cast_with_data_dir(data_dir.path())
        .current_dir(workspace.path())
        .args(["config", "allow"])
        .assert()
        .success();

    // Diff should report no changes
    cast_with_data_dir(data_dir.path())
        .current_dir(workspace.path())
        .args(["config", "diff"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No changes"));
}

#[test]
fn test_config_diff_shows_diff_when_config_changed() {
    let workspace = TempDir::new().unwrap();
    let data_dir = TempDir::new().unwrap();

    // Write config A and approve it
    let config_a = serde_json::json!({ "memory": "1024m" });
    fs::write(workspace.path().join("cast.json"), config_a.to_string()).unwrap();

    cast_with_data_dir(data_dir.path())
        .current_dir(workspace.path())
        .args(["config", "allow"])
        .assert()
        .success();

    // Change to config B
    let config_b = serde_json::json!({ "memory": "4096m" });
    fs::write(workspace.path().join("cast.json"), config_b.to_string()).unwrap();

    // Diff should show the changed memory value
    let output = cast_with_data_dir(data_dir.path())
        .current_dir(workspace.path())
        .args(["config", "diff"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("1024m") || stdout.contains("4096m"),
        "Diff should mention changed memory values, got: {}",
        stdout
    );
}

#[test]
fn test_config_show_hints_allow_when_unapproved() {
    let workspace = TempDir::new().unwrap();
    let data_dir = TempDir::new().unwrap();

    let output = cast_with_data_dir(data_dir.path())
        .current_dir(workspace.path())
        .args(["config", "show"])
        .assert()
        .success()
        .get_output()
        .clone();

    // stdout must still be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<serde_json::Value>(stdout.trim())
        .expect("stdout must still be valid JSON");

    // stderr must suggest `cast config allow` (not `cast config diff`) — no prior approval
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cast config allow"),
        "stderr should mention cast config allow for never-approved workspace, got: {}",
        stderr
    );
    assert!(
        !stderr.contains("cast config diff"),
        "stderr should NOT mention cast config diff when there is no prior approval, got: {}",
        stderr
    );
}

#[test]
fn test_config_show_hints_diff_when_changed() {
    let workspace = TempDir::new().unwrap();
    let data_dir = TempDir::new().unwrap();

    // Write config A and approve it
    let config_a = serde_json::json!({ "memory": "1024m" });
    fs::write(workspace.path().join("cast.json"), config_a.to_string()).unwrap();

    cast_with_data_dir(data_dir.path())
        .current_dir(workspace.path())
        .args(["config", "allow"])
        .assert()
        .success();

    // Change to config B
    let config_b = serde_json::json!({ "memory": "4096m" });
    fs::write(workspace.path().join("cast.json"), config_b.to_string()).unwrap();

    let output = cast_with_data_dir(data_dir.path())
        .current_dir(workspace.path())
        .args(["config", "show"])
        .assert()
        .success()
        .get_output()
        .clone();

    // stdout must still be valid JSON
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<serde_json::Value>(stdout.trim())
        .expect("stdout must still be valid JSON");

    // stderr must suggest `cast config diff` — prior approval exists but config changed
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cast config diff"),
        "stderr should mention cast config diff when config has changed, got: {}",
        stderr
    );
}

#[test]
fn test_adding_local_config_requires_reapproval() {
    let workspace = TempDir::new().unwrap();
    let data_dir = TempDir::new().unwrap();

    fs::write(
        workspace.path().join("cast.json"),
        json!({ "memory": "1024m" }).to_string(),
    )
    .unwrap();

    cast_with_data_dir(data_dir.path())
        .current_dir(workspace.path())
        .args(["config", "allow"])
        .assert()
        .success();

    fs::write(
        workspace.path().join("cast.local.json"),
        json!({ "memory": "4096m" }).to_string(),
    )
    .unwrap();

    let output = cast_with_data_dir(data_dir.path())
        .current_dir(workspace.path())
        .args(["config", "show"])
        .assert()
        .success()
        .get_output()
        .clone();

    let config: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(config["memory"], "4096m");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cast config diff"),
        "adding cast.local.json should require reapproval, got: {}",
        stderr
    );
}
