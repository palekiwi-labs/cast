use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    // Container Identity & Version
    #[serde(default)]
    pub agent_versions: HashMap<String, String>,

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

    pub extra_data_volumes: HashMap<String, VolumeConfig>,

    // Nix Workflow
    pub nix_volume_name: String,
    pub nix_daemon_container_name: String,
    pub nix_extra_substituters: Vec<String>,
    pub nix_extra_trusted_public_keys: Vec<String>,

    // Security
    pub forbidden_paths: Vec<String>,
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
            agent_versions: HashMap::new(),
            container_name: None,
            version_cache_ttl_hours: 24,
            memory: "1024m".to_string(),
            cpus: 1.0,
            pids_limit: 100,
            network: "bridge".to_string(),
            port: None,
            publish_port: true,
            add_host_docker_internal: true,
            use_flake: true,
            use_flake_path: None,
            volumes_namespace: "cast".to_string(),
            extra_data_volumes: HashMap::new(),
            nix_volume_name: "cast-nix".to_string(),
            nix_daemon_container_name: "cast-nix-daemon".to_string(),
            nix_extra_substituters: Vec::new(),
            nix_extra_trusted_public_keys: Vec::new(),
            forbidden_paths: Vec::new(),
        }
    }
}

fn default_volume_mode() -> String {
    "rw".to_string()
}

fn default_volume_type() -> String {
    "volume".to_string()
}
