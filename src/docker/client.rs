use anyhow::{Result, bail};
use std::process::Command;

pub struct DockerClient;

impl DockerClient {
    pub fn run_command(&self, args: Vec<String>) -> Result<()> {
        let output = Command::new("docker").args(&args).output()?;

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
}
