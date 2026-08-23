# Project Log

## [d580f04] Research: current flake config surface (global_shell / use_flake / use_flake_path)

- **Found:** Config exposes three flake-related fields: global_shell (Option<String>, bare devShell name, default None -> falls back to agent name), use_flake (bool, default false), use_flake_path (Option<String>, verbatim project flake ref passed to nix develop)
- **Found:** Global flake location is HARDCODED: build_command.rs:33 formats /home/{user}/.config/cast/nix#{shell}; no config field points at it
- **Found:** Global layer activation is file-presence gated (~/.config/cast/nix/flake.nix exists, run.rs:330-333), NOT gated by use_flake; use_flake only controls the inner project layer
- **Found:** use_flake:true with no flake.nix and no use_flake_path silently no-ops; use_flake_path set with use_flake:false is silently ignored (both are footguns)
- **Found:** Auto-scaffolding: first cast run/exec writes assets/global-flake-template/flake.nix (shells: default, opencode, pi, claudecode, universal) to ~/.config/cast/nix if absent (scaffold_global_flake in run.rs:243 / exec.rs:107, global_flake.rs)
- **Found:** cast flake init subcommand does NOT exist yet; template is also installable via nix flake init -t github:palekiwi-labs/cast#global (documented)
- **Found:** Env override surface affected by any rename: CAST_GLOBAL_SHELL, CAST_USE_FLAKE, CAST_USE_FLAKE_PATH (figment CAST_ prefix convention)
- **Found:** Global (~/.config/cast/cast.json) and project (./cast.json) configs merge per-field, project wins; either file can technically set any field today
- **Found:** Master trace 1784881352 notes agent name is fused: container identity key AND default devShell selector; global_shell override already makes container name lie about contents
- **Found:** Docs touched by any change: crates/cast/docs/nix/flake-integration.md, nix/overview.md, config/reference.md, config/env-overrides.md, agents.md, concepts.md, getting-started.md
- **Found:** Ref resolution happens INSIDE the container (nix develop argv): refs must be resolvable there - relative refs resolve against the workspace CWD; host absolute paths only work because home is bind-mounted at the same path; remote refs (github:...) need container network
- **Open:** Should global_shell keep accepting bare shell names (resolved against the default global flake path) for backward compat, or clean break at 0.2.0?
- **Open:** Does the project layer keep auto-detection (flake.nix present -> '.') or become fully explicit (project_shell unset = never wrap)?
- **Open:** Field naming for the pair: global_shell/project_shell vs harness_shell/dev_shell vs agent_shell/project_shell
- **Open:** cast flake init semantics: target path flag? auto-write global_shell into cast.json or print snippet only?
- **Open:** Should cast warn when legacy use_flake/use_flake_path keys are present (Config has no deny_unknown_fields, keys would be silently ignored)?
- **Open:** Behavior when global shell unset: bare command will fail inside container (image ships no harness) - upfront cast-side notice or docs-only?

## [d580f04] User decisions on flake config redesign (clean break, kill switches, config init)

