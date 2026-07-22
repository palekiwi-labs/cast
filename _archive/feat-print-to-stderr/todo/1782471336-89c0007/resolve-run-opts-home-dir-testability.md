---
status: open
priority: low
---
# Testability: parameterize home-dir in `resolve_run_opts`

Identified during Gemini code review of `feat/print-to-stderr`.

The `user_flake_present` branch in `resolve_run_opts` (`dev/run.rs`) cannot
be exercised in unit tests because `dirs::home_dir()` reads the real host
`$HOME`, which is `~/.config/cast/nix/flake.nix` — not reachable in the
Nix sandbox (no network, `$HOME` = `/homeless-shelter`).

## Suggested fix

Accept an optional home dir override parameter:

```rust
pub fn resolve_run_opts(
    user: ResolvedUser,
    workspace: ResolvedWorkspace,
    port: u16,
    flags: &SessionFlags,
    home_dir_override: Option<PathBuf>,
) -> RunOpts
```

Call sites pass `None` in production; tests pass a `TempDir`-based path.

## Notes

- Low priority: the production path is correct and working; the gap is
  unit-test coverage only, not a correctness risk.
- Discovered by: Gemini 3.5 Flash diff review, PR `feat/print-to-stderr`.
