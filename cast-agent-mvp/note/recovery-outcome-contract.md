---
status: closed
refs:
- .cue/cast-agent-mvp/doc/streaming-supervisor-design.md
- .cue/cast-agent-mvp/spec/index.md
- .cue/cast-agent-mvp/plan/index.md
- .cue/cast-agent-mvp/doc/cast-agent-architecture.md
---
# Note: recovery-oriented outcome contract + signal handling

> **UPDATE (post-implementation, R3):** this note is a historical
> design-conversation record. Its content has DISSOLVED into the now-complete
> outcome artifacts (`spec/index.md`, `plan/index.md`, the `cast-agent-mvp`
> task card) and the implemented code, so it is CLOSED. Read it as a trail of
> how the contract was reasoned out, not as current truth. Two items below were
> revised at implementation time and supersede the body text:
>
> 1. **`exit.signal` semantics (REVISED in R3 / commit `789f300`).** The body
>    implies `exit.signal` records the *trigger* signal cast-agent received.
>    The implemented contract is: `exit.signal`/`exit.code` ALWAYS reflect the
>    CHILD's actual reaped death disposition (SIGTERM if it honored the grace,
>    SIGKILL if force-killed after the grace; a normal exit code otherwise).
>    The trigger signal cast-agent received moved to a SEPARATE
>    `interrupt_signal` field (present only for `interrupted` outcomes). This
>    makes `TimedOut` and `Interrupted` consistent: `exit` always says *how the
>    child died*, never *why cast-agent acted*. See the `result.json` example
>    below and `finalize.rs::exit_info_from_status`.
>
> 2. **`error_detail` (CLARIFIED).** The draft example shows a stream-derived
>    `"provider rate limit"` detail. In-stream error detection is DEFERRED
>    (passive MVP); `error_detail` is populated only for `SpawnFailed` /
>    `SuperviseFailed` (runtime failures), and is omitted from `result.json`
>    when absent (`skip_serializing_if = Option::is_none`).

Conversation anchor. Exploring whether cast-agent should return a structured
OUTCOME + durable artifact bundle (not just final text), and react to signals,
to enable caller-side recovery/continuation. (Originally "not yet folded into
spec/plan — pending a scope decision"; SINCE folded in and shipped — see banner
above.)

## Motivating scenario (from the user)

A calling agent uses cast-agent to spawn a subagent and wants the final
message. The subagent may never produce it (crash, token-limit, network hang),
but has done partial work. On failure OR on a manual stop signal to
cast-agent, the caller should receive meaningful feedback to decide how to
continue:
- that the task was interrupted/aborted/crashed (+ why),
- a filepath to the full trace log,
- a filepath to the original prompt.

Caller can then: restart, retry with a different harness/model, or hand the
trace to a fresh subagent to analyze + continue.

## Core reframe

cast-agent's return value should be a **structured outcome + artifact bundle**,
meaningful on every exit path — not "final message or error." This is the
distinguishing value over a plain wrapper.

Leans on the already-settled live per-line flush: the flushed run-log is the
**crash-durability substrate** — the trace survives even SIGKILL of the child
or of cast-agent itself. Retroactively validates rejecting buffered output.

## Per-run directory abstraction

One addressable handle bundling everything:

```
<run-dir>/
  prompt.txt      # original prompt, ALWAYS persisted (even stdin/positional)
  stream.jsonl    # full JSON-lines trace, live-flushed
  stderr.log      # child diagnostics
  result.json     # outcome manifest, written on teardown
```

`result.json` shape (draft — see the R3 banner above for the revised
`exit`/`interrupt_signal` semantics; `error_detail` is now spawn/supervise
failures only):
```json
{
  "outcome": "interrupted",
  "final_message": null,
  "exit": { "code": null, "signal": "SIGTERM" },
  "interrupt_signal": "SIGINT",
  "prompt_path": "...", "log_path": "...",
  "event_count": 42, "duration_ms": 18734,
  "error_detail": "provider rate limit (from stream error event)"
}
```

Sets up Phase 4 (detached/multiplexer), where the return value IS a handle to
such a directory.

## Outcome taxonomy + detection

- completed  — exit 0 AND extract_result found a final message.
- failed     — non-zero exit; capture code + stderr tail.
- crashed    — child died by signal (segfault/OOM); wait() reports signal.
- timed_out  — wall-clock deadline fired.
- stalled    — (optional) silence watchdog: no output for N secs.
- interrupted— cast-agent received a signal from user/caller.
- error-in-stream — harness emits a logical failure event mid-stream while the
  process is still alive (opencode `error` event / session.error,
  run.ts:776-784). Detectable because we parse every line live -> can tear down
  proactively. Genuinely beyond a wrapper.

## Signal handling mechanics

