//! End-to-end orchestrator tests (AC 3 + AC 7). A scripted `sh` fake stands in
//! for the harness binary; the OpenCode adapter still drives `extract_result`
//! against the fake's opencode-shaped JSON. All fs via tempdir (Nix-safe).
#![cfg(unix)]

use cast_agent::harness::OpenCode;
use cast_agent::run::{limits_from_timeout, orchestrate};
use serde_json::Value;
use std::time::Duration;

fn sh(script: &str) -> (String, Vec<String>) {
    ("sh".to_string(), vec!["-c".to_string(), script.to_string()])
}

fn read_json(path: &std::path::Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[tokio::test]
async fn completed_run_extracts_final_message_and_writes_artifacts() {
    let dir = tempfile::tempdir().unwrap();
    // Two text parts then a terminal step_finish; extract_result takes the
    // last `text` event (opencode contract).
    let (exe, args) = sh(concat!(
        r#"printf '{"type":"text","part":{"text":"thinking"}}\n';"#,
        r#"printf '{"type":"text","part":{"text":"the answer"}}\n';"#,
        r#"printf '{"type":"step_finish"}\n'"#,
    ));
    let report = orchestrate(
        &OpenCode,
        &exe,
        &args,
        "do the thing",
        dir.path(),
        limits_from_timeout(Duration::from_secs(5)),
    )
    .await
    .unwrap();

    assert_eq!(report.exit_code, 0);
    assert_eq!(report.final_message.as_deref(), Some("the answer"));

    // AC 7: the full artifact bundle exists.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("prompt.txt")).unwrap(),
        "do the thing"
    );
    assert!(dir.path().join("cast-agent.pid").exists());
    assert!(dir.path().join("stderr.log").exists());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("stream.jsonl"))
            .unwrap()
            .lines()
            .count(),
        3
    );

    let v = read_json(&dir.path().join("result.json"));
    assert_eq!(v["outcome"], "completed");
    assert_eq!(v["final_message"], "the answer");
    assert_eq!(v["harness"], "opencode");
    assert_eq!(v["event_count"], 3);
    assert_eq!(v["exit"]["code"], 0);
    assert!(v["log_path"].as_str().unwrap().ends_with("stream.jsonl"));
    assert!(v["prompt_path"].as_str().unwrap().ends_with("prompt.txt"));
}

#[tokio::test]
async fn failed_run_reports_nonzero_and_no_final_message() {
    let dir = tempfile::tempdir().unwrap();
    let (exe, args) = sh(r#"printf '{"type":"text","part":{"text":"partial"}}\n'; exit 7"#);
    let report = orchestrate(
        &OpenCode,
        &exe,
        &args,
        "p",
        dir.path(),
        limits_from_timeout(Duration::from_secs(5)),
    )
    .await
    .unwrap();

    assert_eq!(report.exit_code, 1);
    assert!(report.final_message.is_none());

    let v = read_json(&dir.path().join("result.json"));
    assert_eq!(v["outcome"], "failed");
    assert_eq!(v["final_message"], Value::Null);
    assert_eq!(v["exit"]["code"], 7);
}

#[tokio::test]
async fn timed_out_run_reports_timeout_and_partial_trace() {
    let dir = tempfile::tempdir().unwrap();
    let (exe, args) = sh(r#"printf '{"type":"text","part":{"text":"hi"}}\n'; sleep 100"#);
    let report = orchestrate(
        &OpenCode,
        &exe,
        &args,
        "p",
        dir.path(),
        limits_from_timeout(Duration::from_millis(400)),
    )
    .await
    .unwrap();

    assert_eq!(report.exit_code, 3);
    let v = read_json(&dir.path().join("result.json"));
    assert_eq!(v["outcome"], "timed_out");
    // Partial trace survived.
    assert_eq!(v["event_count"], 1);
}
