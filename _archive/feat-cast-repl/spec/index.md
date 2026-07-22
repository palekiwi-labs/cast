# Crate: `cast-repl`

---

## Context

AI agents are unable to directly use any interactive terminal tool such
as REPL consoles (`node`, `irb`, `rails-console`, `psql`, etc) or debuggers
like `gdb`, `rust-gdb`, `lldb`, etc.

One solution to this problem is `tmux`. As long as an application runs inside
a `tmux` session, it can be interacted with via sequential commands such as:

1. `tmux send-keys` to execute commands
2. `tmux capture-pane` to receive output

There are two main challenges of a direct use of these commands for the caller:

1. we do not know when the command has fully executed and the output has been fully printed
2. `capture-pane` captures the entire visible portion of the terminal

## Purpose

`cast-repl` is a CLI tool that wraps the above `tmux` commands in order to
provide a predictable, agent-friendly API for interacting with interactive
shells. Output is always JSON — this tool is designed for AI agent use.

### Solutions to the core problems

**Prompt as readiness gate and completion detector:**
The caller must pass `--prompt <regex>` explicitly. This regex is used in
two phases:
1. *Before* sending a command: poll `capture-pane` until the last non-empty
   line matches the prompt — this confirms the REPL is idle and ready.
2. *After* sending a command: poll `capture-pane` until the prompt reappears
   — this confirms the command has fully executed.

Requiring an explicit prompt avoids the ambiguity of auto-detection during
slow program startup: the caller knows the expected prompt pattern and we
simply wait for it.

**Clean capture via pane clear:**
Between steps 1 and 2 above, clear the pane before sending the command:
- Send `C-l` (or equivalent) to clear the visible terminal
- Run `tmux clear-history` to wipe the tmux scrollback buffer

This ensures `capture-pane` after the command sees only the output of that
command, not prior history. Use `capture-pane -S -` to capture the full
(now clean) scrollback so long output is not truncated by terminal height.

---

## Phase 1 — `cast-repl send`

### Usage

```
cast-repl send --prompt <regex> [--timeout <ms>] <session>:<pane> <command>
```

- `<session>:<pane>` follows standard tmux target notation
- `--pane` is optional; omit for the default pane (`<session>:`)
- `--timeout` sets a maximum wait in milliseconds (applies to both the
  readiness poll and the completion poll); defaults to a sensible value TBD

### Execution flow

1. Validate that `<session>:<pane>` exists; return error if not found
2. Poll `capture-pane` until the last non-empty line matches `--prompt`
   (readiness gate); bail on timeout
3. Clear the pane: `C-l` + `tmux clear-history`
4. `tmux send-keys` to send `<command>`
5. Poll `capture-pane -S -` until the last non-empty line matches `--prompt`
   (completion detector); bail on timeout
6. Extract output: everything above the final prompt line
7. Return JSON response

### JSON response

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

### Initial test target

Use `rust-gdb` as the first test program. Its prompt `(gdb) ` is stable and
well-known, making it a good baseline before testing with other REPLs.

---

## Phase 1.5 — `cast-repl kill`

> **TODO:** Design this carefully before implementing.
>
> Killing the tmux session alone does not guarantee the process inside is
> terminated. This is especially relevant for debuggers like `gdb` which may
> be attached to another process and require explicit teardown.
>
> Considerations:
> - **Graceful quit first:** send a program-specific quit command (e.g.
>   `quit` for gdb, `exit` for shells, `\q` for psql) — but this is
>   program-specific and may not generalise cleanly.
> - **Force-kill by PID:** query the foreground process PID via
>   `tmux display-message -p '#{pane_pid}'`, then `kill <pid>` before
>   destroying the session.
> - **Combined approach:** attempt graceful quit, wait briefly, then
>   force-kill if the process is still alive, then destroy the tmux session.
>
> The PID approach is the most program-agnostic and likely the right default.

---

## Phase 2 — `cast-repl start`

Convenience subcommand to spawn a new REPL inside a fresh tmux session and
return the session target for use with `send`. Not required by callers who
already have a running tmux session — `send` works against any existing
session.
