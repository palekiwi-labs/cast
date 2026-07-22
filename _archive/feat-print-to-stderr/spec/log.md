# Project Log

## [89c0007] feat: stderr routing + global devshell marker — 89c0007

Implemented the full scope of task `stderr-routing-devshell-markers`. Opus consultation confirmed the eprintln! approach with no abstraction needed.

- **Found:** image.rs and daemon.rs both used println! for status messages, polluting stdout for cast run --headless --format json consumers
- **Found:** Each println! already had a paired tracing::info! for the file log — eprintln! is the correct console complement
- **Found:** Global devshell marker prints from run_agent() immediately before container launch, guarded by user_flake_present
- **Found:** Project devshell marker deliberately skipped — the shellHook >&2 pattern (bab865a) is the correct mechanism
- **Found:** cast port and cast config show stdout output is untouched — cli_test.rs assertions confirm this
- **Decided:** Use eprintln! directly, no StatusWriter abstraction needed
- **Decided:** Global marker in run_agent() only (not shell.rs) — lower value in shell context
- **Decided:** All 9 daemon.rs println! sites changed for uniform status-to-stderr rule
- **Decided:** Skip project devshell marker entirely to avoid sh -c quoting complexity

## [05084f4] docs: shellHook stderr convention — 05084f4

Follow-up from Gemini 3.5 Flash review of feat/print-to-stderr. Added guidance to flake-integration.md on redirecting shellHook echoes to stderr to avoid polluting headless JSON pipelines. Todo captured for resolve_run_opts home-dir testability refactor (low priority).

- **Found:** Gemini confirmed no missed println! sites — all remaining stdout uses are intentional data-output commands
- **Found:** user_flake_present branch in resolve_run_opts has no unit test coverage due to Nix sandbox constraints on $HOME
- **Found:** shellHook >&2 convention was undocumented — added to flake-integration.md
- **Decided:** Act on docs suggestion immediately (small, high value)
- **Decided:** Capture resolve_run_opts testability as low-priority todo rather than implementing now

