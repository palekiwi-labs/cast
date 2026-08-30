use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,

    // Resource Limits
    pub memory: String,
    pub cpus: f64,
    pub pids_limit: i32,

    // Networking
    pub network: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    pub add_host_docker_internal: bool,

    // Nix devshells
    /// Full nix flake ref for the sandbox (outer) devshell, passed
    /// verbatim to `nix develop` inside the container (e.g.
    /// `~/.config/cast/nix#default`, `github:org/repo#shell`).
    /// Unset = no sandbox layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_shell: Option<String>,

    /// Full nix flake ref for the project (inner) devshell, passed
    /// verbatim to `nix develop`. Relative refs resolve against the
    /// workspace inside the container. Unset = no project layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_shell: Option<String>,

    /// Wrap sessions in the sandbox devshell when `sandbox_shell` is set.
    /// A disabled layer is skipped silently.
    #[serde(default = "default_true")]
    pub use_sandbox_shell: bool,

    /// Wrap sessions in the project devshell when `project_shell` is set.
    /// A disabled layer is skipped silently.
    #[serde(default = "default_true")]
    pub use_project_shell: bool,

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
    pub env_passthrough: Vec<String>,

    #[serde(default)]
    pub extra_env_passthrough: Vec<String>,

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
    #[serde(default = "default_mcp_global_timeout")]
    pub global_timeout_secs: u64,
    #[serde(default)]
    pub tools: BTreeMap<String, McpToolConfig>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_MCP_PORT,
            hostname: DEFAULT_MCP_HOSTNAME.to_string(),
            global_timeout_secs: default_mcp_global_timeout(),
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

fn default_mcp_global_timeout() -> u64 {
    300
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
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
            container_name: None,
            memory: "1024m".to_string(),
            cpus: 1.0,
            pids_limit: 512,
            network: "bridge".to_string(),
            port: None,
            add_host_docker_internal: true,
            sandbox_shell: None,
            project_shell: None,
            use_sandbox_shell: true,
            use_project_shell: true,
            volumes_namespace: "cast".to_string(),
            extra_data_volumes: BTreeMap::new(),
            nix_volume_name: "cast-nix-herdr-spike".to_string(),
            nix_daemon_container_name: "cast-nix-daemon-herdr-spike".to_string(),
            nix_extra_substituters: Vec::new(),
            nix_extra_trusted_public_keys: Vec::new(),
            forbidden_paths: Vec::new(),
            env_passthrough: Vec::new(),
            extra_env_passthrough: Vec::new(),
            mcp: McpConfig::default(),
        }
    }
}

fn default_volume_mode() -> String {
    "rw".to_string()
}

fn default_true() -> bool {
    true
}

