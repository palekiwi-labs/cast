use anyhow::{bail, Context, Result};
use std::process::{Command, ExitStatus};
use tracing::debug;

use crate::docker::args;

pub struct DockerClient;

/// RAII guard to ignore SIGINT and SIGQUIT in the parent process and restore
/// them to their previous handlers when dropped.
struct SignalGuard {
    old_int: libc::sighandler_t,
    old_quit: libc::sighandler_t,
}

impl SignalGuard {
    fn new() -> Self {
        unsafe {
            Self {
                old_int: libc::signal(libc::SIGINT, libc::SIG_IGN),
                old_quit: libc::signal(libc::SIGQUIT, libc::SIG_IGN),
            }
        }
    }
}

impl Drop for SignalGuard {
    fn drop(&mut self) {
        unsafe {
            libc::signal(libc::SIGINT, self.old_int);
            libc::signal(libc::SIGQUIT, self.old_quit);
        }
    }
}

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
        debug!(command = "docker", args = ?args, "executing command");
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
        debug!(command = "docker", args = ?args, "querying command");
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
        debug!(command = "docker", args = ?args, "streaming command");
        let status = Command::new("docker")
            .args(&args)
            .status()
            .with_context(|| format!("failed to spawn `docker {}`", args.join(" ")))?;

        if !status.success() {
            bail!("`docker {}` failed ({})", args.join(" "), status);
        }

        Ok(())
    }

    /// Run `docker <args>` as a supervised child process with TTY inheritance.
    ///
    /// This keeps `cast` alive to monitor the container's lifecycle and capture
    /// its exit code. It ignores SIGINT and SIGQUIT in the parent to allow
    /// Docker to handle them, resetting them to SIG_DFL in the child.
    pub fn interactive_command(&self, args: Vec<String>) -> Result<ExitStatus> {
        use std::os::unix::process::CommandExt;

        debug!(command = "docker", args = ?args, "starting interactive command");
        // Ignore SIGINT (Ctrl+C) and SIGQUIT (Ctrl+\) in cast so we can
        // wait for Docker to handle them and exit gracefully.
        let _guard = SignalGuard::new();

        let mut cmd = Command::new("docker");
        cmd.args(&args);

        // Reset signals to default in the child process so Docker handles them.
        unsafe {
            cmd.pre_exec(|| {
                libc::signal(libc::SIGINT, libc::SIG_DFL);
                libc::signal(libc::SIGQUIT, libc::SIG_DFL);
                Ok(())
            });
        }

        let status = cmd
            .status()
            .with_context(|| format!("failed to spawn `docker {}`", args.join(" ")))?;

        Ok(status)
    }
}
