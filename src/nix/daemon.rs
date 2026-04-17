use crate::config::Config;
use crate::nix::docker::{DockerClient, Result};

/// Default Docker image for the nix daemon
const NIX_DAEMON_IMAGE: &str = "nixos/nix:latest";

/// Ensure the nix daemon container is running
pub fn ensure_running<D: DockerClient>(docker: &D, config: &Config) -> Result<()> {
    let container_name = &config.nix_daemon_container_name;

    // Check if already running
    if docker.is_container_running(container_name)? {
        println!("Nix daemon is already running: {}", container_name);
        return Ok(());
    }

    // Start the daemon container
    println!("Starting nix daemon container: {}", container_name);

    let volume_mount = format!("{}:/nix:rw", &config.nix_volume_name);
    let volumes = vec![volume_mount.as_str()];

    docker.run_container(
        container_name,
        NIX_DAEMON_IMAGE,
        &volumes,
        true, // detached
        true, // remove on stop
    )?;

    println!("Nix daemon started successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MockDockerClient {
        running_containers: RefCell<Vec<String>>,
        started_containers: RefCell<Vec<String>>,
    }

    impl MockDockerClient {
        fn new() -> Self {
            Self {
                running_containers: RefCell::new(Vec::new()),
                started_containers: RefCell::new(Vec::new()),
            }
        }

        fn set_running(&self, name: &str) {
            self.running_containers.borrow_mut().push(name.to_string());
        }
    }

    impl DockerClient for MockDockerClient {
        fn is_container_running(&self, name: &str) -> Result<bool> {
            Ok(self.running_containers.borrow().contains(&name.to_string()))
        }

        fn run_container(
            &self,
            name: &str,
            _image: &str,
            _volumes: &[&str],
            _detached: bool,
            _remove: bool,
        ) -> Result<()> {
            self.started_containers.borrow_mut().push(name.to_string());
            self.set_running(name);
            Ok(())
        }
    }

    fn default_config() -> Config {
        Config {
            nix_daemon_container_name: "ocx-nix-daemon".to_string(),
            nix_volume_name: "ocx-nix".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_ensure_running_starts_container_when_not_running() {
        let docker = MockDockerClient::new();
        let config = default_config();

        let result = ensure_running(&docker, &config);

        assert!(result.is_ok());
        assert_eq!(docker.started_containers.borrow().len(), 1);
        assert_eq!(docker.started_containers.borrow()[0], "ocx-nix-daemon");
    }

    #[test]
    fn test_ensure_running_does_not_restart_if_already_running() {
        let docker = MockDockerClient::new();
        docker.set_running("ocx-nix-daemon");
        let config = default_config();

        let result = ensure_running(&docker, &config);

        assert!(result.is_ok());
        assert_eq!(docker.started_containers.borrow().len(), 0);
    }
}
