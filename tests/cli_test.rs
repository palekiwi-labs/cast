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
        .stdout(predicate::str::contains("ocx - a secure Docker wrapper for OpenCode"));
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
