use anyhow::{Context, Result};

use crate::dev::run::RunOpts;

pub mod config_dir;
pub mod registry;
pub mod volumes;

/// Prepare host-side state shared across every agent (universal mounts).
///
/// Currently ensures the cross-harness `~/.agents` config directory exists on
/// the host with correct user ownership before the container starts, so a
/// missing directory does not get auto-created by Docker as a root-owned path
/// when the bind mount is attached.
pub fn prepare_host(opts: &RunOpts) -> Result<()> {
    let home = opts
        .host_home_dir
        .as_deref()
        .context("Failed to resolve user home directory")?;
    config_dir::ensure_config_dir(home)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::run::{RunOpts, TtyMode};
    use crate::dev::workspace::ResolvedWorkspace;
    use crate::user::ResolvedUser;
    use std::path::PathBuf;

    fn alice() -> ResolvedUser {
        ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        }
    }

    fn opts_with_home(home: PathBuf) -> RunOpts {
        RunOpts {
            workspace: ResolvedWorkspace {
                container_path: PathBuf::from("/workspace"),
                root: PathBuf::from("/home/alice/project"),
            },
            user: alice(),
            port: 32768,
            host_home_dir: Some(home),
            host_name: "test-host".to_string(),
            user_flake_present: false,
            project_flake_present: false,
            tty_mode: TtyMode::Interactive,
            publish: false,
        }
    }

    #[test]
    fn prepare_host_creates_agents_dir_under_host_home() {
        let temp = tempfile::tempdir().unwrap();
        let opts = opts_with_home(temp.path().to_path_buf());

        prepare_host(&opts).unwrap();

        let agents_dir = temp.path().join(".agents");
        assert!(agents_dir.exists(), "expected .agents to be created");
        assert!(agents_dir.is_dir(), "expected .agents to be a directory");
    }

    #[test]
    fn prepare_host_errors_when_host_home_is_none() {
        let opts = RunOpts {
            host_home_dir: None,
            ..opts_with_home(PathBuf::from("/unused"))
        };

        let result = prepare_host(&opts);

        assert!(
            result.is_err(),
            "prepare_host must error when host_home_dir is None"
        );
    }
}