Feasible + integrates with the settled design WITHOUT breaking it. Reader tasks
stay independent; the parent select! is a pure control plane (cheap futures),
so adding signal arms is safe (the earlier starvation worry was about heavy I/O
arms, not these).

```rust
use tokio::signal::unix::{signal, SignalKind};
tokio::select! {
    res = child.wait()    => // completed/failed/crashed
    _   = sleep(deadline) => // timed_out
    _   = sigterm.recv()  => // interrupted
    _   = sigint.recv()   => // interrupted
    // _ = stream_error_rx.recv() => // error-in-stream (optional)
}
```
Any arm -> SAME teardown: killpg, join readers (EOF), write result.json, exit
with outcome-specific code. RAII guard backstops future-drop.

Refinements to design in:
1. Two-phase graceful stop: on interrupt, SIGTERM the child group, wait a short
   grace (2-5s) to let it flush/exit, then SIGKILL the group.
2. Double-signal escalation: 1st SIGINT graceful; 2nd SIGINT hard-kill + exit.
3. Durability: if cast-agent is itself SIGKILLed, result.json is absent but
   stream.jsonl is complete-up-to-death -> caller detects "log, no result.json
   = hard-killed" and recovers from the trace.

## Scope tension + recommendation

- Changes the MVP output contract: spec says "plain-text output; structured
  JSON deferred" (spec/index.md; plan decision #2). This pulls a slice of the
  structured envelope + signals forward because unhappy-path feedback is core
  to the value prop. Needs an explicit scope decision (do not silently expand).
- Seams are cheap now, expensive to retrofit. Supervisor already returns a
  RunOutcome enum. Recommendation: build the SEAMS in Phase 1 (run-dir,
  always-on result.json, always-persist prompt, signal-driven teardown, full
  RunOutcome taxonomy), keep happy-path stdout = final message, and STAGE the
  fancier detectors (silence watchdog, in-stream error sensing) as fast-follows.

## Resolved decisions (session 2)

- **stdout contract**: final message on stdout; full envelope in `result.json`;
  distinct exit code per outcome. CONFIRMED.
- **Scope**: signal handling IS in the MVP. Run-dir + always-on `result.json`
  + signal-driven teardown are MVP. "Fancy detectors" (silence watchdog,
  in-stream error sensing) DEFERRED. CONFIRMED.
- **In-stream error detection**: passive only for MVP (react to process exit /
  timeout / signals). Active detection is a future feature — design so it can
  slot in (an optional `stream_error_rx` control arm) but do NOT spec/implement
  now. CONFIRMED.
- **Continuation**: pure trace-as-context, NO native resume. cast-agent just
  produces addressable artifacts; the caller hands the run-dir (`result.json`
  + `stream.jsonl`) to a fresh subagent to read/analyze/continue. `result.json`
  REFERENCES the trace via `log_path` (does not embed the multi-MB trace
  inline). CONFIRMED (embed-vs-reference: awaiting final nod).

## Ctrl-C / interrupt semantics (MVP)

The headline manual-QA + programmatic-stop feature.

**Why Ctrl-C hits cast-agent, not the child:** the tty sends SIGINT to the
terminal's FOREGROUND process group. `process_group(0)` puts the child in its
OWN group, which is never made the terminal foreground — so terminal SIGINT
goes to cast-agent ONLY; the child is insulated. cast-agent thus orchestrates
teardown deterministically and gets to write `result.json` before exit. So
`process_group(0)` does double duty: (a) killpg the whole subtree, (b) insulate
the child from terminal signals. Installing a tokio signal handler also
REPLACES SIGINT's default (terminate) disposition -> cast-agent won't die on
Ctrl-C; it fully controls its own exit.

**Sequence:** signal arm fires -> SIGTERM child group -> short grace window ->
SIGKILL group -> re-`wait()` to reap the child's real disposition -> readers
hit EOF, yield collected events -> write `result.json` (outcome=interrupted;
`exit.signal` = how the child died [reaped], `interrupt_signal` = the trigger
cast-agent received; prompt_path, log_path) -> stdout empty -> exit with
interrupted code. Idempotent/race-safe (killpg on dead group = ESRCH
ignored). (R3: `exit` reflects the reaped child, not the trigger — see banner.)

**Knobs:** (1) double-Ctrl-C escalation: 1st = graceful; 2nd during grace =
immediate SIGKILL + exit (still write result.json). cast-agent owns the SIGINT
handler so the 2nd press can't kill it prematurely. (2) exit-code scheme:
small documented table (0=completed, distinct non-zero per outcome) vs shell
128+signal (SIGINT->130). Leaning small table; open.

**One mechanism, two triggers:** manual Ctrl-C (interactive QA) and
programmatic SIGTERM/SIGINT to cast-agent's PID (caller/operator) take the SAME
graceful path. Handle SIGINT + SIGTERM identically. SIGKILL uncatchable ->
live-flushed stream.jsonl is the durability fallback.

