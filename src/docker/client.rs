use anyhow::{bail, Context, Result};
use std::process::Command;

use crate::docker::args;

pub struct DockerClient;

impl DockerClient {
    pub fn is_container_running(&self, name: &str) -> Result<bool> {
        let ps_args = args::build_ps_args(name);
        let output = self.query_command(ps_args)?;
        Ok(!output.trim().is_empty())
    }

    pub fn image_exists(&self, tag: &str) -> Result<bool> {
        let image_args = args::build_image_exists_args(tag);
        let output = self.query_command(image_args)?;
        Ok(!output.trim().is_empty())
    }

    pub fn run_command(&self, args: Vec<String>) -> Result<()> {
        let output = Command::new("docker")
            .args(&args)
            .output()
            .with_context(|| format!("failed to spawn `docker {}`", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "`docker {}` failed ({})\n{}",
                args.join(" "),
                output.status,
                stderr.trim()
            );
        }

        Ok(())
    }

    pub fn query_command(&self, args: Vec<String>) -> Result<String> {
        let output = Command::new("docker")
            .args(&args)
            .output()
            .with_context(|| format!("failed to spawn `docker {}`", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "`docker {}` failed ({})\n{}",
                args.join(" "),
                output.status,
                stderr.trim()
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn stream_command(&self, args: Vec<String>) -> Result<()> {
        let status = Command::new("docker")
            .args(&args)
            .status()
            .with_context(|| format!("failed to spawn `docker {}`", args.join(" ")))?;

        if !status.success() {
            bail!("`docker {}` failed ({})", args.join(" "), status);
        }

        Ok(())
    }

    /// Replace the current process with `docker <args>` via execvp.
    ///
    /// This function never returns on success — the OS replaces the process
    /// image. It returns an `anyhow::Error` only if the exec syscall itself
    /// fails (e.g. docker not found in PATH).
    pub fn exec_command(&self, args: Vec<String>) -> anyhow::Error {
        use std::os::unix::process::CommandExt;
        let err = Command::new("docker").args(&args).exec();
        anyhow::Error::from(err)
    }
}
