# feat/run-headless

## Problem

`cast run <agent>` only supports interactive execution (TTY attached via
`docker run -it`). Running it from a non-terminal environment — systemd
timers, CI pipelines, cron jobs — fails at the Docker level:

```
cannot attach stdin to a TTY-enabled container because stdin is not a terminal
```

There is also no way to pipe agent output cleanly (e.g. to `jq`) because the
TTY injects control sequences and cast injects color environment variables
unconditionally.

## Goal

Add a `--headless` flag to `cast run` that:

- runs the agent without a pseudo-TTY
- disables host port publishing
- assigns an ephemeral, non-colliding container name
- produces clean stdout suitable for piping

## Scope

### In scope

- `cast run --headless <agent> [extra_args...]`
- Optional `--name <NAME>` override for the container name
- Headless execution via a supervised (signal-safe), non-TTY docker path
- All behavioral changes expressed as pure, unit-tested functions

### Out of scope (deferred)

- `docker exec` into a running container (`--attach` flag) — slice 3
- Per-agent JSON convenience method (`Agent::json_output_args`) — slice 2
- Timeout / watchdog for hanging headless runs — YAGNI
- Re-enabling port publishing in headless mode (`--publish`) — YAGNI

## Design decisions

- **`--headless` as a flag on `cast run`**, not a separate command. The
  operation is identical; interactivity is a mode, not a different operation.
  Precedent: `ssh -T`, `gh --json`, `cargo --message-format`.
- **`docker run`** (new container), never auto-detecting and exec-ing into a
  running container. Lifecycle independence is the primary constraint.
- **Interactive remains the default.** `--headless` is explicit opt-in.
  Rationale: existing users and interactive workflows are unaffected; the
  systemd/CI user knows they need the flag.
- **Cast is format-blind.** `--format json` and similar agent output flags
  belong in `extra_args`. Cast does not inject or interpret them.

## Constraints

- All tests must pass under `nix build` (sandboxed, no network, no `$HOME`).
- Existing interactive behaviour must be 100% unaffected.
- `cast port` (which reuses `RunAgent`) must be unaffected.

## References

- Task: `.cue/master/task/1782379419-6a8ecd6/run-headless.md`
- Original spec idea: `.cue/master/spec/cast/run-opts.md`
- Research: `.cue/master/doc/1781804980-2ce36c4/cast-run-opts-research.md`
