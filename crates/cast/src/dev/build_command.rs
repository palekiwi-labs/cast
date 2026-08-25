use crate::config::Config;

/// Build the command vector that will be passed to `docker run` after all flags.
/// Handles the nested Nix develop wrapping logic.
///
/// Each layer (global outer, project inner) wraps the command in
/// `nix develop <ref> -c` iff its ref is set AND its switch is true.
/// Refs are passed verbatim; a disabled layer is skipped silently.
/// Unresolvable refs are not cast's problem: they fail inside the
/// container.
pub fn build_command(config: &Config, base_command: &str, extra_args: Vec<String>) -> Vec<String> {
    let mut capacity = 1 + extra_args.len();
    if global_layer(config).is_some() {
        capacity += 4;
    }
    if project_layer(config).is_some() {
        capacity += 4;
    }

    let mut cmd = Vec::with_capacity(capacity);

    if let Some(ref_) = global_layer(config) {
        cmd.extend([
            "nix".to_string(),
            "develop".to_string(),
            ref_,
            "-c".to_string(),
        ]);
    }

    if let Some(ref_) = project_layer(config) {
        cmd.extend([
            "nix".to_string(),
            "develop".to_string(),
            ref_,
            "-c".to_string(),
        ]);
    }

    cmd.push(base_command.to_string());
    cmd.extend(extra_args);

    cmd
}

/// The effective global layer: the verbatim ref when `global_shell` is
/// set and `use_global_flake` is true, else `None`.
fn global_layer(config: &Config) -> Option<String> {
    if config.use_global_flake {
        config.global_shell.clone()
    } else {
        None
    }
}

/// The effective project layer: the verbatim ref when `project_shell`
/// is set and `use_project_flake` is true, else `None`.
fn project_layer(config: &Config) -> Option<String> {
    if config.use_project_flake {
        config.project_shell.clone()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell_config(global: Option<&str>, project: Option<&str>) -> Config {
        Config {
            global_shell: global.map(str::to_string),
            project_shell: project.map(str::to_string),
            ..Default::default()
        }
    }

    // ── Global layer ──────────────────────────────────────────────────────

    #[test]
    fn global_ref_set_wraps_verbatim() {
        let config = shell_config(Some("~/.config/cast/nix#default"), None);
        let cmd = build_command(&config, "opencode", vec![]);
        assert_eq!(
            cmd,
            vec![
                "nix",
                "develop",
                "~/.config/cast/nix#default",
                "-c",
                "opencode",
            ]
        );
    }

    #[test]
    fn global_ref_untouched_by_agent_name() {
        // The agent name survives only as container identity key; it
        // never leaks into the ref.
        let config = shell_config(Some("github:org/repo#shell"), None);
        let cmd = build_command(&config, "opencode", vec![]);
        assert_eq!(
            cmd,
            vec!["nix", "develop", "github:org/repo#shell", "-c", "opencode"]
        );
    }

    #[test]
    fn global_ref_unset_means_bare_command() {
        let config = shell_config(None, None);
        let cmd = build_command(&config, "test", vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["test", "arg1"]);
    }

    #[test]
    fn global_switch_off_skips_layer_silently() {
        let config = Config {
            global_shell: Some("~/.config/cast/nix#default".to_string()),
            use_global_flake: false,
            ..Default::default()
        };
        let cmd = build_command(&config, "test", vec![]);
        assert_eq!(cmd, vec!["test"]);
    }

    // ── Project layer ─────────────────────────────────────────────────────

    #[test]
    fn project_ref_set_wraps_verbatim() {
        let config = shell_config(None, Some(".#ai"));
        let cmd = build_command(&config, "test", vec!["arg1".to_string()]);
        assert_eq!(cmd, vec!["nix", "develop", ".#ai", "-c", "test", "arg1"]);
    }

    #[test]
    fn project_ref_unset_means_no_project_layer() {
        let config = shell_config(Some("/abs/path#shell"), None);
        let cmd = build_command(&config, "test", vec![]);
        assert_eq!(cmd, vec!["nix", "develop", "/abs/path#shell", "-c", "test"]);
    }

    #[test]
    fn project_switch_off_skips_layer_silently() {
        let config = Config {
            project_shell: Some(".#ai".to_string()),
            use_project_flake: false,
            ..Default::default()
        };
        let cmd = build_command(&config, "test", vec![]);
        assert_eq!(cmd, vec!["test"]);
    }

    // ── Layer composition ─────────────────────────────────────────────────

    #[test]
    fn both_layers_nest_global_outer_project_inner() {
        let config = shell_config(Some("~/.config/cast/nix#default"), Some(".#ai"));
        let cmd = build_command(&config, "test", vec!["arg1".to_string()]);
        assert_eq!(
            cmd,
            vec![
                "nix",
                "develop",
                "~/.config/cast/nix#default",
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
        // Global off, project on: only the inner layer wraps.
        let config = Config {
            global_shell: Some("~/.config/cast/nix#default".to_string()),
            project_shell: Some(".#ai".to_string()),
            use_global_flake: false,
            ..Default::default()
        };
        let cmd = build_command(&config, "test", vec![]);
        assert_eq!(cmd, vec!["nix", "develop", ".#ai", "-c", "test"]);
    }
}
