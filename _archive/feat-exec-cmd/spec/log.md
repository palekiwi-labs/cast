# Project Log

## [20201aa] Design complete, branch and plan created for cast exec

Extended design session with two rounds of opus consultation. Resolved all open questions and produced a concrete implementation plan.

- **Found:** Only opencode exposes an HTTP server; pi and claudecode gain nothing from port publishing by default
- **Found:** publish_port: bool defaulting true in config is the root cause of unnecessary port publishing
- **Found:** config.port (identity) and publish decision are orthogonal concerns — must stay decoupled
- **Found:** resolve_container_name is already pure/token-driven; a one-line change drops the headless literal
- **Found:** The docker ps --filter property is preserved by shared prefix, not the -headless- segment
- **Found:** cast shell resolves containers by stable interactive-run name — must not be broken
- **Decided:** Use --publish/-p CLI flag (opt-in) with optional u16 value replacing publish_port config bool
- **Decided:** --publish alone = auto (calculated port); --publish 8080 = fixed host port; absent = no publish
- **Decided:** cast exec always starts a fresh container (docker run --rm), never docker exec into running
- **Decided:** ExecAgent stays separate from RunAgent; domain exec() takes SessionFlags + raw: bool
- **Decided:** Option C naming: one stable name (interactive run); everything else token-suffixed
- **Decided:** Drop -headless- literal from ephemeral names; interactive exec token = exec-{invocation_id}
- **Decided:** resolve_container_name signature unchanged (token-driven); caller supplies token string
- **Decided:** --publish works in both interactive and headless modes (decouple from TTY mode)
- **Decided:** nix_daemon::ensure_running called unconditionally even for --raw exec
- **Decided:** agent.prepare_host() called unconditionally from exec (may be first command user runs)
- **Open:** Whether headless cast run should ever be allowed to --publish (currently decided: yes, flag owns the decision)

## [28b17ca] Group 1 complete: port publishing redesign

Replaced the config-level publish_port: bool (default true) with an explicit --publish/-p CLI flag on cast run. All 291 tests green, clippy clean.

- **Found:** publish: true appeared in test helpers across build_command.rs, opencode, pi, claudecode modules — all needed updating
- **Found:** claudecode/mod.rs had minor reformatting from clippy but no logic changes
- **Found:** shell.rs needed publish: None added to its SessionFlags literal — it never publishes ports
- **Decided:** PublishPort::FromStr implemented in run.rs so clap can use it as a direct value parser with default_missing_value = auto
- **Decided:** SessionFlags carries publish: Option<PublishPort> so the flag threads through resolve_run_opts without a signature change
- **Decided:** config.port (identity port for container naming) left completely untouched

## [a3db240] Group 2 complete: container naming unification

Dropped -headless- literal from ephemeral container names. All 219 tests green, clippy clean. Commit a3db240.

- **Found:** resolve_container_name needed only a one-line format string change — no signature changes, no caller wiring changes required
- **Found:** The starts_with invariant test confirms docker ps --filter prefix behaviour is preserved
- **Found:** Callers (cli.rs run.rs shell.rs) already supply the token as a string — they are unaffected by the format change
- **Decided:** Drop -headless- segment: naming is now cast-{agent}-{basename}-{port}-{token} for all token cases
- **Decided:** Added test_token_name_starts_with_interactive_stable_name to formalise the prefix-filter invariant

## [d74e339] Group 3 complete: cast exec subcommand implemented

Added cast exec end-to-end: ExecFlags, ExecAgent, Commands::Exec in cli.rs; dev/exec.rs with build_exec_cmd pure helper; wired through dev/mod.rs. 233 tests green, clippy clean. Commit d74e339.

- **Found:** clap trailing_var_arg does not enforce num_args minimum at parse time — empty cmd caught at dispatch via bail!()
- **Found:** --publish=auto (explicit = form) needed to avoid clap consuming the agent subcommand name as the port value
- **Found:** RunMode::Headless sets TtyMode::Headless via From impl — interactive exec must use RunMode::Interactive with a separately-passed name_token
- **Found:** build_exec_cmd is a pure function testable without docker or agent setup
- **Decided:** exec() signature takes name_token: String separately from SessionFlags so TTY mode and container naming are decoupled
- **Decided:** Interactive exec token = exec-{invocation_id}; headless exec token = {invocation_id} (bare)
- **Decided:** nix_daemon::ensure_running called unconditionally even for --raw (nix store is always mounted)
- **Decided:** Empty cmd bails with a clear usage message rather than silently producing an empty docker run

## [a96ec1b] Slice 1 complete: PublishPort → bool

Replaced Option<PublishPort> enum with plain bool across SessionFlags, RunOpts, RunFlags, and ExecFlags. The --publish / -p flag is now a pure boolean. Deleted PublishPort enum, its FromStr impl, and the Fixed-port test. 234 tests green, clippy clean. Commit a96ec1b.

- **Decided:** --publish is a plain bool; fixed host-port override is a cast.json concern (config.port)
- **Decided:** Drop num_args / default_missing_value / value_name from the clap arg definition
- **Decided:** Delete PublishPort::Fixed entirely — YAGNI, never used in production

## [bc8136a] Slice 2 complete: extract run_in_container

Extracted run_in_container(docker, agent, config, run_opts, container_name, image_tag, cmd) from the ~90% shared body of run_agent and exec. Both functions now delegate to this shared core for prepare_host, docker flag building, tty dispatch, and duration logging. 234 tests green, clippy clean. Commit bc8136a.

- **Decided:** run_in_container lives in run.rs alongside run_agent
- **Decided:** Nix devshell announcement stays in run_agent only (it is a run-specific UX concern)
- **Decided:** exec.rs drops ~30 lines of duplicated orchestration

## [3e356b7] Slice 3 complete: minor review nits

Addressed remaining Opus review findings: container_name.rs doc tightened (--name collision note, 'headless' → 'ephemeral'); ExecAgent num_args = 1.. dropped (dead annotation); test_build_exec_cmd_non_raw_splits_cmd_args fixed (now exercises flake path); S11 test added (exec name ≠ run interactive name); S9 test added (build_exec_cmd safe on empty). 236 tests green, clippy clean. Commit 3e356b7.

- **Decided:** --name collision is documented, not guarded — user opted out of the guarantee
- **Decided:** num_args = 1.. on trailing_var_arg is misleading and silently ignored by clap; removed

## [cd7f378] S9 fixed: exec() bail! enforcement test added

Added test_exec_empty_cmd_returns_error which calls exec() directly with an empty cmd and asserts the bail! at the top of exec() fires before any side effects. The original follow-up test (test_build_exec_cmd_empty_returns_empty) tested the build_exec_cmd helper, not the actual enforcement point — kept as a defensive guard. 237 tests green, clippy clean. Commit cd7f378.

- **Decided:** Keep test_build_exec_cmd_empty_returns_empty as a defensive guard against panics; add test_exec_empty_cmd_returns_error as the authoritative S9 test

