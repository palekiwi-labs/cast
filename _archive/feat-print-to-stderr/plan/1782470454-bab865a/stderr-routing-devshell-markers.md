---
status: complete
---
# Exec Plan: stderr routing + global devshell marker

## Foreword

Addresses task `task/1782470454-bab865a/stderr-routing-devshell-markers.md`.

Context: `cast run --headless --format json` produces a polluted stdout
because `dev/image.rs` and `nix_daemon/daemon.rs` use `println!` for
status messages. These interleave with the agent's JSON output.

Opus consultation confirmed:
- Change `println!` → `eprintln!` in `image.rs` and `daemon.rs` (all
  status lines, not data-output commands like `port.rs`/`config.rs`).
- Each `println!` already has a paired `tracing::info!` (file log);
  `eprintln!` is the correct console-facing complement.
- Add global devshell marker in `run_agent()` just before container
  launch, guarded by `run_opts.user_flake_present`.
- Skip the project devshell marker — the correct pattern is
  shellHook `echo ... >&2` (already done in commit bab865a for
  the project flake; document the pattern for user-managed global flake).
- Do NOT touch `commands/port.rs` or `commands/config.rs`.
- No new abstraction needed.

## Steps

- [x] **Step 1 — `dev/image.rs`: stdout → stderr**
  - Change `println!` → `eprintln!` at the 3 status sites (lines ~35, ~37, ~45).
  - Add a brief comment explaining `info!` (file log) + `eprintln!` (console) split.

- [x] **Step 2 — `nix_daemon/daemon.rs`: stdout → stderr**
  - Change `println!` → `eprintln!` at all 9 status sites (lines ~28,
    ~41, ~60, ~86, ~90, ~100, ~104, ~107).
  - Uniform "status → stderr" rule across the entire codebase.

- [x] **Step 3 — Global devshell marker in `run_agent()`**
  - In `dev/run.rs`, immediately before `match run_opts.tty_mode`
    (the container launch), add:
    ```rust
    if run_opts.user_flake_present {
        info!("loading global nix devshell");
        eprintln!("Loading global nix devshell...");
    }
    ```

- [x] **Step 4 — Verify & lint**
  - `cargo clippy -p cast -- -D warnings`
  - `cargo test -p cast`
  - Confirm `cli_test.rs` tests (port, config show) still pass on stdout.

- [x] **Step 5 — Commit**
  - Commit with message: `feat: route status messages to stderr; add global devshell marker`

- [x] **Step 6 — cue-log**
  - Log the milestone with findings and decisions.
