use crate::config::Config;
use crate::dev::env_passthrough::build_passthrough_env_args;
use crate::dev::workspace::ResolvedWorkspace;
use crate::user::ResolvedUser;

/// Build the full set of Docker run flags for an OpenCode session.
pub fn build_run_opts(
    config: &Config,
    user: &ResolvedUser,
    workspace: &ResolvedWorkspace,
    port: u16,
) -> Vec<String> {
    let mut opts: Vec<String> = vec![
        "--rm".to_string(),
        "-it".to_string(),
        // Security hardening
        "--security-opt".to_string(),
        "no-new-privileges".to_string(),
        "--cap-drop".to_string(),
        "ALL".to_string(),
        // Resource constraints
        "--memory".to_string(),
        config.memory.clone(),
        "--cpus".to_string(),
        config.cpus.to_string(),
        "--pids-limit".to_string(),
        config.pids_limit.to_string(),
        // Network
        "--network".to_string(),
        config.network.clone(),
    ];

    // Port publishing.
    if config.publish_port {
        opts.push("-p".to_string());
        opts.push(format!("{}:80", port));
    }

    // Environment: user identity and terminal capabilities.
    opts.extend([
        "-e".to_string(),
        format!("USER={}", user.username),
        "-e".to_string(),
        "TERM=xterm-256color".to_string(),
        "-e".to_string(),
        "COLORTERM=truecolor".to_string(),
        "-e".to_string(),
        "FORCE_COLOR=1".to_string(),
    ]);

    // LLM API keys and OpenCode-specific env vars present on the host.
    opts.extend(build_passthrough_env_args());

    // Workspace bind mount.
    opts.extend([
        "-v".to_string(),
        format!(
            "{}:{}:rw",
            workspace.root.display(),
            workspace.container_path.display()
        ),
        "--workdir".to_string(),
        workspace.container_path.to_string_lossy().into_owned(),
    ]);

    opts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_run_opts_basic() {
        let config = Config::default();
        let user = ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        };
        let workspace = ResolvedWorkspace {
            root: PathBuf::from("/home/alice/project"),
            container_path: PathBuf::from("/home/alice/project"),
        };
        let port = 32768;

        let opts = build_run_opts(&config, &user, &workspace, port);

        // Check for key flags
        assert!(opts.contains(&"--rm".to_string()));
        assert!(opts.contains(&"-it".to_string()));
        assert!(opts.contains(&"no-new-privileges".to_string()));
        assert!(opts.contains(&"USER=alice".to_string()));
        assert!(opts.contains(&"/home/alice/project:/home/alice/project:rw".to_string()));

        // Port check
        if config.publish_port {
            assert!(opts.contains(&"32768:80".to_string()));
        }
    }
}
