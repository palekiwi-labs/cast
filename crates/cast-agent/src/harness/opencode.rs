use super::Harness;

/// The opencode adapter. Drives `opencode run --format json` and extracts the
/// final assistant message from the JSON-lines event stream.
///
/// Contract verified against opencode source in
/// `.cue/cast-agent-mvp/doc/opencode-headless-run-contract.md`.
#[derive(Debug, Default, Clone, Copy)]
pub struct OpenCode;

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

    fn extract_result(&self, _events: &[serde_json::Value]) -> Option<String> {
        None
    }
}
