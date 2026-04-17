use super::docker::{DockerClient, DockerError, Result};
use std::process::Command;

/// Build arguments for `docker ps` command to check if a container is running
pub fn build_ps_args(name: &str) -> Vec<String> {
    vec![
        "ps".to_string(),
        "--filter".to_string(),
        format!("name=^{}$", name),
        "--format".to_string(),
        "{{.Names}}".to_string(),
    ]
}

/// Build arguments for `docker run` command
pub fn build_run_args(
    name: &str,
    image: &str,
    volumes: &[&str],
    detached: bool,
    remove: bool,
) -> Vec<String> {
    let mut args = vec!["run".to_string()];

    if detached {
        args.push("-d".to_string());
    }

    if remove {
        args.push("--rm".to_string());
    }

    args.push("--name".to_string());
    args.push(name.to_string());

    // Add volume mounts
    for volume in volumes {
        args.push("-v".to_string());
        args.push(volume.to_string());
    }

    args.push(image.to_string());

    args
}

/// Real Docker client that executes docker CLI commands
pub struct DockerCliClient;

impl DockerClient for DockerCliClient {
    fn is_container_running(&self, name: &str) -> Result<bool> {
        let args = build_ps_args(name);
        let output = Command::new("docker").args(&args).output()?;

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
        let args = build_run_args(name, image, volumes, detached, remove);
        let output = Command::new("docker").args(&args).output()?;

        if !output.status.success() {
            return Err(DockerError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ps_args() {
        let args = build_ps_args("my-container");

        assert_eq!(
            args,
            vec![
                "ps",
                "--filter",
                "name=^my-container$",
                "--format",
                "{{.Names}}"
            ]
        );
    }

    #[test]
    fn test_build_run_args_minimal() {
        let args = build_run_args("test-container", "nginx:latest", &[], false, false);

        assert_eq!(
            args,
            vec!["run", "--name", "test-container", "nginx:latest"]
        );
    }

    #[test]
    fn test_build_run_args_with_detached() {
        let args = build_run_args("test-container", "nginx:latest", &[], true, false);

        assert_eq!(
            args,
            vec!["run", "-d", "--name", "test-container", "nginx:latest"]
        );
    }

    #[test]
    fn test_build_run_args_with_remove() {
        let args = build_run_args("test-container", "nginx:latest", &[], false, true);

        assert_eq!(
            args,
            vec!["run", "--rm", "--name", "test-container", "nginx:latest"]
        );
    }

    #[test]
    fn test_build_run_args_with_single_volume() {
        let args = build_run_args(
            "test-container",
            "nginx:latest",
            &["my-volume:/data:rw"],
            false,
            false,
        );

        assert_eq!(
            args,
            vec![
                "run",
                "--name",
                "test-container",
                "-v",
                "my-volume:/data:rw",
                "nginx:latest"
            ]
        );
    }

    #[test]
    fn test_build_run_args_with_multiple_volumes() {
        let args = build_run_args(
            "test-container",
            "nginx:latest",
            &["vol1:/data1:rw", "vol2:/data2:ro"],
            false,
            false,
        );

        assert_eq!(
            args,
            vec![
                "run",
                "--name",
                "test-container",
                "-v",
                "vol1:/data1:rw",
                "-v",
                "vol2:/data2:ro",
                "nginx:latest"
            ]
        );
    }

    #[test]
    fn test_build_run_args_nix_daemon_full() {
        // Test the actual command that would be used for the nix daemon
        let args = build_run_args(
            "ocx-nix-daemon",
            "nixos/nix:latest",
            &["ocx-nix:/nix:rw"],
            true,
            true,
        );

        assert_eq!(
            args,
            vec![
                "run",
                "-d",
                "--rm",
                "--name",
                "ocx-nix-daemon",
                "-v",
                "ocx-nix:/nix:rw",
                "nixos/nix:latest"
            ]
        );
    }

    #[test]
    fn test_build_run_args_all_flags() {
        let args = build_run_args("test-container", "test:image", &["vol:/mnt:rw"], true, true);

        assert_eq!(
            args,
            vec![
                "run",
                "-d",
                "--rm",
                "--name",
                "test-container",
                "-v",
                "vol:/mnt:rw",
                "test:image"
            ]
        );
    }
}
