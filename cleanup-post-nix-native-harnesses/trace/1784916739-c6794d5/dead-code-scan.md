# Dead Code Scan — Post Nix-Native Harnesses Pivot

Investigation into dead code left behind by the completed
`nix-native-harnesses` pivot (single dev image + Nix devShell provisioning
replacing per-agent Docker images, baked-in binaries, `agent_versions`,
content-hash tags, and the `universal_container` boolean).

Scope scanned: `crates/cast/`, `crates/cast-mcp-client/`, and nix/flake files.
Method: keyword grep + symbol inspection (agent_versions, universal_container,
version_check, auto_update, content_hash, image_tag, harness_version, etc.).
No code was modified — research only.

## 1. High confidence — safe removals (no API change)

### Dead dependency: `ureq`
- File: `crates/cast/Cargo.toml:24`
- `ureq` HTTP client declared but no remaining usages in `src/`. Previously
  used for the removed agent-harness version-checking logic.
- Callers/refs: none. Safe to remove.

### Stale doc comments
- `crates/cast/src/dev/run.rs:61-62` — `RunOpts` doc references
  `Agent::extra_run_args`, which no longer exists (replaced by
  `config_mount_args` / `env_passthrough_args`). Documentation only.
- `crates/cast/src/nix_daemon/daemon.rs:23` — comment says "Get the dynamic
  image tag based on assets hash"; the nix-daemon image tag now uses
  `CAST_VERSION` (0.2.0), not a hash. Comment only.
- `crates/cast/src/dev/agent.rs:17-18` — doc note "agents no longer contribute
  a Dockerfile or a version..." is accurate but framed as a transition note;
  could be simplified to describe current state only.

## 2. High confidence — requires a CLI breaking change

### Per-agent `BuildAgent` variants
- File: `crates/cast/src/commands/cli.rs:184-221` (dispatch at lines 64-99)
- `cast build opencode`, `cast build pi`, `cast build claudecode` are distinct
  subcommands that now all dispatch to identical logic (single shared dev
  image). Could collapse to a single `cast build` command.
- Removal is a CLI breaking change.

### Unused `_agent` parameter
- File: `crates/cast/src/dev/build.rs:16`
- `build_agent` takes `_agent: &dyn Agent`, never used in the body. Removing
  requires updating callers in `cli.rs`.

## 3. Deliberate leftovers (likely keep)

### Backward-compat config tests
- File: `crates/cast/src/config/schema.rs:289-314`
- Tests assert `agent_versions` and `universal_container` keys in `cast.json`
  are silently ignored (serde tolerance for old configs). "Historical marker"
  tests — not truly dead; guard against breaking existing user configs.

## 4. Verified clean

- `crates/cast-mcp-client` — no leftovers; networking is MCP-focused only.
- `flake.nix`, `assets/global-flake-template` — fully updated to nix-native
  provisioning design.
- Agent impls (`opencode`, `pi`, `claudecode` `mod.rs`) — refactored; no
  leftover versioning or image-build logic.

## Suggested next steps

1. Decide scope on the CLI change — collapse `cast build <agent>` into a single
   `cast build` (0.2.x breaking change) or defer.
2. Quick wins (no API change): drop `ureq`; fix the three stale comments;
   simplify the agent trait note.
3. Draft an executive plan + TDD approach for confirmed removals.
