---
status: complete
---
# Master Plan: feat/run-headless

Implements: `.cue/master/task/1782379419-6a8ecd6/run-headless.md`
Spec: `.cue/feat-run-headless/spec/index.md`

## Problem summary

`cast run <agent>` hardcodes interactive execution (`docker run -it`, color
env vars injected unconditionally, deterministic container name). Non-terminal
environments (systemd, CI) fail at runtime. We add `--headless` as a flag on
`cast run`, altering only what is necessary while leaving the interactive path
100% intact.

## Architecture overview

The change is a mode switch threaded through three layers:

```
cli.rs           dev/run.rs               docker/client.rs
──────────       ───────────────────────  ────────────────────────
RunFlags         TtyMode (Interactive |   interactive_command (TTY)
  .headless  ->    Headless)          ->  headless_command    (no TTY,
  .name            RunOpts.tty_mode         SignalGuard kept)
                   RunOpts.publish
                   RunOpts.container_name_override
                   build_run_opts (conditional -t, -i, -p, env)
                   resolve_container_name (headless + token)
```

All decision logic is extracted into pure functions (testable without Docker).
`DockerClient` gains one new method. `cli.rs` is the only place that touches
`clap`; it translates flags into domain values before calling `dev::run_agent`.

## Key design decisions

### TTY: drop `-t`, keep `-i`
`-t` (pseudo-TTY) corrupts machine output with control sequences and requires a
real terminal. `-i` (stdin forwarding) is harmless for fire-and-forget and
necessary for piped-stdin workflows. Headless = no `-t`, `-i` retained.

### SignalGuard retained in headless path
The `SignalGuard` in `interactive_command` lets Docker own SIGINT and reap the
`--rm` container. Dropping it on a bare `.status()` call risks orphaning the
container if cast is killed before Docker cleans up. The headless path gets a
new `headless_command` method that keeps the signal discipline but omits TTY.

### Color/TTY env suppressed in headless
`build_run_opts` currently injects `TERM=xterm-256color`, `COLORTERM=truecolor`,
`FORCE_COLOR=1` unconditionally in the same `extend([...])` block as `USER=`.
In headless mode:
- `TERM=xterm-256color`, `COLORTERM=truecolor`, `FORCE_COLOR=1` are omitted
- `NO_COLOR=1` is injected (explicit, well-specified signal; broadly respected)
- `TERM=dumb` is NOT set — it can degrade or break agents that check terminfo
- `USER=` is identity, not color, and is kept unconditional in both modes

The `extend` block must be split so `USER=` and `NO_COLOR=1`/color vars are
independently conditional.

### Container naming: ephemeral unique name
Interactive sessions use a deterministic name (stable, findable, re-attachable).
Headless sessions use an ephemeral unique name:

```
cast-{agent}-{basename}-headless-{short_token}
```

Token is generated at the call site in `cli.rs` via the existing
`generate_invocation_id()` (from `logging.rs`) and injected into the pure naming
function as a parameter — keeping the function unit-testable. Do NOT use the
`uuid` crate; it is `mcp`-feature-gated and would fail to compile in the
default run path. `--name` overrides the auto-generated name.

### `run_agent` accepts a `SessionFlags` domain struct
Rather than adding individual parameters, headless mode and name override are
bundled into a small domain struct:

```rust
pub struct SessionFlags {
    pub headless: bool,
    pub name: Option<String>,
    pub token: Option<String>,  // injected by cli.rs
}
```

This avoids positional-swap hazards and keeps the thin handler's job simple:
translate clap `RunFlags` → `SessionFlags` → pass to `run_agent`.

## Implementation phases

### Phase 1 — Pure flag resolution (`dev/run.rs`)

Add `TtyMode` enum:
```rust
pub enum TtyMode { Interactive, Headless }
```

Add a pure resolver:
```rust
pub fn resolve_tty_mode(headless: bool) -> TtyMode { ... }
```

Full unit test suite (truth table). No Docker involved.

### Phase 2 — Mode-aware `build_run_opts` (`dev/run.rs`)

Add `tty_mode: TtyMode` and `publish: bool` to `RunOpts`. Also update
`resolve_run_opts` (the `RunOpts` constructor at run.rs:104) to accept and
forward these new values.

**`RunOpts` construction sites** — all six must be updated (compile-time
errors will surface them):
- `resolve_run_opts` (run.rs:113)
- Five test fixtures (run.rs:241, 290, 318, 352, 391)

Update `build_run_opts`:
- **TTY**: The current fused `"-it"` token (run.rs:130) must be split. In
  headless mode push `"-i"` only; in interactive push `"-it"` (or separate
  `"-i"` + `"-t"`).
