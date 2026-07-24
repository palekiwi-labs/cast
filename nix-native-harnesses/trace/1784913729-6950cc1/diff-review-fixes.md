# Diff Review — feat/universal-container (nix-native-harnesses)

Reviewer: diff-reviewer-opus. Independently verified by consultant-opus
(all six findings confirmed ACCURATE against the real source, line numbers
essentially exact). Branch diff:
`.cue/nix-native-harnesses/tmp/1784891512-f33c89b/branch.diff`.

Verdict: solid, well-tested refactor. Build, clippy, and all 227 unit tests
green. No blocking issues. Two should-fix items and four nits.

## Should-fix

### 1. `cast exec` and `cast run` diverge in mount topology — ACCURATE

Phase 2 made universal mounts unconditional ONLY in `run_agent`.

- `cast run` path: `run_agent` (run.rs:151) inlines
  `build_universal_run_args(all_agents(), ...)` (run.rs:222) → shared
  `{ns}-cache` / `{ns}-local` volumes (volumes.rs:14-23) + the union of every
  agent's `config_mount_args` (volumes.rs:46-48 over `all_agents()`).
- `cast exec` path: `exec` (exec.rs:46) → `run_in_container` (exec.rs:105) →
  `agent.extra_run_args` (run.rs:137) → `build_data_volume_args`
  (opencode/mod.rs:100-109) → per-agent `{ns}-opencode-cache` /
  `{ns}-opencode-local` + only the single launched agent's config mount.
- `cast shell` is fine: `docker exec -it` into a running container
  (shell.rs:34,50), inheriting mounts.

Consequence: `cast run opencode` writes `{ns}-cache`; `cast exec opencode`
reads/writes `{ns}-opencode-cache` — no sharing, and exec omits the config-mount
union. This undercuts the branch's "universal substrate" goal.

Recommendation: converge `exec` onto the universal path, or explicitly log the
intentional difference.

### 2. `resolve_run_opts` scaffolds into real `$HOME` during unit tests — ACCURATE

`scaffold_if_missing` is exemplary (injectable `home_dir: &Path`,
global_flake.rs:21). But `resolve_run_opts` (run.rs:236) sets
`host_home_dir = dirs::home_dir()` (run.rs:242) and calls
`scaffold_if_missing(home)` with the real home (run.rs:247-248), which does a
real `create_dir_all` (global_flake.rs:29) + `write` (global_flake.rs:31) to
`<home>/.config/cast/nix/flake.nix`.

Two unit tests call `resolve_run_opts` directly and do not override home:
- `test_resolve_run_opts_detects_flakes` (run.rs:610)
- `test_resolve_run_opts_populates_host_name` (run.rs:632)

Running the suite therefore writes to `$HOME/.config/cast/nix/flake.nix` —
violating the AGENTS.md nix-build "no `$HOME` side effects" constraint.
Soft-fail (run.rs:257-260 warn only), so no crash, but the FS side effect
occurs.

Recommendation: hoist the scaffold call out of `resolve_run_opts` into
`run_agent` (the only caller that should scaffold), preserving the clean
injectable seam already present at the `global_flake` layer.

## Nits

### 3. Stale doc comment — ACCURATE
run.rs:85-86 doc links `[run_in_container_universal]`, a function that never
existed. Update to reference the real caller (`run_agent` via `dispatch_run`).

### 4. Wrong asset path in flake.nix comment — ACCURATE
Root flake.nix:17 says `crates/cast/assets/global-flake.nix` (nonexistent).
Real path: `crates/cast/assets/global-flake-template/flake.nix` (matches
`templates.global.path` flake.nix:20 and `include_str!` global_flake.rs:9).

### 5. Unnecessary `pub(crate)` — ACCURATE
image.rs:13-14 `IMAGE_BASE` / `CAST_VERSION` are `pub(crate)` but only used
in-module (image.rs:22, 81). `nix_daemon/image.rs:5-7` has its own private
copies. Revert to private `const`.

### 6. `Config::validate()` no-op never wired in — ACCURATE
schema.rs:178-180 returns `Ok(())`; only caller is its own test (schema.rs:327).
`load_config` never invokes it. Either wire into the loader as the validation
seam or drop it.

## Security note

Baking `accept-flake-config = true` + a pinned trusted key into the dev image
is acceptable for the disposable-sandbox threat model (signatures still verified
against the pinned key). Residual risk: any future flake's declared
substituters/keys are auto-trusted non-interactively inside the container.
Suggest a one-line security note in `docs/nix/flake-integration.md` making the
implication explicit.
