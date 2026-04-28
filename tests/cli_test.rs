use assert_cmd::Command;
use predicates::prelude::*;

fn cast() -> Command {
    Command::cargo_bin("cast").unwrap()
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
fn test_cast_run_help() {
    cast()
        .args(["run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cast run"))
        .stdout(predicate::str::contains("opencode"));
}

#[test]
fn test_cast_build_help() {
    cast()
        .args(["build", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cast build"))
        .stdout(predicate::str::contains("opencode"));
}

#[test]
fn test_cast_build_opencode_help() {
    cast()
        .args(["build", "opencode", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--base"))
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("--no-cache"));
}

#[test]
fn test_cast_shell_help() {
    cast()
        .args(["shell", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cast shell"))
        .stdout(predicate::str::contains("opencode"));
}

#[test]
fn test_cast_shell_opencode_help() {
    cast()
        .args(["shell", "opencode", "--help"])
        .assert()
        .success();
}
