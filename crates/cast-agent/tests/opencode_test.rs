use cast_agent::harness::{Harness, OpenCode};

fn parse_fixture(name: &str) -> Vec<serde_json::Value> {
    let raw = std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn headless_args_are_run_format_json() {
    let h = OpenCode;
    assert_eq!(h.headless_args(), vec!["run", "--format", "json"]);
}

#[test]
fn extract_result_returns_last_text_event() {
    let h = OpenCode;
    let events = parse_fixture("opencode-run.jsonl");
    assert_eq!(
        h.extract_result(&events),
        Some("Files in the directory.\n**Total: 12 entries**".to_string())
    );
}

#[test]
fn extract_result_none_when_no_text_event() {
    let h = OpenCode;
    let events = vec![
        serde_json::json!({"type": "step_start"}),
        serde_json::json!({"type": "step_finish", "reason": "stop"}),
    ];
    assert_eq!(h.extract_result(&events), None);
}

#[test]
fn extract_result_picks_last_of_multiple_text_events() {
    let h = OpenCode;
    let events = vec![
        serde_json::json!({"type": "text", "part": {"text": "first"}}),
        serde_json::json!({"type": "text", "part": {"text": "second"}}),
    ];
    assert_eq!(h.extract_result(&events), Some("second".to_string()));
}
