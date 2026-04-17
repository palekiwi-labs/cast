use std::result::Result as StdResult;

/// Error type for Docker operations
#[derive(Debug, thiserror::Error)]
pub enum DockerError {
    #[error("Docker command failed: {0}")]
    CommandFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = StdResult<T, DockerError>;

/// Trait for Docker operations to enable mocking in tests
pub trait DockerClient {
    /// Check if a container is currently running
    fn is_container_running(&self, name: &str) -> Result<bool>;

    /// Check if an image with the given tag exists
    fn image_exists(&self, tag: &str) -> Result<bool>;

    /// Build an image from a context directory
    fn build_image(&self, tag: &str, context_path: &std::path::Path) -> Result<()>;

    /// Start a container with the given configuration
    fn run_container(
        &self,
        name: &str,
        image: &str,
        volumes: &[&str], // Full volume mount strings (e.g., "ocx-nix:/nix:rw")
        env_vars: &[(&str, &str)], // Environment variables (e.g., ("NIX_CONF_CONTENT", "content"))
        detached: bool,
        remove: bool,
    ) -> Result<()>;

    /// Stop a running container by name
    fn stop_container(&self, name: &str) -> Result<()>;
}
