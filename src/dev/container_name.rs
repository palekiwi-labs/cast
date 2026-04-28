use crate::config::Config;

/// Resolve the Docker container name for an agent session.
///
/// - If `cfg.container_name` is set: `"{name}-{port}"`
/// - Otherwise: `"ocx-{agent}-{basename}-{port}"` where `agent` is the
///   agent identifier and `basename` is the name of the current working directory.
///
/// Both `agent_name` and `cwd_basename` are injected by the caller so that
/// this function remains pure and fully unit-testable.
pub fn resolve_container_name(
    cfg: &Config,
    agent_name: &str,
    cwd_basename: &str,
    port: u16,
) -> String {
    match &cfg.container_name {
        Some(name) => format!("{}-{}", name, port),
        None => format!("ocx-{}-{}-{}", agent_name, cwd_basename, port),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_container_name_with_port() {
        let cfg = Config {
            container_name: Some("my-project".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_container_name(&cfg, "opencode", "irrelevant", 8080),
            "my-project-8080",
        );
    }

    #[test]
    fn test_default_uses_agent_basename_and_port() {
        let cfg = Config::default();
        assert_eq!(
            resolve_container_name(&cfg, "opencode", "my-app", 8080),
            "ocx-opencode-my-app-8080",
        );
    }
}
