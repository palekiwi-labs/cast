# Plan: `cast-repl`

## Phases

### Phase 0 — Dev Environment

- [ ] Add `pkgs.tmux` to `devShells.default` in `flake.nix`
- [ ] Add `pkgs.gdb` to `devShells.default` in `flake.nix`
  - `rust-gdb` is a wrapper script bundled with the Rust toolchain (fenix provides it)
  - `gdb` itself must be available separately
- [ ] Verify `rust-gdb` is callable inside the devshell
- [ ] Verify `tmux` is callable inside the devshell

---

### Phase 1 — `cast-repl send`

#### 1.1 — Scaffold crate

- [ ] Add `crates/cast-repl/` to the workspace (`Cargo.toml`)
- [ ] Add `cast-repl` package to `flake.nix`
- [ ] Add `clap` dependency for CLI parsing
- [ ] Add `serde` + `serde_json` for JSON output

#### 1.2 — CLI interface

```
cast-repl send --prompt <regex> [--timeout <ms>] <session>:<pane> <command>
```

- `--prompt <regex>` — required; matched against last non-empty line of pane
- `--timeout <ms>` — optional; default 30000 (30s); applies to both polls
- `<session>:<pane>` — tmux target in standard notation; pane portion is optional
- `<command>` — the command string to send to the REPL

#### 1.3 — Execution flow

1. Validate that `<session>:<pane>` exists; return error JSON if not found
2. **Readiness gate:** poll `tmux capture-pane` until the last non-empty line
   matches `--prompt`; bail with `timeout` error if exceeded
3. Clear the pane:
   - `tmux send-keys -t <target> 'C-l'` (clear visible terminal)
   - `tmux clear-history -t <target>` (wipe tmux scrollback)
4. `tmux send-keys -t <target> '<command>' Enter`
5. **Completion detector:** poll `tmux capture-pane -S - -t <target>` until
   the last non-empty line matches `--prompt`; bail with `timeout` error if exceeded
6. Extract output: everything above the final prompt line
7. Return success JSON

#### 1.4 — JSON response schema

**Success:**
```json
{
  "ok": true,
  "output": "...",
  "prompt": "...",
  "elapsed_ms": 123
}
```

**Error:**
```json
{
  "ok": false,
  "error": "session_not_found | pane_not_found | timeout | tmux_error",
  "message": "human-readable detail",
  "elapsed_ms": 123
}
```

#### 1.5 — Unit tests (no tmux required)

- Prompt regex matching logic
- Output extraction from raw `capture-pane` strings (pure string parsing)
- JSON serialisation of success and error responses
- Run with plain `cargo test`

#### 1.6 — Integration tests (tmux + rust-gdb)

- Gate behind `#[ignore]`; run explicitly with `cargo test -- --ignored`
- Use an isolated tmux socket per test run via `tmux -L cast-repl-test-<uuid>`
  to avoid collisions with the user's own tmux sessions
- Test setup: spawn detached tmux session, start `rust-gdb` inside it
- Test teardown: kill socket / session unconditionally
- First test: send a simple gdb expression (e.g. `print 1 + 1`) and assert
  the captured output and `ok: true` in the JSON response
- Prompt pattern for rust-gdb: `\(gdb\)\s*$`

---

### Phase 1.5 — `cast-repl kill`

> **TODO:** Design carefully before implementing.
>
> Killing the tmux session alone does not guarantee the process inside is
> terminated. Especially relevant for debuggers like `gdb` which may be
> attached to another process.
>
> Candidate approach (most program-agnostic):
> 1. Query foreground process PID: `tmux display-message -p '#{pane_pid}'`
> 2. Attempt graceful quit: send program-specific quit command (e.g. `quit`
>    for gdb) — wait briefly for prompt to disappear
> 3. Force-kill if still alive: `kill <pid>`
> 4. Destroy tmux session
>
> Open questions:
> - How to handle processes that ignore SIGTERM?
> - Should there be a `--force` flag to skip the graceful step?
> - Should kill accept a `--quit-cmd` override for the graceful step?

---

### Phase 2 — `cast-repl start`

Convenience subcommand: spawn a new REPL in a fresh tmux session and return
the session target for use with `send`.

Not required by callers who already have a running tmux session — `send`
works against any existing session.

Design TBD.
