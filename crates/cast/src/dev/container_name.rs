use crate::config::Config;

/// Resolve the Docker container name for an agent session.
///
/// Priority:
/// 1. `explicit_name` set via `--name` → returned as-is
/// 2. `token` present → `cast-{agent}-{basename}-{port}-{token}`
///    The port is included so the name shares a greppable prefix with the
///    interactive container (`cast-{agent}-{basename}-{port}`), allowing
///    `docker ps --filter name=cast-{agent}-{basename}-{port}` to match both.
/// 3. Otherwise (interactive default) → `cast-{agent}-{basename}-{port}`
///    (or `cfg.container_name`-based when config override is set)
///
/// All inputs are injected by the caller so this function remains pure and
/// fully unit-testable.
pub fn resolve_container_name(
    cfg: &Config,
    agent_name: &str,
    cwd_basename: &str,
    port: u16,
    explicit_name: Option<&str>,
    token: Option<&str>,
) -> String {
    // --name override always wins.
    if let Some(name) = explicit_name {
        return name.to_string();
    }

    // Token path: unique ephemeral name with injected token.
    // Format: cast-{agent}-{basename}-{port}-{token}
    // This is a strict suffix-extension of the interactive default so that
    // one docker ps filter can match all containers for a given project/agent.
    if let Some(tok) = token {
        return format!("cast-{}-{}-{}-{}", agent_name, cwd_basename, port, tok);
    }

    // Interactive default: deterministic name (stable, re-attachable).
    match &cfg.container_name {
        Some(name) => format!("{}-{}", name, port),
        None => format!("cast-{}-{}-{}", agent_name, cwd_basename, port),
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
            resolve_container_name(&cfg, "opencode", "irrelevant", 8080, None, None),
            "my-project-8080",
        );
    }

    #[test]
    fn test_default_uses_agent_basename_and_port() {
        let cfg = Config::default();
        assert_eq!(
            resolve_container_name(&cfg, "opencode", "my-app", 8080, None, None),
            "cast-opencode-my-app-8080",
        );
    }

    #[test]
    fn test_explicit_name_overrides_all() {
        let cfg = Config {
            container_name: Some("ignored".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_container_name(
                &cfg,
                "opencode",
                "my-app",
                8080,
                Some("my-custom-name"),
                Some("abc123"),
            ),
            "my-custom-name",
        );
    }

    #[test]
    fn test_headless_with_token() {
        let cfg = Config::default();
        assert_eq!(
            resolve_container_name(&cfg, "opencode", "my-app", 8080, None, Some("abc123")),
            "cast-opencode-my-app-8080-abc123",
        );
    }

    #[test]
    fn test_headless_token_overrides_config_name() {
        // Headless always uses the ephemeral auto-generated format regardless
        // of cfg.container_name. Use --name for an explicit override.
        let cfg = Config {
            container_name: Some("custom".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resolve_container_name(&cfg, "opencode", "my-app", 8080, None, Some("tok42")),
            "cast-opencode-my-app-8080-tok42",
        );
    }

    #[test]
    fn test_token_name_starts_with_interactive_stable_name() {
        // Any token-suffixed name is a strict suffix-extension of the
        // interactive stable name, so one `docker ps --filter name=<prefix>`
        // matches both.
        let cfg = Config::default();
        let stable = resolve_container_name(&cfg, "opencode", "my-app", 8080, None, None);
        let with_token =
            resolve_container_name(&cfg, "opencode", "my-app", 8080, None, Some("xyz"));
        assert!(
            with_token.starts_with(&stable),
            "token name '{}' should start with stable name '{}'",
            with_token,
            stable,
        );
    }
}
