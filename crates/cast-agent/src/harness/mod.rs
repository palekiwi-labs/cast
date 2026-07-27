//! The `Harness` trait encapsulates only what genuinely differs between
//! agent harnesses (opencode / claudecode / pi): identity, executable, the
//! headless invocation baseline, per-flag mappers, and result extraction.
//! Everything harness-agnostic lives in the orchestrator.

pub mod opencode;

pub use opencode::OpenCode;

/// A launchable agent harness in headless (JSON-lines) mode.
pub trait Harness {
    /// Identity, e.g. "opencode".
    fn name(&self) -> &'static str;

    /// Executable, e.g. "opencode" (claudecode resolves to "claude").
    fn base_command(&self) -> &'static str;

    /// Headless invocation baseline: subcommand + JSON-lines format flags,
    /// e.g. `["run", "--format", "json"]` for opencode.
    fn headless_args(&self) -> Vec<String>;

    // Per-flag mappers. `Some(args)` = native mapping; `None` = unsupported
    // (the orchestrator emits a no-op + warn). Defaults return `None` so each
    // harness overrides only what it truly supports. Backlog for the MVP.
    fn model_args(&self, _model: &str) -> Option<Vec<String>> {
        None
    }
    fn escalate_args(&self) -> Option<Vec<String>> {
        None
    }
    fn system_prompt_args(&self, _text: &str) -> Option<Vec<String>> {
        None
    }
    fn agent_args(&self, _name: &str) -> Option<Vec<String>> {
        None
    }

    /// Pull the final assistant text out of the collected JSON-lines events.
    fn extract_result(&self, events: &[serde_json::Value]) -> Option<String>;
}
