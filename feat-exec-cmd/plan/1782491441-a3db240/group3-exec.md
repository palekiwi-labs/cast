---
status: complete
---
# Executive Plan: Group 3 — cast exec implementation

Ref task: `.cue/master/task/1782484874-20201aa/exec-cmd.md`
Ref master plan: `.cue/feat-exec-cmd/plan/1782484874-20201aa/exec-cmd.md` (Group 3)
Branch: `feat/exec-cmd`

## Foreword

Group 1 (port publishing redesign) and Group 2 (container naming unification)
are complete. This plan covers Group 3: the `cast exec` subcommand end-to-end.

Key design constraints from the master plan:
- `cast exec <agent> <cmd> [args...]` starts a **fresh container** (docker run --rm),
  never `docker exec` into a running one.
- `--raw` skips Nix devshell wrapping; nix_daemon::ensure_running is called either way.
- Token for container naming:
  - interactive exec → `Some(format!("exec-{invocation_id}"))`
  - headless exec → `Some(invocation_id)`
- `ExecAgent` mirrors `RunAgent` but carries `cmd: Vec<String>` with `num_args = 1..`
  (required) and `disable_help_flag = true`.
- Same agent aliases (`o`, `p`, `c`) as `RunAgent`.
- `dev/exec.rs` orchestrates the session; `cli.rs` dispatches to it.

Implementation order follows TDD: tests first, then implementation per step.

## Steps

- [x] 3.1 — Write clap-parsing tests for `ExecAgent` and `ExecFlags` (RED)
- [x] 3.2 — Add `ExecAgent` enum and `ExecFlags` struct to `cli.rs`; add
            `Commands::Exec` variant (GREEN for parsing tests)
- [x] 3.3 — Write unit tests for `dev/exec.rs` (raw vs non-raw cmd building) (RED)
- [x] 3.4 — Implement `dev/exec.rs` (GREEN for exec unit tests)
- [x] 3.5 — Wire `Commands::Exec` dispatch in `cli.rs`; expose via `dev/mod.rs`
- [x] 3.6 — Run full test suite + clippy; commit
