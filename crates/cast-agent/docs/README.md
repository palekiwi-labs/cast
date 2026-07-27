# cast-agent Documentation

`cast-agent` launches a supported coding-agent harness (`opencode` first;
`claudecode` and `pi` are planned) as a **process-isolated, supervised child**
in headless JSON-lines mode. It streams the child's output, enforces a
wall-clock timeout, reacts to signals for clean interruption, and produces a
per-run artifact bundle with a structured verdict — so a calling agent gets
meaningful feedback on every exit path, not just the happy one.

## Usage

```
cast-agent run --harness opencode [--file <path>] [--timeout <secs>] \
  [--run-dir <base>] [prompt]
```

- `--harness` is **required** and selects the adapter. The MVP ships
  `opencode` only.
- The prompt is resolved with precedence `--file` > stdin > positional and fed
  to the child over stdin.
- On the happy path the final assistant message is printed to **stdout**;
  diagnostics and the run-dir path go to **stderr**.
- The process exits with an outcome-specific code (see below).

## Run directory (artifact bundle)

Every run creates one addressable directory (base precedence: `--run-dir` >
`CAST_AGENT_RUN_DIR` > `${TMPDIR:-/tmp}/cast-agent/runs/`) containing:

- `prompt.txt` — the original prompt, always persisted.
- `stream.jsonl` — the full JSON-lines trace, live-flushed (tail-able; survives
  a hard kill of cast-agent).
- `stderr.log` — child diagnostics.
- `cast-agent.pid` — cast-agent's own PID, for signalling.
- `result.json` — the structured, harness-agnostic verdict (written
  atomically).

## Outcomes and exit codes

`result.json.outcome` is authoritative; the exit code mirrors it:

- `0` completed
- `1` failed (child exited non-zero)
- `2` usage error (bad args / unreadable prompt / run-dir failure)
- `3` timed_out (wall-clock deadline)
- `4` interrupted (SIGINT/SIGTERM to cast-agent)
- `5` crashed (child died by signal)

## Interruption

cast-agent handles `SIGINT` and `SIGTERM` identically: SIGTERM the child's
process group, wait a fixed 3s grace window, then SIGKILL the group; a second
signal during the grace escalates immediately. The verdict is then written with
`outcome: interrupted`. Because the child runs in its own process group it is
insulated from terminal Ctrl-C, so cast-agent is the sole signal recipient and
always gets to write `result.json`.

## Design references

Authoritative design lives in the `cast-agent-mvp` cue context:

- Concurrency model: `doc/streaming-supervisor-design.md`.
- Outcome + artifacts + signal contract: `note/recovery-outcome-contract.md`.
- Architecture: `doc/cast-agent-architecture.md`.
