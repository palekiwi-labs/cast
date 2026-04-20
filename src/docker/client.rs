use anyhow::{bail, Context, Result};
use std::process::Command;

pub struct DockerClient;

impl DockerClient {
    pub fn run_command(&self, args: Vec<String>) -> Result<()> {
        let output = Command::new("docker")
            .args(&args)
            .output()
            .with_context(|| format!("failed to spawn `docker {}`", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let detail = stderr.trim();
            if detail.is_empty() {
                bail!("`docker {}` failed ({})", args.join(" "), output.status);
            } else {
                bail!(
                    "`docker {}` failed ({})\n{}",
                    args.join(" "),
                    output.status,
                    detail
                );
            }
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
            let detail = stderr.trim();
            if detail.is_empty() {
                bail!("`docker {}` failed ({})", args.join(" "), output.status);
            } else {
                bail!(
                    "`docker {}` failed ({})\n{}",
                    args.join(" "),
                    output.status,
                    detail
                );
            }
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
}
