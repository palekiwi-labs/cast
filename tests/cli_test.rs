use assert_cmd::Command;
use predicates::prelude::*;

fn ocx() -> Command {
    Command::cargo_bin("ocx").unwrap()
}

#[test]
fn test_ocx_help() {
    ocx()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("ocx"));
}

#[test]
fn test_ocx_config_help() {
    ocx()
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: ocx config"))
        .stdout(predicate::str::contains("show"));
}

#[test]
fn test_ocx_config_runs() {
    ocx()
        .arg("config")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_ocx_config_show() {
    ocx()
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_ocx_run_opencode_help() {
    // With disable_help_flag = true, --help is passed to the handler.
    // In the test environment, this fails because docker is missing,
    // but it proves the CLI parsed the command and reached the handler.
    ocx()
        .args(["run", "opencode", "--help"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("docker"));
}

#[test]
fn test_ocx_run_alias_o_help() {
    ocx()
        .args(["run", "o", "--help"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("docker"));
}

#[test]
fn test_ocx_config_show_outputs_valid_json() {
    let output = ocx().args(["config", "show"]).assert().success();

    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // Should be valid JSON - that's all we care about at this level
    serde_json::from_str::<serde_json::Value>(&stdout).expect("Output should be valid JSON");
}

#[test]
fn test_ocx_build_help() {
    ocx()
        .args(["build", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: ocx build"))
        .stdout(predicate::str::contains("opencode"));
}

#[test]
fn test_ocx_build_opencode_help() {
    ocx()
        .args(["build", "opencode", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--base"))
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("--no-cache"));
}
