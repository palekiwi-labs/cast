use anyhow::{Result, bail};

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::claudecode::ClaudeCode;
use crate::dev::opencode::OpenCode;
use crate::dev::pi::Pi;

/// All known agent types, in canonical (alphabetical by name) order.
///
/// This is the registry that maps agent names — the keys used in
/// `agent_versions` — back to their `&dyn Agent` trait objects. It is the
/// single place to add a new agent harness for universal-mode resolution.
pub fn all_agents() -> &'static [&'static dyn Agent] {
    // Sorted by name: "claudecode", "opencode", "pi".
    &[&ClaudeCode, &OpenCode, &Pi]
}

/// Resolve the agents included in the universal image from `config`.
///
/// Returns `(agent, version)` pairs for every known agent whose name appears
/// as a key in `config.agent_versions`. Unknown keys in the map are silently
/// ignored. The returned slices borrow directly into `config`.
pub fn resolve_included_agents(config: &Config) -> Vec<(&'static dyn Agent, &str)> {
    all_agents()
        .iter()
        .filter_map(|agent| {
            config
                .agent_versions
                .get((*agent).name())
                .map(|v| (*agent, v.as_str()))
        })
        .collect()
}

/// Validate that the requested agent is included in the universal image.
///
/// Bails with an actionable message if `agent_name` is not a key in
/// `config.agent_versions`.
pub fn validate_agent_included(config: &Config, agent_name: &str) -> Result<()> {
    if !config.agent_versions.contains_key(agent_name) {
        bail!(
            "{agent_name} is not included in your universal container — \
             add it to agent_versions in cast.json to include it"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn config_with_versions(pairs: &[(&str, &str)]) -> Config {
        let mut config = Config::default();
        config.agent_versions = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>();
        config
    }

    // ── resolve_included_agents ─────────────────────────────────────────────

    #[test]
    fn resolve_returns_agents_that_are_pinned() {
        let config = config_with_versions(&[("opencode", "0.1.0"), ("pi", "0.2.0")]);

        let included = resolve_included_agents(&config);

        let names: Vec<&str> = included.iter().map(|(a, _)| a.name()).collect();
        assert!(names.contains(&"opencode"), "opencode should be included");
        assert!(names.contains(&"pi"), "pi should be included");
        assert!(
            !names.contains(&"claudecode"),
            "claudecode should be excluded (not pinned)"
        );
    }

    #[test]
    fn resolve_returns_empty_when_nothing_pinned() {
        let config = Config::default();
        let included = resolve_included_agents(&config);
        assert!(included.is_empty(), "no pinned agents → empty set");
    }

    #[test]
    fn resolve_returns_versions_from_config() {
        let config = config_with_versions(&[("opencode", "1.2.3")]);

        let included = resolve_included_agents(&config);

        let (agent, version) = included
            .iter()
            .find(|(a, _)| a.name() == "opencode")
            .expect("opencode should be present");
        assert_eq!(*version, "1.2.3");
        assert_eq!(agent.name(), "opencode");
    }

    #[test]
    fn resolve_ignores_unknown_agent_names() {
        let mut config = Config::default();
        config
            .agent_versions
            .insert("ghost".to_string(), "9.9.9".to_string());
        config
            .agent_versions
            .insert("opencode".to_string(), "0.1.0".to_string());

        let included = resolve_included_agents(&config);

        // Only the known agent is returned; "ghost" is silently ignored.
        assert_eq!(included.len(), 1);
        assert_eq!(included[0].0.name(), "opencode");
    }

    #[test]
    fn resolve_returns_all_three_when_all_pinned() {
        let config = config_with_versions(&[
            ("claudecode", "1.0.0"),
            ("opencode", "0.1.0"),
            ("pi", "0.2.0"),
        ]);

        let included = resolve_included_agents(&config);
        assert_eq!(included.len(), 3);
    }

    // ── validate_agent_included ─────────────────────────────────────────────

    #[test]
    fn validate_accepts_pinned_agent() {
        let config = config_with_versions(&[("opencode", "0.1.0")]);
        validate_agent_included(&config, "opencode").expect("pinned agent should validate");
    }

    #[test]
    fn validate_rejects_unpinned_agent() {
        let config = config_with_versions(&[("opencode", "0.1.0")]);
        let err = validate_agent_included(&config, "pi").unwrap_err();
        assert!(
            err.to_string().contains("pi"),
            "error should name the missing agent, got: {err}"
        );
        assert!(
            err.to_string().contains("agent_versions"),
            "error should mention agent_versions, got: {err}"
        );
    }

    #[test]
    fn validate_rejects_when_nothing_pinned() {
        let config = Config::default();
        let err = validate_agent_included(&config, "opencode").unwrap_err();
        assert!(err.to_string().contains("opencode"));
    }
}