**QA (manual + automatable):**
- Manual: run cast-agent on a long prompt; `tail -f stream.jsonl` in another
  terminal; Ctrl-C mid-run; assert prompt exit w/ interrupted code, result.json
  outcome=interrupted, stream.jsonl has partial trace, `pgrep -g <pgid>` empty
  (no orphans).
- Automated (exec.rs style / AC): spawn cast-agent as child,
  `libc::kill(pid, SIGINT)` mid-run, assert result.json outcome + `killpg(pgid,
  0) == ESRCH`.

## Resolved decisions (session 3)

- **Grace window**: hardcoded 3s for MVP (SIGTERM -> 3s -> SIGKILL). Not a flag
  yet.
- **Exit-code table** (explicit; adjustable):
  - 0 completed
  - 1 failed (child non-zero exit)
  - 2 usage error (bad args / missing --harness / unreadable --file)
  - 3 timed_out
  - 4 interrupted (signal)
  - 5 crashed (child died by signal / spawn failure)

- **result.json vs stream.jsonl — division of labor**:
  - `stream.jsonl` = raw firehose, verbatim, append-only, harness-specific,
    large. The evidence. For deep analysis / handing to another agent.
  - `result.json` = small, structured, harness-AGNOSTIC verdict cast-agent
    synthesizes. The receipt. Fields: `outcome`, `final_message` (happy-path
    payload without parsing the stream), `exit{code,signal}` (child's reaped
    disposition), `interrupt_signal` (trigger, interrupted-only), `error_detail`
    (spawn/supervise failures only), pointers (`log_path`, `prompt_path`),
    metadata (harness, event_count, duration, started_at). Rationale: the stream can't say WHY it stopped (a crash just
    stops); only the supervisor knows. Caller reads result.json first (cheap,
    always present), dives into stream.jsonl only when needed. result.json
    REFERENCES the trace via log_path (never embeds inline). CONFIRMED.

## Run-dir: location + layout

One directory per run. Base-dir precedence:
1. `--run-dir <path>` flag (caller-addressable; the Layer-2 MCP tool uses this).
2. `CAST_AGENT_RUN_DIR` env (mirrors cast's CAST_LOG_DIR/CAST_DATA_DIR).
3. Default `${TMPDIR:-/tmp}/cast-agent/runs/`.

Default rationale: TMPDIR is always set in the Nix devshell/container, persists
for the session, honored, avoids $HOME, and UNIFIES test + prod paths (tests
use std::env::temp_dir() == TMPDIR) -> same code path, Nix-safe for free.

Per-run subdir, timestamp-first (sortable, like cue's <timestamp>-<hash>):
```
$RUN_BASE/20260727T153000Z-opencode-48213/
    cast-agent.pid    # cast-agent's OWN pid (for PID-lookup signalling)
    prompt.txt        # original prompt, always persisted
    stream.jsonl      # full trace, live-flushed
    stderr.log        # child diagnostics
    result.json       # verdict + pointers
```
cast-agent prints the run-dir path to STDERR at startup (before child output)
so you can `tail -f stream.jsonl` immediately; also echoed in result.json. The
run-dir is the single addressable HANDLE (sets up Phase 4 multiplexer).

## Signalling an agent-spawned cast-agent by PID

cast-agent is a normal OS process; works with NO tty (agent/MCP-spawned):
- Spawning parent already holds the PID -> `kill -TERM <pid>`.
- Human: `pgrep -f "cast-agent run"` / `ps aux | grep cast-agent` -> `kill -INT
  <pid>`. Or `cat <run-dir>/cast-agent.pid`.
SIGINT/SIGTERM to that PID hit the SAME graceful teardown -> interrupted
result.json + partial trace. Signal cast-agent's PID (the control point), NOT
the child; killing the child directly -> cast-agent reports crashed/failed via
wait() instead of clean interrupted.
-> Write `cast-agent.pid` into the run-dir + print PID to stderr at startup so
lookup is trivial. The run-dir tells you which PID to signal AND collects
results = complete control+recovery handle.

## Open questions / to drill next

- Final `result.json` field list + which fields are reliably populatable per
  harness (opencode first).
- Whether stderr diagnostics tee to BOTH stderr.log and cast-agent's stderr, or
  file only.
- NEXT: fold resolved MVP scope into spec/index.md + plan/index.md +
  phase-1-mvp.md (adds signal handling, run-dir + artifacts, result.json,
  cast-agent.pid, exit-code table, RunOutcome taxonomy to Slice 1c; updates the
  spec's "plain-text only / structured deferred" boundary; adds the interrupt
  QA as an acceptance criterion).
