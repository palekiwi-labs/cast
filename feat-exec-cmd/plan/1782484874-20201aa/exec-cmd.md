---
status: open
---
# Plan: cast exec command

Ref task: `.cue/master/task/1782484874-20201aa/exec-cmd.md`
Branch: `feat/exec-cmd`

## Overview

Three loosely-coupled change groups. Implement in order — each group is
independently committable and keeps the build green.

---

## Group 1 — Port publishing redesign

**Goal:** Remove `publish_port: bool` config default; replace with
`--publish`/`-p` opt-in CLI flag on `cast run`.

### 1.1 — Config schema
- Remove `publish_port: bool` from `Config` struct (`config/schema.rs`).
- Remove it from `Config::default()`.
- Remove `config.publish_port` gate from `build_docker_run_flags` in `run.rs`.

### 1.2 — `RunOpts` and `build_docker_run_flags`
- Replace `opts.publish: bool` with `opts.publish: Option<PublishPort>` in `RunOpts`.
  ```rust
  pub enum PublishPort { Auto, Fixed(u16) }
  ```
- Update `build_docker_run_flags` to match on `opts.publish`:
  - `None` → no `-p` flag
  - `Some(Auto)` → `-p {port}:80`
  - `Some(Fixed(n))` → `-p {n}:80`
- Remove the TTY-mode coupling (`publish: matches!(Interactive)`); `--publish`
  now owns the decision independently of headless/interactive.

### 1.3 — CLI flag on `cast run`
- Add `--publish`/`-p` with `num_args = 0..=1` and `default_missing_value`
  to `RunFlags`.
- Wire it into `session_flags` / `RunOpts` in `cli.rs`.
- Update `resolve_run_opts` signature if needed.

### 1.4 — Tests
- Update existing publish tests in `run.rs` (currently gated on
  `publish_port: true`).
- Add new unit tests for `Auto` and `Fixed` publish variants.
- Verify `cast run opencode` produces no `-p` flag (new default).

**Commit after tests pass.**

---

## Group 2 — Container naming unification

**Goal:** One stable name (interactive `run`); everything else token-suffixed.
Drop `-headless-` literal.

### 2.1 — `resolve_container_name`
- One-line change: drop `-headless-` from the headless format string.
  ```rust
  // before
  format!("cast-{}-{}-{}-headless-{}", agent_name, cwd_basename, port, tok)
  // after
  format!("cast-{}-{}-{}-{}", agent_name, cwd_basename, port, tok)
  ```
- Signature unchanged (`token: Option<&str>` stays).

### 2.2 — Caller wiring in `cli.rs`
- Headless `cast run`: token = `invocation_id` (unchanged behaviour, new format).
- Interactive `cast run`: token = `None` (unchanged).

### 2.3 — Tests
- Update expected strings in `container_name.rs` tests.
- Add `starts_with` invariant test: any token-suffixed name starts with
  the interactive stable name.

**Commit after tests pass.**

---

## Group 3 — `cast exec` implementation

**Goal:** New subcommand end-to-end.

### 3.1 — `ExecAgent` enum (cli.rs)
- Mirror `RunAgent` but with `cmd: Vec<String>` as `trailing_var_arg`,
  `num_args = 1..` (required), and `disable_help_flag = true`.
- Same aliases (`o`, `p`, `c`) as `RunAgent`.
- Add `as_agent()` impl.

### 3.2 — `ExecFlags` struct (cli.rs)
- Fields: `headless: bool`, `name: Option<String>`,
  `publish: Option<PublishPort>`, `raw: bool`.
- Add `Commands::Exec` variant.

### 3.3 — `dev/exec.rs` (new file)
Mirrors `run.rs` / `run_agent()` with these differences:

| Aspect | `run_agent` | `exec` |
|---|---|---|
| Command | `agent.build_command()` | user-supplied `cmd: Vec<String>` |
| Nix wrap | always | skipped when `raw = true` |
| Port publish | from `RunOpts` | from `ExecFlags` |
| `publish` default | opt-in (Group 1) | opt-in (same) |
| Tracing span | `agent_session` | `exec_session` |

Key implementation notes:
- Call `nix_daemon::ensure_running()` unconditionally (even `--raw` mounts `/nix`).
- Call `agent.prepare_host()` unconditionally.
- Call `agent.ensure_image()`.
- Set `run_opts.publish` from `ExecFlags.publish` (do **not** derive from
  `RunMode`).
- Token for `resolve_container_name`:
  - interactive exec → `Some(format!("exec-{invocation_id}"))`
  - headless exec → `Some(invocation_id)`
- `--raw`: pass `cmd` directly; skip `build_command::build_command()`.
- Non-raw: `build_command::build_command(config, &run_opts, &cmd[0], cmd[1..].to_vec())`.

### 3.4 — Wire into `cli.rs` dispatch
- Build `SessionFlags` from `ExecFlags` exactly as the `Run` arm does
  (including token from `invocation_id`).
- Call `dev::exec(agent, &approved, session_flags, raw, publish, cmd)`.

### 3.5 — `dev/mod.rs`
- `pub mod exec;`
- `pub use exec::exec;`

### 3.6 — Tests
- Clap parsing: `cast exec opencode` (no cmd) → error.
- Clap parsing: `cast exec --raw opencode /bin/bash -c "x"` → `raw=true`,
  `cmd=["/bin/bash", "-c", "x"]`.
- Clap parsing: flags must precede subcommand.
- `exec` with `--raw`: cmd vector is not Nix-wrapped.
- `exec` without `--raw`: cmd is Nix-wrapped via `build_command`.
- `exec` interactive token: name contains `exec-`.
- `exec` headless token: name does not contain `exec-`.

**Commit after tests pass.**

---

## Group 4 — Docs

- Update `crates/cast/docs/commands/reference.md`:
  - Add `cast exec` entry.
  - Update `cast run` entry (note `--publish` replaces old default).

**Commit.**

---

## Ordering constraint

```
Group 1 → Group 2 → Group 3 → Group 4
```

Group 3 depends on the `PublishPort` type from Group 1 and the naming
behaviour from Group 2.
