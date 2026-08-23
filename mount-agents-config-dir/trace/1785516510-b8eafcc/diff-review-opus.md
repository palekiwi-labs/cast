# Code Review: feat/mount-agents-config-dir (diff-reviewer-opus)

Reviewer: diff-reviewer-opus
Branch: `feat/mount-agents-config-dir` vs `master`
Net diff: 6 files, +140 / -3 (Dockerfile.dev, image.rs, run.rs,
universal/config_dir.rs, universal/mod.rs, universal/volumes.rs)
Verification at review time: `cargo test -p cast --lib` -> 235 passed;
`cargo clippy --all-targets -D warnings` -> clean; no NEW fmt diffs.

Overall recommendation: **Request changes** — one required fix, several
cheap recommended fixes. Architecture praised (universal placement,
host-prep ordering, exec/headless/macOS coverage all sound).

## Praise
- Universal placement (step 1 of `build_universal_run_args`) is correct;
  `config_mount_args` of a single agent would be wrong for cross-harness.
- Host-prep ordering safe: `ensure_dev_image` -> `universal::prepare_host`
  (run.rs:159) -> mount construction (run.rs:162) -> `docker run` (run.rs:164).
  Host dir always exists before bind mount; root-autocreate hazard closed.
- `cast exec` routes through same `run_in_container`; `cast shell` uses
  `docker exec` into running container; mount is tty-independent -> headless
  covered. macOS `dirs::home_dir()` + hardcoded container target works.

## Required before merge

### F1 (major) — Duplicate mount destination when workspace is ~/.agents
`build_agents_config_args` (volumes.rs:32-47) unconditionally emits
`-v {home}/.agents:/home/{user}/.agents:rw`. If `cast` runs from inside
`~/.agents`, `map_container_path` maps workspace to the same destination
and `build_docker_run_flags` (run.rs:445-452) emits an identical bind ->
daemon rejects ("duplicate mount point"). OpenCode already guards this
(opencode/mod.rs:40-44). Fix: mirror the guard (~5 lines) + regression
test. Same latent hazard exists for `.claude`/`.pi` today (out of scope).

## Strongly recommended (cheap, same sitting)

### F2 — New tests do not pin behaviour
(a) `universal::prepare_host` has ZERO coverage. Testable without real
`$HOME` since `host_home_dir` is a plain RunOpts field. Add: creates dir
under host_home; errors when host_home is None.
(b) Mount test (volumes.rs:148-151) uses loose substring `/.agents:rw`
that passes with empty/wrong host source. Sibling tests assert exact spec.
Add direct unit test for `build_agents_config_args` asserting exact
`["-v", "{home}/.agents:/home/{user}/.agents:rw"]`; error when None.

### F3 — Dockerfile assertion weak + unusable failure output
image.rs:126-130 asserts `.agents` count `>= 2` (doesn't prove
mkdir-vs-chown split) and dumps the whole 85-line Dockerfile on failure.
Tighten to exact `matches("/home/${USERNAME}/.agents").count() == 2`.

### F5 — Stale doc comments in run.rs
run.rs:118-124 (session mount topology) and run.rs:136-139
(`run_in_container` handles list) were not updated for the new .agents
step. (`build_universal_run_args` doc at volumes.rs:52-57 WAS updated.)

## Needs discussion (decision, not code)

### F11 — Unconditional :rw mount of global instruction dir
`~/.agents/skills/` holds auto-loaded instructions. `:rw` lets an agent or
prompt injection in project A write skills loaded in every future
session/project/harness. Consistent with `.claude`/`.pi` precedent, but:
- Should there be an opt-out (`mount_agents_dir: bool`, default true)?
  Note `forbidden_paths` cannot reach it (resolves relative to workspace).
- Is `rw` required, or would `ro` do (breaks in-sandbox skill install)?

## Discretionary / follow-up

### F4 — Test under wrong section header
volumes.rs:135-152: `universal_run_args_includes_cross_harness_agents_mount`
is under the `build_universal_data_volume_args` banner but exercises
`build_universal_run_args` (its banner is at line 185). Move it.

### F6 — Dockerfile change inert on existing hosts; commit overstates it
`image_tag()` = `localhost/cast:{version}` with no content hash;
`ensure_dev_image` short-circuits on `image_exists`. No version bump ->
existing hosts keep an image without `.agents` until `--force`. Benign
(bind overlay means host UID governs), so the REAL protection is
`prepare_host`, not the mkdir/chown. Commit fb342f2 message overstates.

### F7 — 4th verbatim copy of get_config_dir/ensure_config_dir
universal/config_dir.rs:8-30 is char-for-char copy of pi/config_dir.rs
(same shape in opencode/ and claudecode/). The
`host_home_dir.as_deref().context(...)` idiom now duplicated 5x. Follow-up:
shared `dev/utils.rs` (`ensure_dir`, `host_home`). Do NOT refactor inline.

### F8 — Naming ambiguity
`build_agents_config_args` reads as "config args for the agents" (that's
`config_mount_args`). Prefer `build_dot_agents_mount_args` /
`build_agents_dir_mount_args`. Weakly: module `universal/config_dir.rs`
-> `universal/agents_dir.rs` (cast's own `~/.config/cast` is a different
"config dir" in this codebase).

### F9 — tempfile API inconsistency
config_dir.rs:46 uses `tempfile::tempdir()`; rest of crate uses
`tempfile::TempDir::new()`.

### F10 — No user-facing docs for host-mutating default
cast now creates `~/.agents` and bind-mounts it rw into every sandbox for
every user. `docs/concepts.md:41-46` implies host access is opt-in via
`extra_data_volumes`, now untrue. Add a note there / in agents.md.

### F12 — Commit hygiene
b8eafcc only reverts cd470a8 formatter noise. Fine if squash-merging.

## Answers to my questions (summary)
- Universal placement right? Yes.
- Bind mount fail when ~/.agents missing? No (prepare_host precedes).
- If ~/.agents is a file -> clear error; if symlink -> works.
- Ownership concerns? None; Dockerfile mkdir/chown defensive only.
- prepare_host order correct? Yes.
- Pattern consistency? Close; diverges only in missing workspace-collision
  guard (F1). Duplication pattern-conformant (F7).
- Test gaps? All real: None-path untested for both fns; prepare_host
  untested; Dockerfile chown assert weak; mount assert too loose;
  duplicate-target test can't see cross-function collision (F1).
- cast exec / headless / macOS? All covered.
