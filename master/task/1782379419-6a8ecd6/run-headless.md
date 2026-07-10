---
title: Add headless execution mode to `cast run`
status: complete
priority: high
---
# Add headless execution mode to `cast run`

`cast run <agent>` currently only supports interactive execution (TTY attached,
`docker run -it`). Running it in any non-terminal environment — systemd timers,
CI pipelines, cron — fails immediately:

```
cannot attach stdin to a TTY-enabled container because stdin is not a terminal
```

This task adds a `--headless` flag to `cast run` that runs the agent without a
TTY, without port publishing, with a non-colliding ephemeral container name, and
with a clean stdout suitable for piping.

## Source

- `spec/cast/run-opts.md` (master branch)
- Research report: `.cue/master/doc/1781804980-2ce36c4/cast-run-opts-research.md`
- Branch: `feat/run-headless`

## Acceptance Criteria

| #  | Criterion                                                                                   | Verify by                                                              | Evidence |
|----|----------------------------------------------------------------------------------------------|------------------------------------------------------------------------|----------|
| 1  | `cast run --headless opencode run "msg"` executes without error from a non-TTY context       | Run from a systemd unit or `setsid cast run --headless ...`            | Manually verified — container starts immediately, output piped cleanly to stdout. Original hang (docker blocking on inherited stdin) fixed in 7c0fe59. |
| 2  | Headless mode does not allocate a pseudo-TTY (`-t` and `-i` absent)                         | `test_build_docker_run_flags_headless_no_tty_flags`                    | 245 tests pass (9a3b154). `-i` removed in 7c0fe59 after post-initial-impl bug. |
| 3  | Headless mode does not publish ports                                                         | `test_build_docker_run_flags_headless_no_port`                         | 245 tests pass (9a3b154). |
| 4  | Headless mode suppresses color/TTY env vars (`TERM`, `COLORTERM`, `FORCE_COLOR`)            | `test_build_docker_run_flags_headless_no_color_vars`                   | 245 tests pass (9a3b154). |
| 5  | Headless container name is unique and does not collide with an active interactive session    | `test_headless_with_token`, `test_headless_token_overrides_config_name` | 245 tests pass (9a3b154). Format: `cast-{agent}-{basename}-{port}-headless-{token}` (64-bit token). |
| 6  | `--name <NAME>` overrides the auto-generated headless container name                         | `test_explicit_name_overrides_all`                                     | 245 tests pass (9a3b154). |
| 7  | Agent exit code is propagated faithfully (non-zero agent exit = non-zero cast exit)         | `ExitStatus` returned by `headless_command` and propagated via `to_exit_code` | Code path verified by inspection; `headless_command` returns `ExitStatus` without bailing. |
| 8  | Cast produces no diagnostic output on stdout in headless mode                               | stdout/stderr inherited from docker; cast itself emits nothing to stdout | Verified manually — only agent JSON output appears on stdout. |
| 9  | Interactive mode (`cast run opencode`) is fully unaffected                                  | Full test suite passes unchanged                                       | 245 tests pass (9a3b154); `interactive_command` code path untouched. |
| 10 | `cast port` is fully unaffected (shares `RunAgent`, must not gain spurious flags)           | `test_cast_port_unaffected_by_headless_extra_arg`                      | 245 tests pass (9a3b154). |
| 11 | `--headless` flag must precede the agent subcommand; parser rejects it after                | `test_cast_run_headless_before_agent_is_consumed`                      | 245 tests pass (9a3b154). |
