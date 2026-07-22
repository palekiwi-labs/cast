# Code Review: feat/exec-cmd (Opus)

**Reviewer:** consultant-opus
**Date:** 2026-06-27
**Scope:** 3 commits — `--publish` flag refactor, container-name `-headless-` removal, `cast exec` subcommand.

**Overall verdict:** Solid, well-tested work. Handful of correctness gaps worth fixing before merge.

---

## Must Fix

### [M1] Container-name collision via `--name` and concurrent execs
**Files:** `exec.rs:65`, `cli.rs:143-147`

The "never collide" invariant only holds for the auto-generated path. Two leaks:

1. `--name` is honored verbatim by `resolve_container_name`. If a user runs
   `cast run --name foo opencode` then `cast exec --name foo opencode ls`,
   the `docker run --rm` will fail (name conflict). Either reject `--name`
   collisions or document that `--name` opts out of the collision guarantee.
2. Confirm `generate_invocation_id()` is collision-resistant (likely fine but
   worth a code comment).

### [M2] `num_args = 1..` is not enforced at parse time
**Files:** `cli.rs:306/313/320`, `exec.rs:51`

`test_exec_requires_cmd_arg` proves `cast exec opencode` (no cmd) parses
successfully with an empty vec. The `num_args = 1..` annotation is misleading.
Empty-cmd is caught at runtime by `exec.rs:51`'s `bail!`, and `build_exec_cmd`
also guards it — three layers of defense, none at clap level. Either:
- Drop `num_args = 1..` and document the runtime check as authoritative, or
- Restructure so the empty case is a clap-level error (better UX).

---

## Should Fix

### [S1] `--raw` help text oversells "rawness"
**File:** `cli.rs ~286`

`--raw` still forces the Nix daemon up and mounts `/nix`. Help text says
"pass the command directly to docker run" but user pays full daemon-startup
and image-ensure cost. Clarify: `--raw` only skips *command wrapping*, not
the `/nix` mount or daemon.

### [S2] `build_exec_cmd` returning empty vec is a latent hazard
**File:** `exec.rs:27-29`

With `raw=true` and an empty cmd, `build_exec_cmd` returns `vec![]`. Guarded
in practice by the bail at line 51, but as a `pub` function it's callable with
empty input. Document the non-empty precondition or make the function take a
non-empty-guaranteed slice.

### [S3] `exec()` is a near-verbatim copy of `run_agent()` — MOST IMPACTFUL
**Files:** `exec.rs:43-127` vs `run.rs:101-190`

~90% of both function bodies are identical. The only differences: the command
builder, the token source, and the `user_flake_present` announcement (exec
silently drops it — intentional?). Extract:

```rust
fn run_in_container(agent, config, run_opts, container_name, cmd) -> Result<ExitStatus>
```

Both `run_agent` and `exec` build inputs and delegate. Prevents drift.

### [S4] Token/naming responsibility split between `cli.rs` and `exec.rs`
**Files:** `cli.rs:139-147`, `exec.rs`

`cli.rs` computes `name_token` (the `exec-` prefix logic) while `run_agent`
derives its token from `RunMode` internally. The same concept is handled via
two different mechanisms. Consider folding into `RunMode` or a naming helper.

### [S5] `PublishPort::FromStr` in `run.rs` — layering note
**File:** `run.rs:38-52`

CLI-parsing concern (`FromStr` for clap) lives in the orchestration module.
Not blocking, but couples layers. Consider moving to `cli.rs` or a `types` module.

### [S6] `Fixed(0)` accepted but semantically odd
**File:** `run.rs:44-49`

`--publish 0` parses to `Fixed(0)` → `-p 0:80`. Docker interprets host port 0
as "random ephemeral port" — probably not what the user intends. Either reject
or document.

### [S7] Bare `--publish` is fragile with the subcommand structure — IMPORTANT
**Files:** `cli.rs:30`, `cli.rs:~286`

Test comment admits: "Use explicit `=auto` to avoid clap consuming the agent
subcommand name as the port value." So `cast exec --publish opencode ls` will
try to parse `opencode` as a port. The bare `--publish` form is effectively
unusable without `=`. Options:
- Require `=` syntax for the value: `--publish=PORT`
- Use separate flags: `--publish-auto` vs `--publish-port N`
- At minimum, document in help: bare `--publish` must precede agent or use `=`.

### [S9] No test for the empty-cmd bail
**File:** `exec.rs:51`

Only enforcement of "exec requires a command" is untested. Add a test asserting
`exec(..., cmd=vec![])` returns the expected error.

### [S10] No test for `PublishPort::FromStr` error paths
**File:** `run.rs:38-52`

Add unit tests: `"auto"`→Auto, `"8080"`→Fixed(8080), `"99999"`→Err,
`"abc"`→Err, `""`→Err.

### [S11] No integration-level test for exec vs run name collision avoidance
No test asserts that exec's auto-generated name differs from run's interactive
name for the same CWD/agent. A unit test constructing both names and asserting
inequality would lock the invariant.

---

## Nits

- **[N1]** `ExecAgent` and `RunAgent` are parallel enums with identical boilerplate.
  Leave as-is for now; revisit if a 4th agent lands.
- **[N2]** `bail!` usage string in `exec.rs:52-55` is correct; minor cleanup only.
- **[N3]** `test_build_exec_cmd_non_raw_splits_cmd_args` is a near-duplicate of
  `test_build_exec_cmd_non_raw_no_flake_is_bare` — no flake active means no
  splitting occurs and nothing is proven. Use a flake config or rename/remove.
- **[N4]** `--publish` doc comment duplicated on `RunFlags` and `ExecFlags`.
- **[N6]** `container_name.rs:7` doc still says "headless" in parenthetical;
  update to "ephemeral" to match the new reality.
- **[N7]** External tooling grepping `-headless-` in `docker ps` will break.
  Consider CHANGELOG/migration note if any external consumers exist.
- **[N8] BEHAVIOR CHANGE:** `cast run` previously published by default
  (`publish_port: true`); now no port is published unless `-p` is given. This
  is intended (commit 1 rationale) but is user-visible and warrants a release note.

---

## Summary Table

| Severity | Count | Top items |
|---|---|---|
| Must fix | 2 | M1 name-collision, M2 misleading `num_args` |
| Should fix | 9 | S3 run/exec duplication, S7 bare `--publish` fragility, S9-S11 test gaps |
| Nit | 8 | Doc/label cleanups, N8 behavior-change release note |

**Top 3 before merge:** S3 (extract shared core), M1 (close `--name` collision hole), S7 (`--publish` bare-form ergonomics).
