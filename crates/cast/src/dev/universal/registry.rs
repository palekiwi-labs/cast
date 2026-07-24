use crate::dev::agent::Agent;
use crate::dev::claudecode::ClaudeCode;
use crate::dev::opencode::OpenCode;
use crate::dev::pi::Pi;

/// All known agent types, in canonical (alphabetical by name) order.
///
/// This is the single place to add a new agent harness.
pub fn all_agents() -> &'static [&'static dyn Agent] {
    // Sorted by name: "claudecode", "opencode", "pi".
    &[&ClaudeCode, &OpenCode, &Pi]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_agents_returns_three_known_agents() {
        let agents = all_agents();
        let names: Vec<&str> = agents.iter().map(|a| a.name()).collect();
        assert_eq!(names, vec!["claudecode", "opencode", "pi"]);
    }
}
