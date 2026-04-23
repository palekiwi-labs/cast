use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    // Container Identity & Version
    pub opencode_version: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,

    pub version_cache_ttl_hours: u32,

    // Resource Limits
    pub memory: String,
    pub cpus: f64,
    pub pids_limit: i32,
    pub tmp_size: String,
    pub workspace_tmp_size: String,

    // Networking
    pub network: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    pub publish_port: bool,
    pub add_host_docker_internal: bool,

    // Paths & Files
    pub opencode_config_dir: String,
    pub opencode_command: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rgignore_file: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_base_dockerfile: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_file: Option<String>,

    // Data Volumes
    pub data_volumes_name: String,

    pub extra_data_volumes: HashMap<String, VolumeConfig>,

    // Nix Workflow
    pub nix_volume_name: String,
    pub nix_daemon_container_name: String,
    pub nix_extra_substituters: Vec<String>,
    pub nix_extra_trusted_public_keys: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nix_opencode_command: Option<Vec<String>>,

    // Security
    pub read_only: bool,
    pub forbidden_paths: Vec<String>,

    // Environment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
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
        // In debug builds, prefix container/volume names to avoid conflicts with production
        #[cfg(debug_assertions)]
        let dev_prefix = "dev-";
        #[cfg(not(debug_assertions))]
        let dev_prefix = "";

        Config {
            opencode_version: "latest".to_string(),
            container_name: None,
            version_cache_ttl_hours: 24,
            memory: "1024m".to_string(),
            cpus: 1.0,
            pids_limit: 100,
            tmp_size: "500m".to_string(),
            workspace_tmp_size: "500m".to_string(),
            network: "bridge".to_string(),
            port: None,
            publish_port: true,
            add_host_docker_internal: true,
            opencode_config_dir: "~/.config/opencode".to_string(),
            opencode_command: vec!["opencode".to_string()],
            rgignore_file: None,
            custom_base_dockerfile: None,
            env_file: None,
            data_volumes_name: "ocx".to_string(),
            extra_data_volumes: HashMap::new(),
            nix_volume_name: format!("{}ocx-nix", dev_prefix),
            nix_daemon_container_name: format!("{}ocx-nix-daemon", dev_prefix),
            nix_extra_substituters: Vec::new(),
            nix_extra_trusted_public_keys: Vec::new(),
            nix_opencode_command: None,
            read_only: false,
            forbidden_paths: Vec::new(),
            timezone: None,
        }
    }
}

fn default_volume_mode() -> String {
    "rw".to_string()
}

fn default_volume_type() -> String {
    "volume".to_string()
}