fn default_volume_type() -> String {
    "volume".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn spike_defaults_isolate_the_nix_daemon_and_store() {
        let config = Config::default();

        assert_eq!(
            config.nix_daemon_container_name,
            "cast-nix-daemon-herdr-spike"
        );
        assert_eq!(config.nix_volume_name, "cast-nix-herdr-spike");
    }

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

    #[test]
    fn test_mcp_config_deserializes_global_timeout() {
        let json = json!({
            "port": 32123,
            "global_timeout_secs": 120,
            "tools": {}
        });
        let config: McpConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.global_timeout_secs, 120u64);
    }

    #[test]
    fn test_mcp_config_default_global_timeout() {
        let json = json!({
            "port": 32123,
            "tools": {}
        });
        let config: McpConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.global_timeout_secs, 300);
    }

    #[test]
    fn test_mcp_tool_config_deserializes_timeout() {
        let json = json!({
            "description": "A tool",
            "command": "sleep",
            "args": [],
            "parameters": {}
        });
        let tool: McpToolConfig = serde_json::from_value(json).unwrap();
        assert_eq!(tool.timeout_secs, None);
    }

    #[test]
    fn test_mcp_tool_config_deserializes_explicit_timeout() {
        let json = json!({
            "description": "A tool",
            "command": "sleep",
            "args": [],
            "parameters": {},
            "timeout_secs": 30
        });
        let tool: McpToolConfig = serde_json::from_value(json).unwrap();
        assert_eq!(tool.timeout_secs, Some(30));
    }

    #[test]
    fn test_agent_versions_in_json_is_silently_ignored() {
        // agent_versions was removed from Config; existing JSON with the key
        // must still load without error (no deny_unknown_fields on Config).
        let mut json = serde_json::to_value(Config::default()).unwrap();
        json["agent_versions"] = serde_json::json!({"opencode": "0.1.0"});
        let result: Result<Config, _> = serde_json::from_value(json);
        assert!(
            result.is_ok(),
            "agent_versions should be silently ignored: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_config_defaults_have_no_shell_refs_and_layers_enabled() {
        let config = Config::default();

        assert_eq!(config.sandbox_shell, None);
        assert_eq!(config.project_shell, None);
        assert!(
            config.use_sandbox_shell,
            "use_sandbox_shell must default to true"
        );
        assert!(
            config.use_project_shell,
            "use_project_shell must default to true"
        );
    }

    #[test]
    fn test_partial_json_deserializes_with_shell_defaults() {
        // A JSON object naming only the shell fields must deserialize:
        // switches fall back to true and unset refs to None without the
        // loader's Serialized::defaults merge underneath.
        let json = json!({
            "memory": "2048m",
            "cpus": 2.0,
            "pids_limit": 256,
            "network": "host",
            "add_host_docker_internal": true,
            "volumes_namespace": "ns",
            "extra_data_volumes": {},
            "nix_volume_name": "v",
            "nix_daemon_container_name": "d",
            "nix_extra_substituters": [],
            "nix_extra_trusted_public_keys": [],
            "forbidden_paths": []
        });
        let config: Config = serde_json::from_value(json).unwrap();

        assert_eq!(config.sandbox_shell, None);
        assert_eq!(config.project_shell, None);
        assert!(config.use_sandbox_shell);
        assert!(config.use_project_shell);
    }

    #[test]
    fn test_shell_refs_and_switches_load_from_json() {
        let json = json!({
            "memory": "2048m",
            "cpus": 2.0,
            "pids_limit": 256,
            "network": "host",
            "add_host_docker_internal": true,
            "volumes_namespace": "ns",
            "extra_data_volumes": {},
            "nix_volume_name": "v",
            "nix_daemon_container_name": "d",
            "nix_extra_substituters": [],
            "nix_extra_trusted_public_keys": [],
            "forbidden_paths": [],
            "sandbox_shell": "~/.config/cast/nix#default",
            "project_shell": ".#ai",
            "use_sandbox_shell": false,
            "use_project_shell": false
        });
        let config: Config = serde_json::from_value(json).unwrap();

        assert_eq!(
            config.sandbox_shell.as_deref(),
            Some("~/.config/cast/nix#default")
        );
        assert_eq!(config.project_shell.as_deref(), Some(".#ai"));
        assert!(!config.use_sandbox_shell);
        assert!(!config.use_project_shell);
    }

    #[test]
    fn test_legacy_use_flake_keys_are_silently_ignored() {
        // use_flake/use_flake_path were removed at 0.2.0; existing JSON
        // carrying them must still load without error (no
        // deny_unknown_fields on Config).
        let mut json = serde_json::to_value(Config::default()).unwrap();
        json["use_flake"] = serde_json::json!(true);
        json["use_flake_path"] = serde_json::json!(".#shell");
        let result: Result<Config, _> = serde_json::from_value(json);
        assert!(
            result.is_ok(),
            "legacy use_flake keys should be silently ignored: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_universal_container_in_json_is_silently_ignored() {
        // universal_container was removed from Config; existing JSON with the
        // key must still load without error (no deny_unknown_fields on Config).
        let mut json = serde_json::to_value(Config::default()).unwrap();
        json["universal_container"] = serde_json::json!(true);
        let result: Result<Config, _> = serde_json::from_value(json);
        assert!(
            result.is_ok(),
            "universal_container should be silently ignored: {:?}",
            result.err()
        );
    }
}
