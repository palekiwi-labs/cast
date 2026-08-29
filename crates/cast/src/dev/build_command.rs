use crate::config::Config;

/// Build the command vector that will be passed to `docker run` after all flags.
/// Handles the nested Nix develop wrapping logic.
///
/// Each layer (sandbox outer, project inner) wraps the command in
/// `nix develop <ref> -c` iff its ref is set AND its switch is true.
/// Leading `~/` refs resolve against the container user's home; other refs
/// are passed verbatim. A disabled layer is skipped silently.
/// Unresolvable refs are not cast's problem: they fail inside the
/// container.
pub fn build_command(
    config: &Config,
    container_username: &str,
    base_command: &str,
    extra_args: Vec<String>,
) -> Vec<String> {
    let sandbox = sandbox_layer(config);
    let project = project_layer(config);
    let mut capacity = 1 + extra_args.len();
    if sandbox.is_some() {
        capacity += 4;
    }
    if project.is_some() {
        capacity += 4;
    }

    let mut cmd = Vec::with_capacity(capacity);

    if let Some(ref_) = sandbox {
        cmd.extend([
            "nix".to_string(),
            "develop".to_string(),
            expand_container_home(ref_, container_username),
            "-c".to_string(),
        ]);
    }

    if let Some(ref_) = project {
        cmd.extend([
            "nix".to_string(),
            "develop".to_string(),
            expand_container_home(ref_, container_username),
            "-c".to_string(),
        ]);
    }

    cmd.push(base_command.to_string());
    cmd.extend(extra_args);

    cmd
}

/// The effective sandbox layer: the verbatim ref when `sandbox_shell` is
/// set and `use_sandbox_shell` is true, else `None`.
pub(crate) fn sandbox_layer(config: &Config) -> Option<&str> {
    effective_layer(config.sandbox_shell.as_deref(), config.use_sandbox_shell)
}

/// The effective project layer: the verbatim ref when `project_shell`
/// is set and `use_project_shell` is true, else `None`.
fn project_layer(config: &Config) -> Option<&str> {
    effective_layer(config.project_shell.as_deref(), config.use_project_shell)
}

fn effective_layer(shell: Option<&str>, enabled: bool) -> Option<&str> {
    enabled
        .then_some(shell)
        .flatten()
        .filter(|shell| !shell.trim().is_empty())
}

fn expand_container_home(ref_: &str, username: &str) -> String {
    ref_.strip_prefix("~/").map_or_else(
        || ref_.to_string(),
        |rest| format!("/home/{username}/{rest}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell_config(sandbox: Option<&str>, project: Option<&str>) -> Config {
        Config {
            sandbox_shell: sandbox.map(str::to_string),
            project_shell: project.map(str::to_string),
            ..Default::default()
        }
    }

    // ── Sandbox layer ─────────────────────────────────────────────────────

    #[test]
    fn sandbox_home_ref_expands_inside_container() {
        let config = shell_config(Some("~/.config/cast/nix#default"), None);
        let cmd = build_command(&config, "alice", "opencode", vec![]);
        assert_eq!(
            cmd,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix#default",
                "-c",
                "opencode",
            ]
        );
    }

    #[test]
    fn sandbox_ref_untouched_by_agent_name() {
        // The agent name survives only as container identity key; it
        // never leaks into the ref.
        let config = shell_config(Some("github:org/repo#shell"), None);
        let cmd = build_command(&config, "alice", "opencode", vec![]);
        assert_eq!(
            cmd,
            vec!["nix", "develop", "github:org/repo#shell", "-c", "opencode"]
        );
    }

    #[test]
    fn sandbox_ref_unset_means_bare_command() {
        let config = shell_config(None, None);
        let cmd = build_command(&config, "alice", "test", vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["test", "arg1"]);
    }

    #[test]
    fn sandbox_switch_off_skips_layer_silently() {
        let config = Config {
            sandbox_shell: Some("~/.config/cast/nix#default".to_string()),
            use_sandbox_shell: false,
            ..Default::default()
        };
        let cmd = build_command(&config, "alice", "test", vec![]);
        assert_eq!(cmd, vec!["test"]);
    }

    // ── Project layer ─────────────────────────────────────────────────────

    #[test]
    fn project_ref_set_wraps_verbatim() {
        let config = shell_config(None, Some(".#ai"));
        let cmd = build_command(&config, "alice", "test", vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["nix", "develop", ".#ai", "-c", "test", "arg1"]);
    }

    #[test]
    fn project_ref_unset_means_no_project_layer() {
        let config = shell_config(Some("/abs/path#shell"), None);
        let cmd = build_command(&config, "alice", "test", vec![]);
        assert_eq!(cmd, vec!["nix", "develop", "/abs/path#shell", "-c", "test"]);
    }

    #[test]
    fn project_switch_off_skips_layer_silently() {
        let config = Config {
            project_shell: Some(".#ai".to_string()),
            use_project_shell: false,
            ..Default::default()
        };
        let cmd = build_command(&config, "alice", "test", vec![]);
        assert_eq!(cmd, vec!["test"]);
    }

    // ── Layer composition ─────────────────────────────────────────────────

    #[test]
    fn both_layers_nest_sandbox_outer_project_inner() {
        let config = shell_config(Some("~/.config/cast/nix#default"), Some(".#ai"));
        let cmd = build_command(&config, "alice", "test", vec!["arg1".to_string()]);
        assert_eq!(
            cmd,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix#default",
                "-c",
                "nix",
                "develop",
                ".#ai",
                "-c",
                "test",
                "arg1",
            ]
        );
    }

    #[test]
    fn one_layer_off_yields_single_wrap() {
        // Sandbox off, project on: only the inner layer wraps.
        let config = Config {
            sandbox_shell: Some("~/.config/cast/nix#default".to_string()),
            project_shell: Some(".#ai".to_string()),
            use_sandbox_shell: false,
            ..Default::default()
        };
        let cmd = build_command(&config, "alice", "test", vec![]);
        assert_eq!(cmd, vec!["nix", "develop", ".#ai", "-c", "test"]);
    }

    #[test]
    fn empty_shell_refs_are_ignored() {
        let config = shell_config(Some("  "), Some(""));
        let cmd = build_command(&config, "alice", "test", vec![]);
        assert_eq!(cmd, vec!["test"]);
    }
}
