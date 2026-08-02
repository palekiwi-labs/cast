use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::run::RunOpts;
use crate::dev::universal::config_dir;
use crate::user::ResolvedUser;

/// Build the shared data volume arguments for the dev container.
///
/// Returns `-v` pairs for `{namespace}-cache` and `{namespace}-local`. A single
/// pair of named volumes is shared by every agent running inside the container.
pub fn build_universal_data_volume_args(config: &Config, user: &ResolvedUser) -> Vec<String> {
    let namespace = &config.volumes_namespace;
    let username = &user.username;
    vec![
        "-v".to_string(),
        format!("{namespace}-cache:/home/{username}/.cache:rw"),
        "-v".to_string(),
        format!("{namespace}-local:/home/{username}/.local:rw"),
    ]
}

/// Build the cross-harness `.agents` bind-mount arguments.
///
/// `.agents` is the vendor-neutral standard (`agentskills.io`) shared across
/// harnesses for global skills (`~/.agents/skills/`). It is mounted
/// unconditionally — regardless of which agents are included — so global
/// skills are available across every agent session.
pub fn build_agents_config_args(opts: &RunOpts) -> Result<Vec<String>> {
    let home = opts
        .host_home_dir
        .as_deref()
        .context("Failed to resolve user home directory")?;
    let agents_host_dir = config_dir::get_config_dir(home);

    // Skip if the workspace root is the same as the .agents dir (the
    // workspace bind mount already covers the target).
    if agents_host_dir == opts.workspace.root {
        return Ok(vec![]);
    }

    Ok(vec![
        "-v".to_string(),
        format!(
            "{}:/home/{}/.agents:rw",
            agents_host_dir.display(),
            opts.user.username
        ),
    ])
}

