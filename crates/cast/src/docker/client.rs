use anyhow::{Context, Result, bail};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
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

/// Atomic flag set by `headless_signal_handler`. A `static` is required
/// because signal handlers cannot capture state.
static HEADLESS_SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Async-signal-safe handler: sets the shutdown flag and returns immediately.
/// All cleanup work happens in normal code (the poll loop), not here.
extern "C" fn headless_signal_handler(_sig: libc::c_int) {
    HEADLESS_SHUTDOWN.store(true, Ordering::SeqCst);
}

/// RAII guard for headless mode: installs `headless_signal_handler` for
/// SIGINT, SIGTERM, and SIGQUIT, and restores the previous handlers on drop.
struct HeadlessSignalGuard {
    old_int: libc::sighandler_t,
    old_term: libc::sighandler_t,
    old_quit: libc::sighandler_t,
}

impl HeadlessSignalGuard {
    fn new() -> Self {
        HEADLESS_SHUTDOWN.store(false, Ordering::SeqCst);
        let handler = headless_signal_handler as *const () as libc::sighandler_t;
        unsafe {
            Self {
                old_int: libc::signal(libc::SIGINT, handler),
                old_term: libc::signal(libc::SIGTERM, handler),
                old_quit: libc::signal(libc::SIGQUIT, handler),
            }
        }
    }
}

impl Drop for HeadlessSignalGuard {
    fn drop(&mut self) {
        unsafe {
            libc::signal(libc::SIGINT, self.old_int);
            libc::signal(libc::SIGTERM, self.old_term);
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

    /// Run `docker <args>` without a pseudo-TTY (headless / non-interactive).
    ///
    /// cast acts as the supervisor: it catches SIGINT, SIGTERM, and SIGQUIT
    /// and responds by calling `docker stop <container_name>`, which triggers
    /// `--rm` cleanup deterministically. The container name is passed by the
    /// caller so cleanup can happen from normal code (not the signal handler).
    ///
    /// Stdout and stderr are inherited so output flows cleanly to pipes or logs.
    /// Stdin is closed (`/dev/null`) so docker never blocks waiting for input.
    ///
    /// Returns the `ExitStatus` without bailing on non-zero exit codes, allowing
    /// the caller to decide how to handle agent failures.
    pub fn headless_command(&self, args: Vec<String>, container_name: &str) -> Result<ExitStatus> {
        debug!(
            command = "docker",
            args = ?args,
            container = container_name,
            "starting headless command"
        );

        // Install signal handlers for SIGINT, SIGTERM, SIGQUIT. These only set
        // the HEADLESS_SHUTDOWN flag; all cleanup happens in the poll loop below.
        // Unlike interactive_command, we do NOT ignore these signals — cast is
        // the supervisor in headless mode and must drive orderly container stop.
        let _guard = HeadlessSignalGuard::new();

        let mut child = Command::new("docker")
            .args(&args)
            // Stdin closed: docker must not block waiting for input.
            .stdin(Stdio::null())
            // No pre_exec signal reset: cast (not docker) is the supervisor here.
            .spawn()
            .with_context(|| format!("failed to spawn `docker {}`", args.join(" ")))?;

        let mut stop_issued = false;
        loop {
            // Non-blocking reap: if docker run has exited, return its status.
            if let Some(status) = child.try_wait()? {
                return Ok(status);
            }

            // If a signal arrived, issue `docker stop` exactly once from normal
            // code (async-signal-unsafe operations are forbidden in the handler).
            if HEADLESS_SHUTDOWN.load(Ordering::SeqCst) && !stop_issued {
                stop_issued = true;
                debug!(
                    container = container_name,
                    "signal received; stopping container"
                );
                // docker stop sends SIGTERM then SIGKILL after --time seconds,
                // and triggers --rm cleanup. Errors are best-effort.
                let _ = Command::new("docker")
                    .args(["stop", "--time", "10", container_name])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                // Loop continues; docker run will exit once the container stops.
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
