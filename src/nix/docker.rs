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

    /// Start a container with the given configuration
    fn run_container(
        &self,
        name: &str,
        image: &str,
        volumes: &[&str], // Full volume mount strings (e.g., "ocx-nix:/nix:rw")
        detached: bool,
        remove: bool,
    ) -> Result<()>;
}
