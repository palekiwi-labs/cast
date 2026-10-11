use assert_cmd::Command;
use predicates::prelude::*;

fn cast() -> Command {
    let mut cmd = Command::cargo_bin("cast").unwrap();
    cmd.env("CAST_LOG_DIR", std::env::temp_dir().join("cast-test-logs"))
        .env("CAST_DATA_DIR", std::env::temp_dir().join("cast-test-data"))
        .env("CAST_NIX_VERSION", "2.34.6");
    cmd
}

#[test]
fn test_cast_help() {
    cast()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("cast"));
}

#[test]
fn test_cast_config_help() {
    cast()
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cast config"))
        .stdout(predicate::str::contains("show"));
}

#[test]
fn test_cast_config_runs() {
    cast()
        .arg("config")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_cast_config_show() {
    cast()
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_cast_config_show_outputs_valid_json() {
    let output = cast().args(["config", "show"]).assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    serde_json::from_str::<serde_json::Value>(&stdout).expect("Output should be valid JSON");
}

#[test]
fn test_cast_port_is_not_available() {
    cast()
        .args(["port", "--help"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand 'port'"));
}

#[test]
fn test_cast_run_is_not_available() {
    cast()
        .args(["run", "--help"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand 'run'"));
}

#[test]
fn test_cast_up_help() {
    cast()
        .args(["up", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cast up"));
}

#[test]
fn test_cast_up_accepts_service_name() {
    cast()
        .args(["up", "--name", "isolated", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--name <NAME>"));
}

#[test]
fn test_cast_down_accepts_service_name() {
    cast()
        .args(["down", "--name", "isolated", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cast down"))
        .stdout(predicate::str::contains("--name <NAME>"));
}

#[test]
fn test_cast_status_accepts_service_name() {
    cast()
        .args(["status", "--name", "isolated", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cast status"))
        .stdout(predicate::str::contains("--name <NAME>"));
}

#[test]
fn test_cast_exec_accepts_arbitrary_command() {
    cast()
        .args(["exec", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<CMD>..."))
        .stdout(predicate::str::contains("--name <NAME>"))
        .stdout(predicate::str::contains("--headless"))
        .stdout(predicate::str::contains("--raw"))
        .stdout(predicate::str::contains("opencode").not());
}

#[test]
fn test_cast_build_help() {
    // `cast build` takes no agent subcommand: there is one shared dev image.
    cast()
        .args(["build", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cast build"))
        .stdout(predicate::str::contains("--nix-daemon"))
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("--no-cache"));
}

#[test]
fn test_cast_nix_daemon_up_help() {
    cast()
        .args(["nix-daemon", "up", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cast nix-daemon up"));
}

#[test]
fn test_cast_nix_daemon_down_help() {
    cast()
        .args(["nix-daemon", "down", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cast nix-daemon down"));
}

#[test]
fn test_cast_shell_help() {
    cast()
        .args(["shell", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cast shell"))
        .stdout(predicate::str::contains("--name <NAME>"))
        .stdout(predicate::str::contains("opencode").not())
        .stdout(predicate::str::contains("--raw"));
}
