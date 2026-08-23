---
title: Implement repo-defined agent and project shells (0.2.0)
status: open
priority: high
refs: .cue/design-repo-defined-shells/spec/index.md
kind: build
parent: design-repo-defined-shells
---
Implement the repo-defined shells redesign settled in the design task
(spec: .cue/design-repo-defined-shells/spec/index.md — the decisions
there are final; read it first).

## Problem Statement

The global flake location is hardcoded to `~/.config/cast/nix`, shell
selection is a bare name defaulting to the agent name, and the project
layer is governed by the ambiguous `use_flake`/`use_flake_path` pair
with implicit file-presence detection. Repositories cannot provide both
the agent shell and the project shell.

## Objectives

1. Config schema (`crates/cast/src/config/schema.rs`): add
   `global_shell: Option<String>` and `project_shell: Option<String>`
   (full nix flake refs, verbatim pass-through) plus
   `use_global_flake: bool` / `use_project_flake: bool` (default `true`,
   normal config options). Remove `use_flake`, `use_flake_path`.
   A layer wraps iff its ref is set AND its switch is true; disabled
   layers skip silently.
2. Rewrite `crates/cast/src/dev/build_command.rs`: refs passed verbatim
   to `nix develop <ref> -c`; delete the hardcoded path formatting and
   the agent-name fallback (`unwrap_or(agent_name)`). Update the three
   funneling call paths (`dev/run.rs`, `dev/exec.rs`, `dev/shell.rs`).
3. Remove implicit machinery: both file-presence probes and their
   `RunOpts` fields; `scaffold_global_flake`/`scaffold_global_cast_json`
   call sites in `run.rs`/`exec.rs`.
4. Add `cast config init` (flagless, global-only): writes
   `~/.config/cast/cast.json` (numtide cache keys + `global_shell` =
   `~/.config/cast/nix#default`) and the flake template; never
   overwrites; partial success with notice on skipped files. Absorb
   `dev/global_flake.rs` and `dev/global_config.rs`.
5. Template fold (`crates/cast/assets/global-flake-template/flake.nix`):
   merge empty `default` + `universal` into `default` = base tooling +
   all harnesses; keep per-harness shells; drop the `universal` name.
6. Mount (`dev/universal/volumes.rs:81-87`): keep the narrow
   `~/.config/cast/nix` subdir bind mount, drop the flake.nix presence
   gate — mount whenever the directory exists.
7. Re-gate the "loading global nix devshell" stderr announcement on the
   effective config (ref set AND switch true).
8. Failure behavior: no cast-side ref validation; unresolvable refs and
   missing harnesses surface as container-side errors unchanged.
9. Docs: getting-started (init-first flow), nix/overview,
   nix/flake-integration, config/reference, config/env-overrides
   (CAST_GLOBAL_SHELL, CAST_PROJECT_SHELL, CAST_USE_GLOBAL_FLAKE,
   CAST_USE_PROJECT_FLAKE), agents.md, concepts.md; 0.2.0 changelog
   entry calling out the clean break.

## Constraints

- Clean break at 0.2.0: no back-compat shims; legacy keys are silently
  ignored by config loading (no deny_unknown_fields).
- Tests must conform to nix-build sandbox constraints (temp dirs, no
  network, no $HOME assumptions).
- TDD per AGENTS.md.
