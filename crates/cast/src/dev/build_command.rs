use crate::config::Config;
use crate::dev::run::RunOpts;

/// Build the command vector that will be passed to `docker run` after all flags.
/// Handles the nested Nix develop wrapping logic.
pub fn build_command(
    config: &Config,
    opts: &RunOpts,
    base_command: &str,
    extra_args: Vec<String>,
) -> Vec<String> {
    // Calculate estimated capacity to avoid reallocations.
    // Each flake layer adds 4 arguments.
    let mut capacity = 1 + extra_args.len();
    if opts.user_flake_present {
        capacity += 4;
    }
    if config.use_flake && (config.use_flake_path.is_some() || opts.project_flake_present) {
        capacity += 4;
    }

    let mut cmd = Vec::with_capacity(capacity);

    // Global flake (outer layer - always applies if present)
    if opts.user_flake_present {
        let global_flake = format!("/home/{}/.config/cast/nix", opts.user.username);
        cmd.extend([
            "nix".to_string(),
            "develop".to_string(),
            global_flake,
            "-c".to_string(),
        ]);
    }

    // Project flake (inner layer)
    if config.use_flake {
        let project_flake = if let Some(path) = &config.use_flake_path {
            Some(path.as_str())
        } else if opts.project_flake_present {
            Some(".")
        } else {
            None
        };

        if let Some(flake_ref) = project_flake {
            cmd.extend([
                "nix".to_string(),
                "develop".to_string(),
                flake_ref.to_string(),
                "-c".to_string(),
            ]);
        }
    }

    // Base command and extra args
    cmd.push(base_command.to_string());
    cmd.extend(extra_args);

    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn run_opts(user_flake_present: bool, project_flake_present: bool) -> RunOpts {
        RunOpts {
            workspace: ResolvedWorkspace {
                root: PathBuf::from("/work"),
                container_path: PathBuf::from("/work"),
            },
            user: alice(),
            port: 8080,
            host_home_dir: None,
            user_flake_present,
            project_flake_present,
        }
    }

    #[test]
    fn test_build_command_use_flake_false_no_global() {
        let config = Config {
            use_flake: false,
            use_flake_path: Some(".#my-shell".to_string()),
            ..Default::default()
        };

        // Scenario 1: use_flake false, no global flake -> bare command
        let opts = run_opts(false, true);
        let cmd = build_command(&config, &opts, "test", vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["test", "arg1"]);
    }

    #[test]
    fn test_build_command_use_flake_false_with_global() {
        let config = Config {
            use_flake: false,
            use_flake_path: Some(".#my-shell".to_string()),
            ..Default::default()
        };

        // Scenario 2: use_flake false, global flake present -> wrapped ONLY in global flake
        let opts = run_opts(true, true);
        let cmd = build_command(&config, &opts, "test", vec!["arg1".to_string()]);
        assert_eq!(
            cmd,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix",
                "-c",
                "test",
                "arg1"
            ]
        );
    }

    #[test]
    fn test_build_command_use_flake_true_no_global_no_project() {
        let config = Config {
            use_flake: true,
            use_flake_path: None,
            ..Default::default()
        };

        // Scenario 3: use_flake true, no global, no project flake -> bare command
        let opts = run_opts(false, false);
        let cmd = build_command(&config, &opts, "test", vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["test", "arg1"]);
    }

    #[test]
    fn test_build_command_use_flake_true_no_global_with_project_auto() {
        let config = Config {
            use_flake: true,
            use_flake_path: None,
            ..Default::default()
        };

        // Scenario 4: use_flake true, no global, project flake present (auto-detect)
        let opts = run_opts(false, true);
        let cmd = build_command(&config, &opts, "test", vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["nix", "develop", ".", "-c", "test", "arg1"]);
    }

    #[test]
    fn test_build_command_use_flake_true_no_global_with_project_path() {
        let config = Config {
            use_flake: true,
            use_flake_path: Some(".#shell".to_string()),
            ..Default::default()
        };

        // Scenario 5: use_flake true, no global, explicit project flake path
        let opts = run_opts(false, false); // project_flake_present false to prove path overrides it
        let cmd = build_command(&config, &opts, "test", vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["nix", "develop", ".#shell", "-c", "test", "arg1"]);
    }

    #[test]
    fn test_build_command_use_flake_true_with_global_and_project_auto() {
        let config = Config {
            use_flake: true,
            use_flake_path: None,
            ..Default::default()
        };

        // Scenario 6: use_flake true, global present, project auto-detected -> nested wrap
        let opts = run_opts(true, true);
        let cmd = build_command(&config, &opts, "test", vec!["arg1".to_string()]);
        assert_eq!(
            cmd,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix",
                "-c",
                "nix",
                "develop",
                ".",
                "-c",
                "test",
                "arg1"
            ]
        );
    }

    #[test]
    fn test_build_command_use_flake_true_with_global_and_project_path() {
        let config = Config {
            use_flake: true,
            use_flake_path: Some(".#shell".to_string()),
            ..Default::default()
        };

        // Scenario 7: use_flake true, global present, explicit project path -> nested wrap
        let opts = run_opts(true, false);
        let cmd = build_command(&config, &opts, "test", vec!["arg1".to_string()]);
        assert_eq!(
            cmd,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix",
                "-c",
                "nix",
                "develop",
                ".#shell",
                "-c",
                "test",
                "arg1"
            ]
        );
    }
}
