---
refs:
- /home/pl/code/palekiwi-labs/cast/.cue/master/task/cast-agent-mvp.md
- .cue/cast-agent-mvp/doc/cast-agent-architecture.md
- .cue/cast-agent-mvp/doc/streaming-supervisor-design.md
- .cue/cast-agent-mvp/note/recovery-outcome-contract.md
---
# cast-agent — Spec

> REVIEW NOTE (2026-07-27): the MVP scope was expanded during design to make
> the **unhappy path** first-class — a supervised run now yields a structured
> outcome + a durable artifact bundle, and cast-agent reacts to signals for
> clean interruption. The rationale, alternatives, and the full design trail
> live in `note/recovery-outcome-contract.md` (discussion record) and
> `doc/streaming-supervisor-design.md` (concurrency design). The paragraphs
> below marked "(expanded)" are the additions under review.

## Why

Agent harnesses (`opencode`, `claudecode`) ship built-in "subagent"/task
delegation; `pi` does not. The built-in implementations are inconsistent across
harnesses and give the caller little visibility or control. In particular,
opencode's `task` tool runs a subagent **in-process** (a child session in the
same Bun process), so a stuck, looping, or crashing subagent shares fate and
resources with the parent, and the launch context is hard to govern.

`cast-agent` is the process-isolated alternative. It launches any supported
harness as a **separate, supervised child process** in **headless mode with
JSON-lines streaming**, so the caller retains control: monitor the stream,
enforce a timeout, kill cleanly, and return a properly formatted result.

The distinguishing value over a plain wrapper is **control and recovery**: a
wrapper gives you "final message or nothing," but a subagent frequently does
useful partial work and then fails (crash, token-limit, network hang) or must
be stopped by a human/caller. cast-agent must therefore return **meaningful
feedback on every exit path** — what happened, plus addressable artifacts (the
full trace and the original prompt) — so the calling agent can decide whether
to retry, retry with a different harness/model, or hand the trace to a fresh
subagent to analyze and continue. That recovery loop is the reason cast-agent
exists, so it belongs in the MVP, not a later phase.

## What should be true

- A `cast-agent` binary exists in the cast workspace and exposes a `run`
  subcommand: `cast-agent run --harness <opencode|claudecode|pi> [prompt]`.
- `--harness` is **required** (no default); it is the one flag that cannot be
  no-op'd, selecting the adapter (binary, JSON dialect, result extractor).
- The prompt is supplied via `--file <path>`, stdin, or a positional argument,
  and is fed to the child through stdin uniformly (avoids shell-escaping large
  prompts — the `cue-plugins` temp-file pattern).
- The child runs headless, emitting JSON-lines
  (`opencode run --format json`, `claude -p --output-format stream-json`,
  `pi --mode json`). cast-agent consumes that one stream shape for all three.
- On the **happy path**, cast-agent returns the **final assistant message as
  plain text** on stdout; progress/diagnostics and the raw passthrough stream
  go to stderr/a run-log.
- The child runs in its **own process group**; a wall-clock timeout kills the
  entire group cleanly (mirrors `crates/cast/src/mcp/exec.rs:37-65`).

### Outcome + artifacts contract (expanded)