/// Build the complete set of agent-specific `docker run` arguments for the
/// universal container.
///
/// Composes four layers:
/// 1. Shared data volumes (`-cache` / `-local`) plus the cross-harness
///    `.agents` bind mount (all universal — present for every session).
/// 2. Union of config-directory bind mounts from **every** included agent.
/// 3. Environment-variable passthrough from the **launched** agent only.
/// 4. User flake mount (`~/.config/cast/nix`) if present on the host.
pub fn build_universal_run_args(
    included_agents: &[&dyn Agent],
    launched_agent: &dyn Agent,
    config: &Config,
    opts: &RunOpts,
    env: &HashMap<String, String>,
) -> Result<Vec<String>> {
    let mut args: Vec<String> = vec![];

    // 1. Shared data volumes (cache/local — once) + cross-harness .agents.
    args.extend(build_universal_data_volume_args(config, &opts.user));
    args.extend(build_agents_config_args(opts)?);

    // 2. Union of config mounts for every included agent.
    for agent in included_agents {
        args.extend(agent.config_mount_args(config, opts)?);
    }

    // 3. Env passthrough from the launched agent only (subprocess agents
    //    inherit the container env).
    args.extend(launched_agent.env_passthrough_args(env));

    // 4. User flake mount (~/.config/cast/nix), if present.
    let user_flake_host_dir = opts
        .host_home_dir
        .as_ref()
        .filter(|h| h.join(".config/cast/nix/flake.nix").exists())
        .map(|h| h.join(".config/cast/nix"));
    if let Some(flake_dir) = &user_flake_host_dir {
        args.extend([
            "-v".to_string(),
            format!(
                "{}:/home/{}/.config/cast/nix:rw",
                flake_dir.display(),
                opts.user.username
            ),
        ]);
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::claudecode::ClaudeCode;
    use crate::dev::opencode::OpenCode;
    use crate::dev::pi::Pi;
    use crate::dev::run::TtyMode;
    use crate::dev::workspace::ResolvedWorkspace;
    use std::path::PathBuf;

    fn alice() -> ResolvedUser {
        ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        }
    }

    fn basic_opts() -> RunOpts {
        RunOpts {
            workspace: ResolvedWorkspace {
                container_path: PathBuf::from("/workspace"),
                root: PathBuf::from("/home/alice/project"),
            },
            user: alice(),
            port: 32768,
            host_home_dir: Some(PathBuf::from("/home/alice")),
            host_name: "test-host".to_string(),
            user_flake_present: false,
            project_flake_present: false,
            tty_mode: TtyMode::Interactive,
            publish: false,
        }
    }

    // ── build_agents_config_args ───────────────────────────────────────────

    #[test]
    fn build_agents_config_args_skips_when_workspace_is_agents_dir() {
        // workspace root == host .agents dir → no duplicate mount (the
        // workspace bind mount already covers the target).
        let opts = RunOpts {
            workspace: ResolvedWorkspace {
                container_path: PathBuf::from("/home/alice/.agents"),
                root: PathBuf::from("/home/alice/.agents"),
            },
            ..basic_opts()
        };

        let args = build_agents_config_args(&opts).unwrap();

        assert!(
            args.is_empty(),
            "expected no .agents mount when workspace root is ~/.agents, got: {args:?}"
        );
    }

    // ── build_universal_data_volume_args ───────────────────────────────────

    #[test]
    fn universal_run_args_includes_cross_harness_agents_mount() {
        let config = Config::default();
        let opts = basic_opts();
        let env = HashMap::new();

        let agents: Vec<&dyn Agent> = vec![&OpenCode];
        let args = build_universal_run_args(&agents, &OpenCode, &config, &opts, &env).unwrap();

        // .agents is a universal cross-harness dir; always present regardless
        // of which agents are included.
        assert!(
            args.iter().any(|a| a.contains("/.agents:rw")),
            "cross-harness .agents mount missing: {args:?}"
        );
    }

    #[test]
    fn data_volumes_use_universal_namespace_and_correct_paths() {
        let config = Config::default(); // volumes_namespace = "cast"
        let user = alice();

        let args = build_universal_data_volume_args(&config, &user);

        assert!(
            args.contains(&"cast-cache:/home/alice/.cache:rw".to_string()),
            "expected cache volume, got: {args:?}"
        );
        assert!(
            args.contains(&"cast-local:/home/alice/.local:rw".to_string()),
            "expected local volume, got: {args:?}"
        );
    }

    #[test]
    fn data_volumes_respect_custom_volumes_namespace() {
        let config = Config {
            volumes_namespace: "myteam".to_string(),
            ..Config::default()
        };
        let user = alice();

        let args = build_universal_data_volume_args(&config, &user);

        assert!(args.contains(&"myteam-cache:/home/alice/.cache:rw".to_string()));
        assert!(args.contains(&"myteam-local:/home/alice/.local:rw".to_string()));
    }

    // ── build_universal_run_args ───────────────────────────────────────────

    #[test]
    fn all_three_agents_includes_all_config_dirs_and_single_data_volumes() {
        let config = Config::default();
        let opts = basic_opts();
        let env = HashMap::new();

        let agents: Vec<&dyn Agent> = vec![&OpenCode, &ClaudeCode, &Pi];
        let args = build_universal_run_args(&agents, &OpenCode, &config, &opts, &env).unwrap();

        // All config dirs present (opencode, claude, claude.json, pi).
        assert!(
            args.iter().any(|a| a.contains("/.config/opencode:rw")),
            "opencode config dir missing: {args:?}"
        );
        assert!(
            args.iter().any(|a| a.contains("/.claude:rw")),
            ".claude config dir missing: {args:?}"
        );
        assert!(
            args.iter().any(|a| a.contains("/.claude.json:rw")),
            ".claude.json config file missing: {args:?}"
        );
        assert!(
            args.iter().any(|a| a.contains("/.pi:rw")),
            ".pi config dir missing: {args:?}"
        );

        // Exactly one cache mount.
        let cache_count = args.iter().filter(|a| a.contains("/.cache:rw")).count();
        assert_eq!(
            cache_count, 1,
            "expected exactly one cache mount, got {cache_count}"
        );

        // Exactly one local mount.
        let local_count = args.iter().filter(|a| a.contains("/.local:rw")).count();
        assert_eq!(
            local_count, 1,
            "expected exactly one local mount, got {local_count}"
        );
    }

    #[test]
    fn subset_excludes_excluded_agent_config_dirs() {
        let config = Config::default();
        let opts = basic_opts();
        let env = HashMap::new();

        // Only opencode + pi included — claudecode excluded.
        let agents: Vec<&dyn Agent> = vec![&OpenCode, &Pi];
        let args = build_universal_run_args(&agents, &OpenCode, &config, &opts, &env).unwrap();

        // Included agents' config dirs present.
        assert!(args.iter().any(|a| a.contains("/.config/opencode:rw")));
        assert!(args.iter().any(|a| a.contains("/.pi:rw")));

        // Excluded agent's config dirs absent.
        assert!(
            !args.iter().any(|a| a.contains("/.claude:rw")),
            "claude dir should be absent when claudecode excluded: {args:?}"
        );
        assert!(
            !args.iter().any(|a| a.contains("/.claude.json:rw")),
            "claude.json should be absent when claudecode excluded: {args:?}"
        );
    }

    #[test]
    fn no_two_mounts_share_the_same_container_target() {
        let config = Config::default();
        let opts = basic_opts();
        let env = HashMap::new();

        let agents: Vec<&dyn Agent> = vec![&OpenCode, &ClaudeCode, &Pi];
        let args = build_universal_run_args(&agents, &OpenCode, &config, &opts, &env).unwrap();

        // Extract the container target path from each `-v` arg.
        // A `-v` arg has the shape "host_src:container_target:rw".
        let targets: Vec<&str> = args
            .iter()
            .filter(|a| a.starts_with("-v") || a.contains(":rw"))
            .filter(|a| a.contains(':'))
            .filter_map(|a| {
                // Skip the "-v" flag itself; process the value that follows.
                if a == "-v" {
                    return None;
                }
                // Extract "container_target:rw" portion.
                let parts: Vec<&str> = a.splitn(3, ':').collect();
                if parts.len() == 3 {
                    Some(parts[1])
                } else {
                    None
                }
            })
            .collect();

        let mut seen = std::collections::HashSet::new();
        for target in &targets {
            assert!(seen.insert(target), "duplicate mount target: {target}",);
        }
    }

    #[test]
    fn run_args_use_custom_volumes_namespace() {
        let config = Config {
            volumes_namespace: "myteam".to_string(),
            ..Config::default()
        };
        let opts = basic_opts();
        let env = HashMap::new();

        let agents: Vec<&dyn Agent> = vec![&OpenCode, &Pi];
        let args = build_universal_run_args(&agents, &OpenCode, &config, &opts, &env).unwrap();

        assert!(
            args.iter().any(|a| a.contains("myteam-cache")),
            "custom namespace missing on cache volume: {args:?}"
        );
        assert!(
            args.iter().any(|a| a.contains("myteam-local")),
            "custom namespace missing on local volume: {args:?}"
        );
    }

    #[test]
    fn env_passthrough_from_launched_agent_only() {
        let config = Config::default();
        let opts = basic_opts();

        // ANTHROPIC_API_KEY is in every agent's passthrough set; OPENCODE_*
        // is opencode-only; PI_OFFLINE is pi-only. By launching OpenCode we
        // should see OPENCODE_MODELS_URL but not PI_OFFLINE.
        let mut env = HashMap::new();
        env.insert("ANTHROPIC_API_KEY".to_string(), "sk-123".to_string());
        env.insert(
            "OPENCODE_MODELS_URL".to_string(),
            "http://models".to_string(),
        );
        env.insert("PI_OFFLINE".to_string(), "true".to_string());

        let agents: Vec<&dyn Agent> = vec![&OpenCode, &ClaudeCode, &Pi];
        let args = build_universal_run_args(&agents, &OpenCode, &config, &opts, &env).unwrap();

        // Launched agent's passthrough vars present.
        assert!(
            args.contains(&"ANTHROPIC_API_KEY".to_string()),
            "ANTHROPIC_API_KEY should be present via launched agent passthrough"
        );
        assert!(
            args.contains(&"OPENCODE_MODELS_URL".to_string()),
            "OPENCODE_MODELS_URL should be present (launched agent is opencode)"
        );

        // PI_OFFLINE is pi-specific; absent because env comes from the
        // launched agent (opencode) only.
        assert!(
            !args.contains(&"PI_OFFLINE".to_string()),
            "PI_OFFLINE should be absent (env from launched agent only)"
        );
    }
}
