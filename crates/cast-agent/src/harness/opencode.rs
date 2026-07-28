use super::Harness;

/// The opencode adapter. Drives `opencode run --format json` and extracts the
/// final assistant message from the JSON-lines event stream.
///
/// Contract verified against opencode source in
/// `.cue/cast-agent-mvp/doc/opencode-headless-run-contract.md`.
#[derive(Debug, Default, Clone, Copy)]
pub struct OpenCode;

/// Pull the text content out of an opencode `text` event, tolerating both the
/// nested `part.text` shape (opencode spreads the message `part` into the
/// event, matching how `tool_use` carries the full `part` object) and a
/// top-level `text` field as a fallback. Field-name assumption logged pending
/// re-verification against opencode source.
fn text_of_event(event: &serde_json::Value) -> Option<String> {
    event
        .get("part")
        .and_then(|p| p.get("text"))
        .or_else(|| event.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
}

impl Harness for OpenCode {
    fn name(&self) -> &'static str {
        "opencode"
    }

    fn base_command(&self) -> &'static str {
        "opencode"
    }

    fn headless_args(&self) -> Vec<String> {
        ["run", "--format", "json"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn agent_args(&self, name: &str) -> Option<Vec<String>> {
        // Maps 1:1 to `opencode run --agent <name>`: the child opencode loads
        // the persona prompt + permission profile + model itself. An unknown
        // name is validated by opencode and surfaces as a non-zero child exit
        // (a failed/crashed verdict), never a silent fall-back to the default.
        Some(vec!["--agent".to_string(), name.to_string()])
    }

    fn extract_result(&self, events: &[serde_json::Value]) -> Option<String> {
        // Filter to `text`-typed events and take the last one whose text is
        // actually extractable. The stream's terminal line is always a
        // `step_finish`, so the last usable `text` event holds the substantive
        // answer (verified: opencode-self-exit trace). Using `filter_map`
        // rather than `find().and_then()` means a malformed trailing `text`
        // event falls back to an earlier valid one instead of yielding None.
        events
            .iter()
            .rev()
            .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some("text"))
            .find_map(text_of_event)
    }
}