- **Every run produces a run directory** — one addressable handle bundling:
  `prompt.txt` (the original prompt, always persisted), `stream.jsonl` (the
  full JSON-lines trace, **live-flushed** so it survives even a hard kill and
  can be `tail -f`'d), `stderr.log` (child diagnostics), `cast-agent.pid`
  (cast-agent's own PID, for signalling), and `result.json`.
- **`result.json` is a small, structured, harness-agnostic verdict** that
  cast-agent synthesizes: `outcome` (completed / failed / crashed / timed_out /
  interrupted), `final_message` (when produced), `exit` (code / signal),
  pointers (`log_path`, `prompt_path`), and light metadata (harness,
  event_count, duration). It is the "receipt"; `stream.jsonl` is the
  "evidence." The stream cannot say *why* a run stopped — only the supervisor
  knows — so `result.json` is where the verdict lives. It **references** the
  trace via `log_path`; it never embeds it inline.
- **Distinct process exit code per outcome** (0 completed; non-zero per
  failure kind) for shell/QA ergonomics; `result.json.outcome` is
  authoritative.
- **Signal-driven interruption (expanded).** cast-agent installs handlers for
  `SIGINT` and `SIGTERM`; both trigger the same graceful teardown (SIGTERM the
  child's process group -> a short fixed grace window -> SIGKILL the group),
  then it writes `result.json` with `outcome: interrupted` and the partial
  trace. This serves both **interactive** interruption (Ctrl-C in a terminal)
  and **programmatic** interruption (a caller/operator sends a signal to
  cast-agent's PID with no TTY). `process_group(0)` does double duty here: it
  lets us kill the whole subtree AND insulates the child from terminal signals
  so cast-agent — the sole signal recipient — orchestrates the teardown and is
  guaranteed the chance to write `result.json` before exiting. If cast-agent is
  itself `SIGKILL`ed (uncatchable), the live-flushed `stream.jsonl` is the
  durability fallback.

## Constraints / grounding facts (verified against source)

- All three harnesses emit JSON-lines in headless mode; one internal stream
  consumer suffices. See `doc/opencode-headless-run-contract.md` for the
  opencode dialect (the MVP's first adapter).
- Permissions are already config-governed and sane in headless mode — there is
  no "dead subagent" problem. opencode's global default is `"*": "allow"`; the
  headless auto-reject fires only for tools resolved to `"ask"`. Permission
  escalation is a **later** flag, not part of the MVP.
- The delegation runs **inside the same cast container** as the caller (all
  three harness binaries are provided by the workspace `agents` devshell). The
  container is the sandbox boundary; isolation from the parent is at the OS
  **process-tree** level (process group + SIGKILL).
- `cast-agent` is a **separate binary/crate**, so `cast-agent run` does not
  collide with `cast run`. `run` is a subcommand to reserve the namespace for
  later verbs.

## Scope boundary

In scope: the Priority-1 MVP (headless + foreground + supervised, own cwd,
final-message extraction, process-group timeout kill; opencode adapter first,
then claudecode + pi) **plus the outcome + artifacts + signal-interruption
contract above** (run directory, `result.json`, exit codes, SIGINT/SIGTERM
graceful teardown).

Out of scope (deferred to later priorities, tracked in `plan/index.md`): git
worktrees, the flag vocabulary beyond `--harness`/`--file`, session/resume
(and any native harness `--continue`/`--session`/`--fork` — MVP "continuation"
is purely caller-side, feeding `stream.jsonl` to a fresh run as context), the
multiplexer, and the Layer-2 MCP wrapper tools (which live in `cue-plugins`,
not cast).

**Deliberately deferred, but designed-for (not in the MVP):** the "fancy
detectors" — a **silence watchdog** (kill a run that produces no output for N
seconds, distinct from wall-clock timeout) and **in-stream error detection**
(parse harness `error` events and proactively tear down a doomed-but-still-alive
run). The MVP is **passive**: it reacts only to process exit, wall-clock
timeout, and signals. The supervisor is structured to accept these later as an
extra control-plane arm without a rewrite. See `note/recovery-outcome-contract.md`.

> SCOPE-CHANGE FLAG for the reviewer: this expands the boundary from an earlier
> draft that said "structured JSON output" and "signal control" were later
> priorities. The bet is that the recovery loop is the core value proposition
> and the *seams* (a `RunOutcome` enum, the run-dir, always-persisting the
> prompt, a signal-aware supervisor `select!`) are cheap to build now and
> expensive to retrofit. The *richness* (fancy detectors) is still staged. Is
> this the right scope line, or should any of {run-dir, result.json, signal
> handling} also be deferred?

## Prerequisites / related

- Design origin: the `cast-agent-design` context in `palekiwi/palekiwi`.
- Authoritative architecture: `doc/cast-agent-architecture.md`.
- Additional headless research for later priorities (claudecode + pi flag
  mappings) lives in the `cast-agent-design` context of `palekiwi/palekiwi`
  under `doc/1784797530-5c9a955/`.
