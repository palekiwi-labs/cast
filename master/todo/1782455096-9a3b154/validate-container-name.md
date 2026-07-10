---
status: open
priority: low
---
# Validate `--name` against Docker container naming rules

The `--name` override on `cast run --headless` (and `config.container_name` in
`cast.json`) is passed to Docker without validation. Docker names must match
`[a-zA-Z0-9][a-zA-Z0-9_.-]*`. An invalid value produces a Docker error deep
in the call stack rather than a clean early message.

## What to do

- Add `validate_container_name(name: &str) -> Result<()>` in
  `crates/cast/src/dev/container_name.rs`
- Call it in `run_agent` (and optionally at config load time for
  `config.container_name`) before any Docker invocation
- Cover with unit tests (valid names, leading hyphen, space, slash, empty)

## Notes

- Discovered during Sonnet code review of `feat/run-headless` (9a3b154)
- Deferred: Docker's own error is reasonably clear; this is polish, not
  correctness. Branch was ready to merge without it.
