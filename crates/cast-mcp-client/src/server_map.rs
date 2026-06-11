use crate::config::{ClientConfig, RemoteServerConfig};
use std::collections::HashMap;

/// Build the active server map from config, optionally injecting a cast URL
/// from a CLI flag or environment variable.
///
/// - All config entries with `enabled: true` are included (except `"cast"`,
///   which is handled specially).
/// - If `cast_url` is `Some(url)` (sourced from CLI flag or env var), a bare
///   `"cast"` entry is injected with that URL and no headers, overriding any
///   `"cast"` entry in the config.
/// - If `cast_url` is `None`, the config's `"cast"` entry is included as-is
///   (with its headers).
pub fn build_server_map(
    cast_url: Option<String>,
    config: &ClientConfig,
) -> HashMap<String, RemoteServerConfig> {
    let mut map = HashMap::new();

    // Include all enabled non-cast entries from config
    for (name, server) in &config.mcp {
        if name == "cast" {
            continue; // handled separately below
        }
        if server.enabled {
            map.insert(name.clone(), server.clone());
        }
    }

    // Resolve the "cast" entry
    match cast_url {
        Some(url) => {
            // URL came from flag/env — inject bare entry, no headers
            map.insert(
                "cast".to_string(),
                RemoteServerConfig {
                    url,
                    headers: HashMap::new(),
                    enabled: true,
                },
            );
        }
        None => {
            // URL from config (if present and enabled) — use full entry
            // including headers
            if let Some(server) = config.mcp.get("cast")
                && server.enabled
            {
                map.insert("cast".to_string(), server.clone());
            }
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parse_from_str;

    #[test]
    fn test_build_server_map_includes_enabled_servers() {
        let config = parse_from_str(
            r#"{"mcp":{"sentry":{"url":"http://sentry.com/mcp"},"ctx7":{"url":"http://ctx7.com/mcp"}}}"#,
            &HashMap::new(),
        );
        let map = build_server_map(None, &config);
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("sentry"));
        assert!(map.contains_key("ctx7"));
    }

    #[test]
    fn test_build_server_map_excludes_disabled_servers() {
        let config = parse_from_str(
            r#"{"mcp":{"sentry":{"url":"http://sentry.com/mcp"},"ctx7":{"url":"http://ctx7.com/mcp","enabled":false}}}"#,
            &HashMap::new(),
        );
        let map = build_server_map(None, &config);
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("sentry"));
        assert!(!map.contains_key("ctx7"));
    }

    #[test]
    fn test_build_server_map_injects_bare_cast_entry_when_url_from_flag_or_env() {
        // Config has a "cast" entry with a header — flag/env URL must
        // override it (no headers).
        let config = parse_from_str(
            r#"{"mcp":{"cast":{"url":"http://config.com/mcp","headers":{"X-Token":"secret"}}}}"#,
            &HashMap::new(),
        );
        let map = build_server_map(Some("http://flag.com/mcp".to_string()), &config);
        let cast = map.get("cast").expect("cast entry should be present");
        assert_eq!(cast.url, "http://flag.com/mcp");
        assert!(
            cast.headers.is_empty(),
            "headers must be stripped when URL comes from flag/env"
        );
    }

    #[test]
    fn test_build_server_map_preserves_full_cast_entry_when_url_from_config() {
        // No explicit URL provided — config entry (including headers) should
        // be used as-is.
        let config = parse_from_str(
            r#"{"mcp":{"cast":{"url":"http://config.com/mcp","headers":{"X-Token":"secret"}}}}"#,
            &HashMap::new(),
        );
        let map = build_server_map(None, &config);
        let cast = map.get("cast").expect("cast entry should be present");
        assert_eq!(cast.url, "http://config.com/mcp");
        assert_eq!(
            cast.headers.get("X-Token").map(String::as_str),
            Some("secret")
        );
    }

    #[test]
    fn test_build_server_map_empty_when_no_servers() {
        let config = ClientConfig::default();
        let map = build_server_map(None, &config);
        assert!(map.is_empty());
    }
}
