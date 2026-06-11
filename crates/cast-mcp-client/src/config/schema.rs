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

pub(super) fn default_enabled() -> bool {
    true
}
