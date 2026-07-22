# Project Log

## [6a8ecd6] Plan verified by Opus — amendments applied

Opus reviewed the master plan for feat/run-headless and found the design sound but identified two materially incorrect assumptions about the codebase that would have caused immediate failures in Phase 2.

- **Found:** '-it' is a single fused token in build_run_opts (run.rs:130), not separable flags — must be split explicitly
- **Found:** Port publishing is already conditional on config.publish_port (run.rs:148); new opts.publish must AND with it, not replace it
- **Found:** USER= and color env vars are in the same extend() block (run.rs:161-170) — must be split so USER= remains unconditional
- **Found:** Phase 4 (headless_command) has no unit-testable red state — shelling out to docker cannot be tested in Nix sandbox
- **Found:** uuid crate is mcp-feature-gated — cannot use for token generation; generate_invocation_id() from logging.rs is the right primitive
- **Found:** resolve_run_opts (run.rs:104) is the RunOpts constructor and must also be updated — 6 total construction sites
- **Decided:** Color env: omit TERM/COLORTERM/FORCE_COLOR in headless; optionally add NO_COLOR=1; do NOT set TERM=dumb
- **Decided:** Token source: Option C — inject via generate_invocation_id() from cli.rs, passed as parameter to keep naming function pure
- **Decided:** run_agent accepts SessionFlags struct { headless, name, token } — avoids positional-swap hazards
- **Decided:** Phase 4 verified by compilation + manual systemd-timer smoke test, not a TDD red-green cycle
- **Open:** Should NO_COLOR=1 be injected in headless mode or just omit color vars entirely? (user preference)

## [6a8ecd6] Decided: inject NO_COLOR=1 in headless mode

- **Decided:** Headless mode injects NO_COLOR=1 (not optional) and omits TERM/COLORTERM/FORCE_COLOR. TERM=dumb is not used.

## [624fc46] feat/run-headless fully implemented — 624fc46

All 6 phases implemented in a single commit. 246 tests pass (216 unit + 21 CLI integration + 9 config). Existing interactive behavior verified unchanged.

- **Found:** All 6 RunOpts construction sites in test fixtures across build_command.rs, claudecode/mod.rs, opencode/mod.rs (×2), pi/mod.rs plus the main resolve_run_opts needed the new tty_mode/publish fields
- **Found:** shell.rs had a 7th resolve_container_name call site that cargo check (not just the main crate check) surfaced
- **Found:** cast run opencode --help hits config approval before clap help because disable_help_flag=true on RunAgent subcommands; CLI parse tests must use run --help or failure-mode assertions instead
- **Found:** cargo fmt reordered imports in config/loader.rs and lib.rs — benign, included in commit
- **Decided:** Token for headless container name injected from generate_invocation_id() at cli.rs call site; naming function stays pure
- **Decided:** SessionFlags.token = Some(invocation_id) only in headless mode; interactive path passes None
- **Decided:** TtyMode and SessionFlags exported from dev/mod.rs for cli.rs consumption
- **Decided:** CLI parse tests use --headless alone (subcommand-required failure) to prove flag is consumed by RunFlags without triggering config approval

## [7c0fe59] fix: headless hang — removed -i flag, added Stdio::null() guard

cast run --headless was hanging because docker run -i inherited cast's terminal stdin and waited for EOF that never came. Fixed in commit 7c0fe59.

- **Found:** Headless mode was passing -i to docker run, causing docker to block on inherited terminal stdin indefinitely
- **Found:** Container only started after Ctrl-C because SIGINT broke the stdin attach, not because docker was starting normally
- **Found:** design doc claim that -i is 'harmless for fire-and-forget' was incorrect — it is the direct cause of the hang
- **Decided:** Remove -i entirely from headless tty_flags (TtyMode::Headless => vec![])
- **Decided:** Add cmd.stdin(Stdio::null()) in headless_command as belt-and-suspenders — defensive, not load-bearing
- **Decided:** If piped-stdin is ever needed it should be an explicit --stdin flag, not a silent -i

## [1e5e864] refactor: 64-bit token + remove duplicate test (1e5e864)

- **Found:** generate_invocation_id was casting u64 to u32 before formatting — widening the format string alone would not have helped, the cast was the actual truncation
- **Found:** test_cast_run_headless_after_agent_is_passthrough was a duplicate of test_cast_run_headless_flag_in_help with a misleading name
- **Decided:** Use one 16-char ID for both log spans and headless container name token — simplicity over separate IDs
- **Decided:** Remove the misleading test entirely rather than rewriting it

## [9a3b154] refactor: RunMode enum + signal safety + name format (9a3b154)

Four review items addressed in one cohesive refactor commit.

- **Found:** resolve_tty_mode(bool) was dead code after RunMode introduction — replaced by From<&RunMode> for TtyMode
- **Found:** shell.rs had a 4th SessionFlags construction site (interactive-only) that needed updating
- **Found:** clippy flagged match-with-single-arm in headless poll loop — fixed to if let
- **Decided:** RunMode and TtyMode are distinct types at different layers — RunMode is CLI/session (owns token), TtyMode is docker-arg rendering (no payload)
- **Decided:** Headless name format: cast-{agent}-{basename}-{port}-headless-{token} (strict suffix-extension of interactive prefix)
- **Decided:** HeadlessSignalGuard handles SIGINT + SIGTERM + SIGQUIT; interactive_command SignalGuard unchanged (SIGINT+SIGQUIT only, correct for terminal process-group)
- **Decided:** build_run_opts renamed to build_docker_run_flags to disambiguate from resolve_run_opts

## [9a3b154] feat/run-headless verified — ready to merge

User confirmed manual smoke test passed. All task acceptance criteria filled. Master plan and both executive plans marked complete.

- **Found:** 245 tests pass (216 unit + 20 CLI integration + 9 config)
- **Found:** Working tree clean, no uncommitted changes
- **Found:** 4 commits on branch: 624fc46 (feat), 7c0fe59 (fix), 1e5e864 (refactor), 9a3b154 (refactor)
- **Decided:** Task status set to complete with all evidence cells filled
- **Decided:** Master plan and post-review executive plan marked complete

