use super::docker::{DockerClient, DockerError, Result};
use std::path::Path;
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

/// Build arguments for `docker images` command to check if an image exists
pub fn build_image_exists_args(tag: &str) -> Vec<String> {
    vec![
        "images".to_string(),
        "--filter".to_string(),
        format!("reference={}", tag),
        "--format".to_string(),
        "{{.Repository}}:{{.Tag}}".to_string(),
    ]
}

/// Build arguments for `docker build` command
pub fn build_build_args(tag: &str, context_path: &Path) -> Vec<String> {
    vec![
        "build".to_string(),
        "-t".to_string(),
        tag.to_string(),
        context_path.to_string_lossy().to_string(),
    ]
}

/// Build arguments for `docker run` command
pub fn build_run_args(
    name: &str,
    image: &str,
    volumes: &[&str],
    env_vars: &[(&str, &str)],
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

    // Add environment variables
    for (key, value) in env_vars {
        args.push("-e".to_string());
        args.push(format!("{}={}", key, value));
    }

    // Add volume mounts
    for volume in volumes {
        args.push("-v".to_string());
        args.push(volume.to_string());
    }

    args.push(image.to_string());

    args
}

/// Build arguments for `docker stop` command
pub fn build_stop_args(name: &str) -> Vec<String> {
    vec!["stop".to_string(), name.to_string()]
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

    fn image_exists(&self, tag: &str) -> Result<bool> {
        let args = build_image_exists_args(tag);
        let output = Command::new("docker").args(&args).output()?;

        if !output.status.success() {
            return Err(DockerError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(!stdout.trim().is_empty())
    }

    fn build_image(&self, tag: &str, context_path: &Path) -> Result<()> {
        let args = build_build_args(tag, context_path);

        // Use .status() to allow Docker build output to stream to the terminal
        let status = Command::new("docker").args(&args).status()?;

        if !status.success() {
            return Err(DockerError::CommandFailed(format!(
                "docker build failed with exit code: {}",
                status.code().unwrap_or(-1)
            )));
        }

        Ok(())
    }

    fn run_container(
        &self,
        name: &str,
        image: &str,
        volumes: &[&str],
        env_vars: &[(&str, &str)],
        detached: bool,
        remove: bool,
    ) -> Result<()> {
        let args = build_run_args(name, image, volumes, env_vars, detached, remove);
        let output = Command::new("docker").args(&args).output()?;

        if !output.status.success() {
            return Err(DockerError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    fn stop_container(&self, name: &str) -> Result<()> {
        let args = build_stop_args(name);
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
    fn test_build_image_exists_args() {
        let args = build_image_exists_args("my-image:tag");

        assert_eq!(
            args,
            vec![
                "images",
                "--filter",
                "reference=my-image:tag",
                "--format",
                "{{.Repository}}:{{.Tag}}"
            ]
        );
    }

    #[test]
    fn test_build_build_args() {
        let context = Path::new("/tmp/build");
        let args = build_build_args("my-image:tag", context);

        assert_eq!(args, vec!["build", "-t", "my-image:tag", "/tmp/build"]);
    }

    #[test]
    fn test_build_run_args_minimal() {
        let args = build_run_args("test-container", "nginx:latest", &[], &[], false, false);

        assert_eq!(
            args,
            vec!["run", "--name", "test-container", "nginx:latest"]
        );
    }

    #[test]
    fn test_build_run_args_with_detached() {
        let args = build_run_args("test-container", "nginx:latest", &[], &[], true, false);

        assert_eq!(
            args,
            vec!["run", "-d", "--name", "test-container", "nginx:latest"]
        );
    }

    #[test]
    fn test_build_run_args_with_remove() {
        let args = build_run_args("test-container", "nginx:latest", &[], &[], false, true);

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
            &[],
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
    fn test_build_run_args_with_env_vars() {
        let args = build_run_args(
            "test-container",
            "nginx:latest",
            &[],
            &[("FOO", "bar"), ("BAZ", "qux")],
            false,
            false,
        );

        assert_eq!(
            args,
            vec![
                "run",
                "--name",
                "test-container",
                "-e",
                "FOO=bar",
                "-e",
                "BAZ=qux",
                "nginx:latest"
            ]
        );
    }

    #[test]
    fn test_build_run_args_nix_daemon_full() {
        // Test the actual command that would be used for the nix daemon
        let args = build_run_args(
            "ocx-nix-daemon",
            "localhost/ocx-nix-daemon:sha-12345678",
            &["ocx-nix:/nix:rw"],
            &[(
                "NIX_CONF_CONTENT",
                "experimental-features = nix-command flakes",
            )],
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
                "-e",
                "NIX_CONF_CONTENT=experimental-features = nix-command flakes",
                "-v",
                "ocx-nix:/nix:rw",
                "localhost/ocx-nix-daemon:sha-12345678"
            ]
        );
    }

    #[test]
    fn test_build_stop_args() {
        let args = build_stop_args("my-container");
        assert_eq!(args, vec!["stop", "my-container"]);
    }
}
