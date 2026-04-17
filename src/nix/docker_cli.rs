use super::docker::{DockerClient, DockerError, Result};
use std::process::Command;

/// Real Docker client that executes docker CLI commands
pub struct DockerCliClient;

impl DockerClient for DockerCliClient {
    fn is_container_running(&self, name: &str) -> Result<bool> {
        let output = Command::new("docker")
            .args([
                "ps",
                "--filter",
                &format!("name=^{}$", name),
                "--format",
                "{{.Names}}",
            ])
            .output()?;

        if !output.status.success() {
            return Err(DockerError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(!stdout.trim().is_empty())
    }

    fn run_container(
        &self,
        name: &str,
        image: &str,
        volumes: &[&str],
        detached: bool,
        remove: bool,
    ) -> Result<()> {
        let mut args = vec!["run"];

        if detached {
            args.push("-d");
        }

        if remove {
            args.push("--rm");
        }

        args.push("--name");
        args.push(name);

        // Add volume mounts
        for volume in volumes {
            args.push("-v");
            args.push(volume);
        }

        args.push(image);

        let output = Command::new("docker").args(&args).output()?;

        if !output.status.success() {
            return Err(DockerError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }
}
