---
status: complete
---
# Executive Plan: Post-Review Fixes — feat/run-headless

## Foreword

This plan addresses findings from the Sonnet code review (confirmed/extended by
Opus consultation) of the `feat/run-headless` implementation. It is scoped to
the items the user approved: signal handling, SessionFlags type safety, container
naming format, token width, a trivial test cleanup, and the `build_run_opts`
rename. It does not address: `--name` validation, `resolve_tty_mode` refactor,
`pub` surface tightening, or doc-only nits (all deferred).

Current branch: `feat/run-headless`. All work is on this branch.

---

## Items

### Item 1 — Widen `generate_invocation_id` token to 64 bits

**File:** `crates/cast/src/logging.rs`

**Problem:** The current generator truncates a 64-bit hash to `u32` then formats
as 8 hex chars. The format change alone would not help — the truncation must be
fixed. The token is also used as the log span ID, and 16 chars is still compact
enough for log scanning.

**Change:**
```rust
// Before:
format!("{:08x}", hash as u32)
// After:
format!("{:016x}", hash)    // full u64, no truncation
```

No callers other than `cli.rs` (log span and headless token). Log span IDs get
longer; that is acceptable.

**Tests:** No unit tests exist for `generate_invocation_id` (it's random). The
change is trivially correct — just remove the `as u32` cast and widen the format.

---

### Item 2 — Adopt `RunMode` enum in `SessionFlags`

**Files:** `crates/cast/src/dev/run.rs`, `crates/cast/src/commands/cli.rs`,
`crates/cast/src/dev/mod.rs`

**Problem:** `headless: bool` and `token: Option<String>` in `SessionFlags` can
drift — the invariant (token is Some iff headless) is enforced only at the
call site. `RunMode` makes illegal states unrepresentable.

**New type (in `dev/run.rs`):**
```rust
pub enum RunMode {
    Interactive,
    Headless { token: String },
}

pub struct SessionFlags {
    pub mode: RunMode,
    pub name: Option<String>,
}
```

**Downstream changes:**
- `resolve_run_opts`: derive `tty_mode` and `publish` from `flags.mode`:
  ```rust
  tty_mode: match &flags.mode {
      RunMode::Headless { .. } => TtyMode::Headless,
      RunMode::Interactive   => TtyMode::Interactive,
  },
  publish: matches!(flags.mode, RunMode::Interactive),
  ```
- `run_agent`: extract token from `flags.mode` for `resolve_container_name`:
  ```rust
  let token = match &flags.mode {
      RunMode::Headless { token } => Some(token.as_str()),
      RunMode::Interactive        => None,
  };
  ```
- `cli.rs`: construct `SessionFlags` with `RunMode`:
  ```rust
  let mode = if flags.headless {
      RunMode::Headless { token: invocation_id.clone() }
  } else {
      RunMode::Interactive
  };
  let session_flags = SessionFlags { mode, name: flags.name.clone() };
  ```
- `dev/mod.rs`: re-export `RunMode` alongside `SessionFlags`.
- Remove `resolve_tty_mode` if it becomes dead code (it currently has tests —
  convert it to `From<&RunMode> for TtyMode` or keep for the tests and mark
  as `pub(crate)`). Preferred: keep `resolve_tty_mode` tests but update the
  function to accept `&RunMode`, or inline into `resolve_run_opts`.

**Test sites** (3 `SessionFlags` constructions to update):
- `cli.rs:103` — main construction site (above)
- `run.rs` test fixtures: `make_interactive_opts` / `make_headless_opts` do not
  construct `SessionFlags` directly, only `RunOpts` — no changes needed there.
- `run.rs:463`: `SessionFlags { headless: false, ... }` in
  `test_resolve_run_opts_detects_flakes` — update to `mode: RunMode::Interactive`.
- Any shell.rs site if present (needs grep).

---

### Item 3 — Update headless container name format

**File:** `crates/cast/src/dev/container_name.rs`

**Problem:** The headless name currently excludes the port, so you cannot filter
`docker ps` with the same prefix as the interactive container. The port is the
stable per-project/agent identity and should appear in both names.

**New format (Option 1 from Opus):**
```
cast-{agent}-{basename}-{port}-headless-{token}
```

This is a strict suffix-extension of the interactive default
(`cast-{agent}-{basename}-{port}`), so one `docker ps --filter name=cast-{agent}-{basename}-{port}`
matches both interactive and all headless containers for that project.

**Decision on `config.container_name` override in headless:**
Currently the headless path ignores `config.container_name`. Opus flagged this
as an inconsistency. We will **preserve the current behaviour** (headless ignores
the override) because:
- The test `test_headless_token_overrides_config_name` already documents it
- The override is a user-managed stable name, unsuitable for ephemeral headless
- The user can use `--name` if they want an explicit name

**Change to `resolve_container_name`:**
```rust
// Headless path: unique ephemeral name with injected token.
if let Some(tok) = token {
    return format!("cast-{}-{}-{}-headless-{}", agent_name, cwd_basename, port, tok);
}
```

**Test updates** (format change, same logic):
- `test_headless_with_token`: update expected string to include port
- `test_headless_token_overrides_config_name`: update expected string to include port
- Update doc comment at top of function

---

### Item 4 — Fix `headless_command` signal handling (orphan prevention)

**File:** `crates/cast/src/docker/client.rs`

**Problem:** The current `SignalGuard` ignores SIGINT/SIGQUIT in `cast` but does
not handle SIGTERM (the systemd stop signal). When `cast` is killed, the Docker
container is orphaned because `docker stop <name>` is never called. The
`--rm` flag only fires when `docker run` exits normally.

**Approach (Opus recommendation):** Replace `SignalGuard` in `headless_command`
with a `HeadlessSignalGuard` that:
1. Sets a static `AtomicBool` flag via an async-signal-safe handler for
   SIGINT, SIGTERM, and SIGQUIT
2. The `headless_command` loop polls `child.try_wait()` in a 100ms loop
3. On flag set, calls `docker stop --time 10 <container_name>` from normal
   code (not the handler — spawning a process from a signal handler is UB)
4. After stop, continues the loop until `docker run` exits and is reaped

**Required signature change:** `headless_command` must receive the container
name so it can issue `docker stop <name>`.

```rust
pub fn headless_command(
    &self,
    args: Vec<String>,
    container_name: &str,
) -> Result<ExitStatus>
```

**Call site change** in `dev/run.rs`:
```rust
TtyMode::Headless => docker.headless_command(docker_args, &container_name)?,
```

**Key implementation shape:**
```rust
static HEADLESS_SHUTDOWN: AtomicBool = AtomicBool::new(false);

extern "C" fn headless_signal_handler(_sig: libc::c_int) {
    HEADLESS_SHUTDOWN.store(true, Ordering::SeqCst);
}

struct HeadlessSignalGuard { old_int, old_term, old_quit }
// installs headless_signal_handler for SIGINT, SIGTERM, SIGQUIT
// Drop restores all three
```

**Do NOT** reset child signals to `SIG_DFL` via `pre_exec` in headless — cast
is the supervisor; signals should come to cast so it can run the orderly stop.

**Testability:** The signal delivery path cannot be unit-tested in the Nix
sandbox (no docker, sandboxed). The poll loop and `docker stop` dispatch can be
tested via a `Reaper` trait or injectable closure, but this is optional polish.
For now, verify by a manual smoke test (systemd-timer or `kill -TERM <pid>`).

**Note:** `interactive_command` is unchanged — it retains its original
`SignalGuard` (SIGINT/SIGQUIT ignore, `SIG_DFL` in child) which is correct for
the shared-process-group / foreground-TTY scenario.

---

### Item 5 — Remove misleading test

**File:** `crates/cast/tests/cli_test.rs`

**Problem:** `test_cast_run_headless_after_agent_is_passthrough` (line 233) does
not test what its name claims. It asserts `--help` output contains `--headless`,
which is already covered by `test_cast_run_headless_flag_in_help`. The test body
even acknowledges the original intent was different.

**Change:** Delete the function `test_cast_run_headless_after_agent_is_passthrough`
(lines 232–242). No other changes needed; the coverage it claimed to provide
is already exercised by adjacent tests.

---

### Item 6 — Rename `build_run_opts` → `build_docker_run_flags`

**File:** `crates/cast/src/dev/run.rs`

**Problem:** `resolve_run_opts` (builds a `RunOpts` struct) and `build_run_opts`
(builds a `Vec<String>` of Docker CLI flags) have confusingly similar names in
the same module.

**Change:** Rename the function and all call sites:
```rust
// Before:
pub fn build_run_opts(config: &Config, opts: &RunOpts) -> Vec<String>
// After:
pub fn build_docker_run_flags(config: &Config, opts: &RunOpts) -> Vec<String>
```

Call sites in `run.rs` (`run_agent` uses it as `build_run_opts`). Also update
all test function names that reference `build_run_opts` → `build_docker_run_flags`.

---

## Execution order

These items are partially independent but share file touches. Recommended order:

1. **Item 5** (remove test) — smallest, zero risk, no dependencies
2. **Item 1** (widen token) — one-line change, no dependencies
3. **Item 2** (RunMode enum) — structural; do before Item 3 since Item 3
   changes `resolve_container_name` signature which is called with the token
   extracted from `RunMode`
4. **Item 3** (naming format) — depends on Item 2 (token now from RunMode)
5. **Item 6** (rename) — pure rename, independent but easiest after Item 2's
   churn settles
6. **Item 4** (signal handling) — largest change, depends on Items 2–3
   (container name and RunMode must be stable before wiring into headless_command)

Commit after each item reaching GREEN. Do not batch items into one commit.

## Testing

After all items: `cargo test -p cast` must show 246+ tests passing (the removed
test in Item 5 reduces the count by 1 → expect 245+ passing before Item 4 adds
any new tests).
