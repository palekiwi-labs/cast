---
priority: high
tag: 0.2.0
kind: design
refs: task/design-repo-defined-shells.md
---
# Repository-defined agent and project shells

## Problem Statement

`cast` runs agents inside a nested devshell model: an outer "global" shell
providing the agent harness and an inner "project" shell providing the
project environment. Today the global flake's location is hardcoded to
`~/.config/cast/nix`, shell selection within it is a bare name defaulting
to the agent name, and the project layer is governed by the ambiguous
`use_flake`/`use_flake_path` pair with implicit file-presence detection.
There is no way for a repository to provide both shells, which is what
organizations want: git-tracked, team-shareable, nixpkgs-deduplicated.

## Intent

Replace all implicit flake selection with two explicit, symmetric config
fields holding full nix flake references. Flake location becomes fully
user-chosen; `cast` passes refs verbatim to `nix develop` inside the
container. Agent harnesses are treated as ordinary packages the user
provisions themselves, with a single manual bootstrap command replacing
all automatic scaffolding.

## Decisions (agreed with user)

### 1. Config fields

- `sandbox_shell: Option<String>` — full nix flake ref for the outer sandbox
  environment (e.g.
  `~/.config/cast/nix#default`, `.#ai`, `/abs/path#shell`,
  `github:org/repo#shell`). Unset = no sandbox layer. Verbatim pass-through.
- `project_shell: Option<String>` — same ref format. Unset = no project
  layer. Relative refs resolve against the workspace inside the container.
- `use_sandbox_shell: bool` (default `true`) and `use_project_shell: bool`
  (default `true`) — normal config options, no special treatment. A layer
  wraps iff its ref is set AND its switch is true; a disabled layer is
  skipped silently.

### 2. Removals (clean break, shipped in 0.2.0)

- `use_flake`, `use_flake_path` (legacy keys silently ignored by config
  loading; changelog and docs must call the break out loudly).
- Both file-presence probes (`user_flake_present`,
  `project_flake_present`) and their `RunOpts` fields.
- Auto-scaffolding side effects on `cast run`/`cast exec`
  (`scaffold_global_flake`, `scaffold_global_cast_json` call sites).
- Agent-name-as-shell-fragment fallback (`unwrap_or(agent_name)`). The
  agent name survives only as container identity key.
- Hardcoded global flake path formatting in `build_command`.
- The template's `universal` shell name.

Per-invocation sandbox shell selection remains reachable via
`CAST_SANDBOX_SHELL` env
(per invocation) or project `cast.json` (per project). Placeholder
templating in refs was considered and rejected; it is the noted escape
hatch if config-file per-agent selection is ever needed.

### 3. `cast config init`

Flagless, global-only bootstrap. Writes `~/.config/cast/cast.json`
(numtide substituter/key + `sandbox_shell` set to the tilde ref
`~/.config/cast/nix#default`) and `~/.config/cast/nix/flake.nix` from the
embedded template. Never overwrites; partial success — writes whichever
file is missing and notifies about skipped existing ones.

### 4. Template fold

The template's empty `default` shell (base inputs only, never reachable
by `cast`) and `universal` (all harnesses) fold into a new `default` =
base tooling + all three harnesses. A fragment-less ref then lands on the
all-harness shell, matching nix convention. Per-harness shells
(`opencode`, `pi`, `claudecode`) are retained for opt-in minimal setups.

### 5. Mounting

Keep the narrow bind mount of the `~/.config/cast/nix` subdirectory (rw,
same path inside the container). Drop the flake.nix presence gate: mount
whenever the directory exists. Flake locations outside this directory or
the workspace are user-managed mounts (documentation responsibility).

### 6. Failure behavior

- Unresolvable refs: `nix develop` fails inside the container; no
  cast-side pre-validation (resolution is container-side by nature).
- No sandbox shell configured: docker exec failure surfaces as-is; no
  cast-side notice (user will evaluate the raw failure mode later).
- The "loading sandbox nix devshell" stderr announcement re-gates on the
  effective config (ref set AND switch true), preserving the headless
  JSON contract.

## Scope

- Config schema change and env override surface (`CAST_SANDBOX_SHELL`,
  `CAST_PROJECT_SHELL`, `CAST_USE_SANDBOX_SHELL`,
  `CAST_USE_PROJECT_SHELL`; `CAST_USE_FLAKE`/`CAST_USE_FLAKE_PATH` die).
- `cast config init` command (absorbing `global_flake.rs` /
  `global_config.rs` scaffolding logic).
- Template fold and `cast config init` defaults.
- Mount gate change in universal volumes.
- Docs: getting-started (init-first flow), nix/overview,
  nix/flake-integration, config/reference, config/env-overrides,
  agents.md, concepts.md; 0.2.0 changelog entry.

## Out of Scope

- Implementation (downstream build task).
- Project-level scaffolding (project `cast.json` is hand-written).
- Ref validation, placeholder templating, per-agent config selection.
- Any cast-side handling of the missing-harness failure mode.