- **Decided:** Clean break at 0.2.0: no backward compat, no shims for old global_shell (bare name), use_flake, use_flake_path
- **Decided:** Field pair: global_shell + project_shell, both Option<String> holding full nix flake refs (path/URI with optional #fragment). Unset = layer not loaded; set = wrapped; unresolvable = nix fails inside container (accepted, no cast-side validation)
- **Decided:** use_flake replaced by two kill switches: use_global_flake and use_project_flake, both default true. Exist for temporary env-var disable (CAST_USE_GLOBAL_FLAKE=false etc.), not intended for cast.json files. Effective only when the corresponding ref is set
- **Decided:** Project layer auto-detection (flake.nix present -> '.') is removed: fully explicit via project_shell
- **Decided:** cast config init replaces both auto-scaffolds: provisions global cast.json (numtide cache keys + global_shell pointing at the scaffolded flake's universal shell) and the flake template at ~/.config/cast/nix. Auto-scaffold side effects in run/exec paths are deleted
- **Decided:** No special handling when global_shell is unset: docker exec fails inside container, user will test the failure mode later before deciding if a notice is warranted
- **Decided:** Flake location is user-managed: cast auto-mounts ~/.config/cast so it works as default location; repo flake via .#shell also works since workspace is always mounted; other locations require the user to ensure the mount
- **Open:** cast config init overwrite policy: never overwrite either file; skip-and-notify per file, or hard error if cast.json exists?
- **Open:** Ref written by init for global_shell: tilde form ~/.config/cast/nix#universal (portable across users, tilde expands in-container to bind-mounted home) vs absolute /home/<user>/... form
- **Open:** Should cast config init accept any flags (e.g. --path) or stay flagless v1
- **Open:** Does the 'Loading global nix devshell...' stderr notice get re-gated on config.global_shell.is_some() (trivial, keep) or dropped
- **Open:** Cast config init scope: global-only v1; project-level cast.json scaffolding (with project_shell snippet) deferred?

## [d580f04] Kill switches normalized; cast config init semantics settled

- **Decided:** use_global_flake/use_project_flake are normal config options with no special treatment: valid anywhere config is (cast.json or env), no special docs framing, no extra notices when they disable a layer
- **Decided:** cast config init: partial success policy - writes whichever of cast.json / flake.nix is missing, notifies about skipped existing files, never overwrites
- **Decided:** cast config init writes global_shell using the tilde form: ~/.config/cast/nix#<shell>
- **Decided:** cast config init is flagless in v1 (fixed targets: ~/.config/cast/cast.json + ~/.config/cast/nix)
- **Decided:** cast config init is global-only; no project-level scaffolding
- **Open:** Naming: rename template's universal shell to default requires folding the existing default (base-tooling-only) shell; per-harness shells likely retained for opt-in minimal setups
- **Open:** Fate of agent-name-as-shell-fragment fallback (cast run opencode -> #opencode): drop entirely vs placeholder templating in global_shell

## [d580f04] Explore: agent-fragment fallback is single-site; volumes.rs mount gate discovered

- **Found:** Agent-name-as-fragment fallback lives in exactly ONE site: build_command.rs:32 (config.global_shell.as_deref().unwrap_or(agent_name)). Three CLI paths funnel into it: cast run (run.rs:259 -> agent.rs:45 default trait impl -> build_command), cast exec (exec.rs:113 -> build_exec_cmd:36 -> build_command), cast shell non-raw (shell.rs:45 -> build_command). Single point of change
- **Found:** NEW discovery: dev/universal/volumes.rs:81-87 bind-mounts the global flake into the container, gated on host file presence of ~/.config/cast/nix/flake.nix, mounting ONLY the .config/cast/nix subdir (rw) at the same path — not the whole ~/.config/cast as assumed. Under free-placement flakes this gate/logic needs rework (spec item)
- **Found:** Template ships BOTH a default shell (mkShell "default" [] — baseInputs only, no harnesses, unreachable from cast run under old model since fragment always = agent name or global_shell) and universal (all 3 harnesses). baseInputs = git+ripgrep, shared mkShell helper
- **Found:** Tests pinning the fallback: test_global_shell_defaults_to_agent_name, test_global_shell_override_is_honoured, plus three scenario tests using #test fragment (build_command.rs:102-280)
- **Open:** Rename universal->default requires folding the template's existing empty default shell (it has no cast-reachable purpose today); per-harness shells retained for opt-in minimal setups
- **Open:** Fate of agent-name fallback: recommend Option A (drop entirely; refs verbatim; per-agent selection via CAST_GLOBAL_SHELL env or project config), alternative B ({agent} placeholder templating in global_shell), C (fragment-less implies agent) rejected as ambiguous
- **Open:** Mount rework: drop the flake.nix presence gate and mount ~/.config/cast/nix whenever the dir exists, or widen to ~/.config/cast; custom flake locations = user-managed mounts (docs)

## [d580f04] Final decisions: drop agent-fragment fallback (A), narrow mount scope

- **Decided:** Agent-fragment fallback: Option A — dropped entirely. global_shell/project_shell refs are verbatim, passed to nix develop untouched. Per-agent shell selection via CAST_GLOBAL_SHELL env (per invocation) or project cast.json (per project). Placeholder templating (B) considered, deferred as escape hatch if per-agent config selection is ever needed
- **Decided:** Mount scope: narrow — keep bind-mounting only the ~/.config/cast/nix subdirectory (rw, same path in container), drop the flake.nix presence gate, mount whenever the directory exists. Custom flake locations are user-managed mounts (docs-only)

## [d580f04] Design converged; handed off to build task

- **Decided:** Design task marked complete (user confirmed convergence)
- **Decided:** Build task card created: .cue/master/task/build-repo-defined-shells (kind: build, priority: high, parent: design-repo-defined-shells, refs the spec)

