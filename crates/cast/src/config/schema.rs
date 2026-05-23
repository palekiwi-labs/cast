use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    // Container Identity & Version
    #[serde(default)]
    pub agent_versions: BTreeMap<String, String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,

    pub version_cache_ttl_hours: u32,

    // Resource Limits
    pub memory: String,
    pub cpus: f64,
    pub pids_limit: i32,

    // Networking
    pub network: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    pub publish_port: bool,
    pub add_host_docker_internal: bool,

    // Paths & Files
    pub use_flake: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_flake_path: Option<String>,

    // Data Volumes
    pub volumes_namespace: String,

    pub extra_data_volumes: BTreeMap<String, VolumeConfig>,

    // Nix Workflow
    pub nix_volume_name: String,
    pub nix_daemon_container_name: String,
    pub nix_extra_substituters: Vec<String>,
    pub nix_extra_trusted_public_keys: Vec<String>,

    // Security
    pub forbidden_paths: Vec<String>,

    #[serde(default)]
    pub mcp: McpConfig,
}

pub const DEFAULT_MCP_PORT: u16 = 8080;
pub const DEFAULT_MCP_HOSTNAME: &str = "127.0.0.1";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpConfig {
    #[serde(default = "default_mcp_port")]
    pub port: u16,
    #[serde(default = "default_mcp_hostname")]
    pub hostname: String,
    #[serde(default)]
    pub tools: BTreeMap<String, McpToolConfig>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_MCP_PORT,
            hostname: DEFAULT_MCP_HOSTNAME.to_string(),
            tools: BTreeMap::new(),
        }
    }
}

fn default_mcp_port() -> u16 {
    DEFAULT_MCP_PORT
}

fn default_mcp_hostname() -> String {
    DEFAULT_MCP_HOSTNAME.to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct McpToolConfig {
    pub description: String,
    pub command: String,
    pub args: Vec<ArgTemplate>,
    #[serde(default)]
    pub env: Option<McpEnvConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct McpEnvConfig {
    #[serde(default)]
    pub inherit: Vec<String>,
    #[serde(default)]
    pub set: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ArgTemplate {
    Literal(String),
    Conditional(ConditionalBlock),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionalBlock {
    pub if_present: Option<String>,
    pub if_true: Option<String>,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VolumeConfig {
    pub target: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    // VolumeConfig fields need serde defaults because they're deserialized
    // directly from JSON (nested in extra_data_volumes)
    #[serde(default = "default_volume_mode")]
    pub mode: String,

    #[serde(default = "default_volume_type", rename = "type")]
    pub volume_type: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            agent_versions: BTreeMap::new(),
            container_name: None,
            version_cache_ttl_hours: 24,
            memory: "1024m".to_string(),
            cpus: 1.0,
            pids_limit: 512,
            network: "bridge".to_string(),
            port: None,
            publish_port: true,
            add_host_docker_internal: true,
            use_flake: false,
            use_flake_path: None,
            volumes_namespace: "cast".to_string(),
            extra_data_volumes: BTreeMap::new(),
            nix_volume_name: "cast-nix".to_string(),
            nix_daemon_container_name: "cast-nix-daemon".to_string(),
            nix_extra_substituters: Vec::new(),
            nix_extra_trusted_public_keys: Vec::new(),
            forbidden_paths: Vec::new(),
            mcp: McpConfig::default(),
        }
    }
}

fn default_volume_mode() -> String {
    "rw".to_string()
}

fn default_volume_type() -> String {
    "volume".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_config_deserialization() {
        let json = json!({
            "port": 32123,
            "tools": {
                "run_rspec": {
                    "description": "Run RSpec",
                    "command": "rspec",
                    "args": [
                        "--format",
                        "{format}",
                        { "if_true": "fail_fast", "args": ["--fail-fast"] },
                        "{...test_paths}"
                    ],
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "test_paths": { "type": "array" }
                        }
                    }
                }
            }
        });

        let config: McpConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.port, 32123);
        let tool = config.tools.get("run_rspec").unwrap();
        assert_eq!(tool.command, "rspec");
        assert_eq!(tool.args.len(), 4);

        match &tool.args[2] {
            ArgTemplate::Conditional(c) => {
                assert_eq!(c.if_true, Some("fail_fast".to_string()));
                assert_eq!(c.args, vec!["--fail-fast".to_string()]);
            }
            _ => panic!("Expected conditional block"),
        }
    }

    #[test]
    fn test_mcp_config_with_env_and_optional_args() {
        let json = json!({
            "description": "Test Tool",
            "command": "ls",
            "working_dir": "/tmp/sandbox",
            "args": [
                { "if_present": "dir", "args": ["{dir}"] }
            ],
            "env": {
                "inherit": ["HOME"],
                "set": { "DEBUG": "1" }
            },
            "parameters": {}
        });

        let tool: McpToolConfig = serde_json::from_value(json).unwrap();
        assert_eq!(tool.command, "ls");
        assert_eq!(tool.working_dir, Some("/tmp/sandbox".to_string()));
        assert_eq!(tool.env.as_ref().unwrap().inherit, vec!["HOME"]);
        assert_eq!(tool.env.as_ref().unwrap().set.get("DEBUG").unwrap(), "1");
    }
}
