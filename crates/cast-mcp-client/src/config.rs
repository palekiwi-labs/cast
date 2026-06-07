use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ClientConfig {
    #[serde(default)]
    pub mcp: HashMap<String, RemoteServerConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RemoteServerConfig {
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

pub fn parse_from_str(s: &str) -> ClientConfig {
    let mut config: ClientConfig = serde_json::from_str(s).unwrap_or_else(|e| {
        eprintln!("Warning: Failed to parse MCP config: {}", e);
        ClientConfig::default()
    });

    for server in config.mcp.values_mut() {
        for value in server.headers.values_mut() {
            *value = apply_env_substitution(value);
        }
    }

    config
}

pub(crate) fn merge(mut global: ClientConfig, project: ClientConfig) -> ClientConfig {
    for (name, server) in project.mcp {
        global.mcp.insert(name, server);
    }
    global
}

pub fn load_from_files(
    global: Option<&std::path::Path>,
    project: Option<&std::path::Path>,
) -> ClientConfig {
    let global_config = if let Some(path) = global {
        load_single_file(path)
    } else {
        ClientConfig::default()
    };

    let project_config = if let Some(path) = project {
        load_single_file(path)
    } else {
        ClientConfig::default()
    };

    merge(global_config, project_config)
}

pub fn load() -> ClientConfig {
    let global = global_config_path();
    let project = std::path::Path::new("cast-mcp-client.json");
    load_from_files(global.as_deref(), Some(project))
}

fn global_config_path() -> Option<std::path::PathBuf> {
    if let Some(xdg) = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        return Some(std::path::PathBuf::from(xdg.trim()).join("cast/cast-mcp-client.json"));
    }
    if let Some(home) = std::env::var("HOME").ok().filter(|s| !s.trim().is_empty()) {
        return Some(
            std::path::PathBuf::from(home.trim()).join(".config/cast/cast-mcp-client.json"),
        );
    }
    None
}

fn load_single_file(path: &std::path::Path) -> ClientConfig {
    match std::fs::read_to_string(path) {
        Ok(s) => parse_from_str(&s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ClientConfig::default(),
        Err(e) => {
            eprintln!("Warning: Failed to read MCP config at {:?}: {}", path, e);
            ClientConfig::default()
        }
    }
}

fn apply_env_substitution(s: &str) -> String {
    let mut result = String::new();
    let mut current = s;

    while let Some(start_index) = current.find("{env:") {
        result.push_str(&current[..start_index]);
        let remaining = &current[start_index + 5..];
        if let Some(end_index) = remaining.find('}') {
            let var_name = &remaining[..end_index];
            let val = std::env::var(var_name).unwrap_or_default();
            result.push_str(&val);
            current = &remaining[end_index + 1..];
        } else {
            result.push_str("{env:");
            current = remaining;
        }
    }
    result.push_str(current);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let json = r#"{"mcp":{"myserver":{"url":"http://example.com/mcp"}}}"#;
        let config = parse_from_str(json);
        assert_eq!(config.mcp.len(), 1);
        assert_eq!(config.mcp["myserver"].url, "http://example.com/mcp");
    }

    #[test]
    fn test_default_enabled_and_headers() {
        let json = r#"{"mcp":{"myserver":{"url":"http://example.com/mcp"}}}"#;
        let config = parse_from_str(json);
        let server = &config.mcp["myserver"];
        assert!(server.enabled);
        assert!(server.headers.is_empty());
    }

    #[test]
    fn test_env_var_substitution() {
        unsafe {
            std::env::set_var("CAST_TEST_TOKEN", "secret");
        }
        let json = r#"{"mcp":{"myserver":{"url":"http://example.com/mcp", "headers": {"Authorization": "Bearer {env:CAST_TEST_TOKEN}"}}}}"#;
        let config = parse_from_str(json);
        unsafe {
            std::env::remove_var("CAST_TEST_TOKEN");
        }
        assert_eq!(
            config.mcp["myserver"].headers["Authorization"],
            "Bearer secret"
        );
    }

    #[test]
    fn test_unset_env_var_becomes_empty() {
        let json = r#"{"mcp":{"myserver":{"url":"http://example.com/mcp", "headers": {"Authorization": "Bearer {env:NON_EXISTENT_VAR}"}}}}"#;
        let config = parse_from_str(json);
        assert_eq!(config.mcp["myserver"].headers["Authorization"], "Bearer ");
    }

    #[test]
    fn test_project_overrides_global() {
        let global = parse_from_str(r#"{"mcp":{"server":{"url":"http://global"}}}"#);
        let project = parse_from_str(r#"{"mcp":{"server":{"url":"http://project"}}}"#);
        let merged = merge(global, project);
        assert_eq!(merged.mcp["server"].url, "http://project");
    }

    #[test]
    fn test_merge_adds_project_only_servers() {
        let global = parse_from_str(r#"{"mcp":{"serverA":{"url":"http://globalA"}}}"#);
        let project = parse_from_str(r#"{"mcp":{"serverB":{"url":"http://projectB"}}}"#);
        let merged = merge(global, project);
        assert_eq!(merged.mcp.len(), 2);
        assert_eq!(merged.mcp["serverA"].url, "http://globalA");
        assert_eq!(merged.mcp["serverB"].url, "http://projectB");
    }

    #[test]
    fn test_load_from_files_with_project_override() {
        use std::fs;
        let temp_dir = std::env::temp_dir();
        let global_path = temp_dir.join("global.json");
        let project_path = temp_dir.join("project.json");

        fs::write(
            &global_path,
            r#"{"mcp":{"server":{"url":"http://global"}}}"#,
        )
        .unwrap();
        fs::write(
            &project_path,
            r#"{"mcp":{"server":{"url":"http://project"}}}"#,
        )
        .unwrap();

        let config = load_from_files(Some(&global_path), Some(&project_path));

        fs::remove_file(global_path).unwrap();
        fs::remove_file(project_path).unwrap();

        assert_eq!(config.mcp["server"].url, "http://project");
    }

    #[test]
    fn test_missing_files_skipped() {
        let config = load_from_files(
            Some(std::path::Path::new("/non/existent/global.json")),
            Some(std::path::Path::new("/non/existent/project.json")),
        );
        assert!(config.mcp.is_empty());
    }

    #[test]
    fn test_malformed_config_falls_back() {
        use std::fs;
        let temp_dir = std::env::temp_dir();
        let malformed_path = temp_dir.join("malformed.json");

        fs::write(&malformed_path, "not valid json").unwrap();

        let config = load_from_files(Some(&malformed_path), None);

        fs::remove_file(malformed_path).unwrap();

        assert!(config.mcp.is_empty());
    }
}