- **Port**: Condition becomes `config.publish_port && opts.publish`. The
  existing `config.publish_port` check must be ANDed with the new `opts.publish`
  — not replaced. Headless sets `opts.publish = false`.
- **Color env**: Split the single `extend([USER=..., TERM=..., ...])` block
  at run.rs:161–170 into two parts:
  - `USER=`, `TERM`/`COLORTERM`/`FORCE_COLOR` are currently one call
  - Keep `USER=` unconditional in both modes
  - Inject `TERM=xterm-256color`, `COLORTERM=truecolor`, `FORCE_COLOR=1`
    only in Interactive mode
  - Inject `NO_COLOR=1` in Headless mode (no `TERM=dumb`)

Headless-mode unit tests must assert:
- `-i` present, `-t` / `-it` absent
- `-p` absent even when `config.publish_port == true`
- `FORCE_COLOR` / `COLORTERM` / `xterm-256color` absent
- `USER=` present in both modes

### Phase 3 — Container naming (`dev/container_name.rs`)

Extend `resolve_container_name` (or add a sibling) to accept an
`Option<&str>` explicit name and a `Option<&str>` uniqueness token:

- Explicit `--name` → returned as-is
- Headless + token → `cast-{agent}-{basename}-headless-{token}`
- Interactive / no token → existing deterministic logic unchanged

Keep function pure (token generated and injected by caller). Full unit tests
for each branch.

### Phase 4 — `DockerClient::headless_command` (`docker/client.rs`)

Add:
```rust
pub fn headless_command(&self, args: Vec<String>) -> Result<ExitStatus> { ... }
```

- Same `SignalGuard` / signal masking as `interactive_command`
- Uses `.status()` (inherits stdio, no TTY allocation)
- Returns `ExitStatus` without bailing on non-zero (unlike `stream_command`)

**Note:** This phase has no meaningful red state — `DockerClient` shells out
to real `docker` and cannot be unit-tested in the Nix sandbox. Treat as a
"compiles + manual smoke test" phase, not a red→green cycle. Verification is
via the systemd-timer reproduction case (the original bug trigger).

### Phase 5 — Execution branch (`dev/run.rs`)

Update `run_agent` signature to accept `SessionFlags`:
```rust
pub fn run_agent(
    agent: &dyn Agent,
    config: &ApprovedConfig,
    flags: SessionFlags,
    extra_args: Vec<String>,
) -> Result<ExitStatus>
```

Branch on `TtyMode`:
```rust
let status = match run_opts.tty_mode {
    TtyMode::Interactive => docker.interactive_command(docker_args)?,
    TtyMode::Headless    => docker.headless_command(docker_args)?,
};
```

No other changes to the orchestration logic.

### Phase 6 — CLI wiring (`commands/cli.rs`)

Add:
```rust
#[derive(clap::Args, Clone)]
pub struct RunFlags {
    /// Run without a TTY (for CI, systemd, and piped output)
    #[arg(long)]
    pub headless: bool,

    /// Override the container name (default: auto-generated unique name)
    #[arg(long)]
    pub name: Option<String>,
}
```

Lift to `Commands::Run`:
```rust
Run {
    #[command(flatten)]
    flags: RunFlags,
    #[command(subcommand)]
    agent: RunAgent,
},
```

Translate in the thin handler (clap `RunFlags` → `SessionFlags` → `run_agent`).
Token sourced from `generate_invocation_id()` (logging.rs). Leave `RunAgent`
variants and `Commands::Port` untouched.

Add two parse tests:
1. `cast run --headless opencode run "msg"` — `--headless` consumed by
   `RunFlags`, `run "msg"` in `extra_args`.
2. `cast run opencode --headless run "msg"` — `--headless` in `extra_args`,
   NOT consumed by `RunFlags` (mirror of existing `test_cast_port_ignores_extra_args`
   pattern in cli_test.rs:85).

Also add a `cast port opencode --headless` regression test to confirm
`Commands::Port` is unaffected.

Ensure `dev::SessionFlags` (and `TtyMode` if used directly in cli.rs) are
re-exported via `dev/mod.rs`.

## File change summary

| File | Change |
|---|---|
| `crates/cast/src/dev/run.rs` | `TtyMode`, `SessionFlags`, `resolve_tty_mode`, `RunOpts` fields, `resolve_run_opts` updated, `build_run_opts` conditional, `run_agent` signature + branch |
| `crates/cast/src/dev/mod.rs` | Re-export `SessionFlags` (and `TtyMode` if needed in cli.rs) |
| `crates/cast/src/dev/container_name.rs` | Headless naming with injected token; `--name` override |
| `crates/cast/src/docker/client.rs` | `headless_command` method |
| `crates/cast/src/commands/cli.rs` | `RunFlags`, lift to `Commands::Run`, translate → `SessionFlags` |

No other files require changes for slice 1.
