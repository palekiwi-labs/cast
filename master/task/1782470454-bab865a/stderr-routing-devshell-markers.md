---
title: Route cast status messages to stderr; add global devshell marker
status: complete
priority: normal
---
# Route cast status messages to stderr; add global devshell marker

Route all `cast` status/progress messages from stdout to stderr, and
print a marker when the global nix devshell is entered. This unblocks
clean programmatic use of `cast run --headless --format json | jq`.

## Source

- todo: `.cue/master/todo/1781804980-2ce36c4/print-on-each-nix-devshell.md`
- Opus analysis: see exec plan for full consultation summary

## Background

In headless mode, `docker run` inherits cast's stdout. The agent's
structured JSON output and cast's own status `println!` calls both
land on the same stream, breaking pipelines like:

```
cast run opencode run "..." --format json | jq
```

`dev/image.rs` and `nix_daemon/daemon.rs` emit status via `println!`.
Changing these to `eprintln!` is the correct fix — they already have
a paired `tracing::info!` that writes to the file log; `eprintln!`
is the console-facing equivalent.

Additionally, a transparent "Loading global nix devshell..." marker is
printed to stderr immediately before container launch when the global
flake is present.

The project devshell marker is NOT added to the command vector — the
correct pattern is `shellHook` echoing to stderr (`>&2`), as committed
in `bab865a`.

## Acceptance Criteria

| #  | Criterion (outcome)                                              | Verify by                                        | Evidence |
|----|------------------------------------------------------------------|--------------------------------------------------|----------|
| 1  | `cast run` status lines appear on stderr, not stdout             | manual inspection / `cast run ... 2>/dev/null`   |          |
| 2  | `cast run --headless --format json` stdout contains only JSON    | `cast run ... --format json 2>/dev/null \| jq .` |          |
| 3  | `cast port` and `cast config show` still output data on stdout   | `cargo test -p cast` (cli_test.rs)               |          |
| 4  | Global devshell marker prints to stderr when user flake present  | manual inspection with a global flake present    |          |
| 5  | All existing unit tests pass                                     | `cargo test -p cast`                             |          |
| 6  | `nix build` succeeds (no sandbox regressions)                    | `nix build`                                      |          |
