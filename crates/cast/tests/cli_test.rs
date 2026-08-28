use assert_cmd::Command;
use predicates::prelude::*;

fn cast() -> Command {
    let mut cmd = Command::cargo_bin("cast").unwrap();
    cmd.env("CAST_LOG_DIR", std::env::temp_dir().join("cast-test-logs"))
        .env("CAST_DATA_DIR", std::env::temp_dir().join("cast-test-data"));
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
fn test_cast_port_different_for_different_agents() {
    let output_o = cast().args(["port", "opencode"]).assert().success();
    let stdout_o = String::from_utf8_lossy(&output_o.get_output().stdout);
    let port_o: u16 = stdout_o.trim().parse().unwrap();

    let output_p = cast().args(["port", "pi"]).assert().success();
    let stdout_p = String::from_utf8_lossy(&output_p.get_output().stdout);
    let port_p: u16 = stdout_p.trim().parse().unwrap();

    let output_c = cast().args(["port", "claudecode"]).assert().success();
    let stdout_c = String::from_utf8_lossy(&output_c.get_output().stdout);
    let port_c: u16 = stdout_c.trim().parse().unwrap();

    assert_ne!(
        port_o, port_p,
        "opencode and pi should have different ports"
    );
    assert_ne!(
        port_o, port_c,
        "opencode and claudecode should have different ports"
    );
    assert_ne!(
        port_p, port_c,
        "pi and claudecode should have different ports"
    );
}

#[test]
fn test_cast_port_ignores_extra_args() {
    let output1 = cast().args(["port", "pi"]).assert().success();
    let stdout1 = String::from_utf8_lossy(&output1.get_output().stdout);
    let port1: u16 = stdout1.trim().parse().unwrap();

    let output2 = cast()
        .args(["port", "pi", "--some-flag", "value"])
        .assert()
        .success();
    let stdout2 = String::from_utf8_lossy(&output2.get_output().stdout);
    let port2: u16 = stdout2.trim().parse().unwrap();

    assert_eq!(
        port1, port2,
        "Extra args should not affect port computation"
    );
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
fn test_cast_shell_help() {
    cast()
        .args(["shell", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: cast shell"))
        .stdout(predicate::str::contains("opencode"))
        .stdout(predicate::str::contains("--raw"));
}

#[test]
fn test_cast_shell_raw_opencode_help() {
    cast()
        .args(["shell", "--raw", "opencode", "--help"])
        .assert()
        .success();
}

#[test]
fn test_cast_shell_opencode_raw_fails() {
    cast()
        .args(["shell", "opencode", "--raw"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "unexpected argument '--raw' found",
        ));
}

#[test]
fn test_cast_shell_opencode_help() {
    cast()
        .args(["shell", "opencode", "--help"])
        .assert()
        .success();
}

#[test]
fn test_cast_shell_claudecode_help() {
    cast()
        .args(["shell", "claudecode", "--help"])
        .assert()
        .success();
}

/// `cast port opencode --headless` — extra flag is forwarded to port
/// computation as extra_args; port output is unaffected.
#[test]
fn test_cast_port_unaffected_by_headless_extra_arg() {
    let output1 = cast().args(["port", "opencode"]).assert().success();
    let stdout1 = String::from_utf8_lossy(&output1.get_output().stdout);
    let port1: u16 = stdout1.trim().parse().unwrap();

    let output2 = cast()
        .args(["port", "opencode", "--headless"])
        .assert()
        .success();
    let stdout2 = String::from_utf8_lossy(&output2.get_output().stdout);
    let port2: u16 = stdout2.trim().parse().unwrap();

    assert_eq!(
        port1, port2,
        "port must be identical with or without --headless in extra_args"
    );
}
