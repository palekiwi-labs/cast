---
status: complete
---
# Executive Plan: feat/run-headless Implementation

## Foreword

This plan covers the full implementation of `feat/run-headless` across all 6
phases defined in the master plan (`plan/index.md`). The feature adds a
`--headless` flag to `cast run` enabling non-interactive Docker execution for
CI/systemd environments, without affecting existing interactive behavior.

Key constraints:
- All tests must pass under `nix build` (sandboxed, no network, no `$HOME`)
- Existing interactive behavior must be 100% unaffected
- `cast port` (which reuses `RunAgent`) must be unaffected
- No `uuid` crate (MCP-feature-gated)
- Token injected by caller from `generate_invocation_id()`

## Steps

### Phase 1 — Pure flag resolution (`dev/run.rs`)
- [x] Add `TtyMode` enum (`Interactive`, `Headless`) to `dev/run.rs`
- [x] Add `resolve_tty_mode(headless: bool) -> TtyMode` pure function
- [x] Write unit tests (truth table) for `resolve_tty_mode`
- [x] Run tests RED→GREEN, commit

### Phase 2 — Mode-aware `build_run_opts` (`dev/run.rs`)
- [x] Add `tty_mode: TtyMode` and `publish: bool` fields to `RunOpts`
- [x] Update `resolve_run_opts` to accept/forward new fields
- [x] Update all 6 `RunOpts` construction sites (compile errors will surface them)
- [x] Split fused `-it` token: headless pushes `-i` only, interactive pushes `-it`
- [x] Split env `extend` block: `USER=` unconditional; color vars interactive-only; `NO_COLOR=1` headless-only
- [x] Make port publishing conditional: `config.publish_port && opts.publish`
- [x] Write headless-mode unit tests (assert `-i`/no `-t`, no `-p`, no color vars, `USER=` present, `NO_COLOR=1` present)
- [x] Run tests RED→GREEN, commit

### Phase 3 — Container naming (`dev/container_name.rs`)
- [x] Extend `resolve_container_name` to accept `Option<&str>` explicit name and `Option<&str>` token
- [x] Implement: explicit name → as-is; headless+token → `cast-{agent}-{basename}-headless-{token}`; otherwise → existing logic
- [x] Write unit tests for all three branches
- [x] Run tests RED→GREEN, commit

### Phase 4 — `DockerClient::headless_command` (`docker/client.rs`)
- [x] Add `headless_command(&self, args: Vec<String>) -> Result<ExitStatus>`
- [x] Same `SignalGuard` as `interactive_command`, uses `.status()` (inherits stdio, no TTY)
- [x] Compile check, commit (no TDD red state — docker cannot run in Nix sandbox)

### Phase 5 — Execution branch (`dev/run.rs`)
- [x] Add `SessionFlags { headless: bool, name: Option<String>, token: Option<String> }` struct
- [x] Update `run_agent` signature to accept `SessionFlags`
- [x] Branch on `TtyMode`: `Interactive` → `docker.interactive_command`, `Headless` → `docker.headless_command`
- [x] Wire `resolve_container_name` with new signature
- [x] Export `SessionFlags` (and `TtyMode`) from `dev/mod.rs`
- [x] Compile check, commit

### Phase 6 — CLI wiring (`commands/cli.rs`)
- [x] Add `RunFlags { headless: bool, name: Option<String> }` struct with clap attributes
- [x] Lift `RunFlags` into `Commands::Run` via `#[command(flatten)]`
- [x] Translate `RunFlags` → `SessionFlags` in thin handler (token from `generate_invocation_id()`)
- [x] Add parse test: `cast run --headless` consumed by RunFlags (subcommand-required failure, not unknown-arg)
- [x] Add parse test: `cast run opencode --headless` passes through as extra_arg
- [x] Add regression test: `cast port opencode --headless` (unaffected)
- [x] Run tests RED→GREEN, commit
