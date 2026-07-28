---
status: complete
priority: critical
refs:
- .cue/cast-agent-mvp/trace/1785127643-8ca9cdd/opus-verification-of-design-review.md
- .cue/cast-agent-mvp/doc/opencode-headless-run-contract.md
- .cue/cast-agent-mvp/doc/streaming-supervisor-design.md
- .cue/cast-agent-mvp/plan/1785127643-8ca9cdd/phase-1-mvp.md
- .cue/cast-agent-mvp/trace/1785127643-8ca9cdd/opencode-self-exit-verification.md
---
> **RESOLVED (GO).** opencode v1.18.4 self-exits (EXIT=0) 52-203ms after the
> final JSON event on every run; `child.wait()` stands, no idle-detection
> fallback needed. stdin-EOF is LOAD-BEARING: opencode blocks on stdin to EOF
> before emitting anything, so `drop(stdin)` is mandatory. extract_result
> "last `text` event" confirmed valid (terminal event is `step_finish`, not a
> trailing text). See trace `opencode-self-exit-verification.md`.

# TODO: Empirically verify opencode self-exits on completion (idle)

**Priority: CRITICAL — this is a go/no-go for the entire supervisor
architecture. Do this BEFORE writing any supervisor code (Slice 1c).**

## Why this matters (finding C6, confirmed by two Opus passes)

The whole supervisor model rests on `child.wait()` returning when the
delegation completes. The opencode headless contract
(`doc/opencode-headless-run-contract.md:77-79`) is worded ambiguously:

> "The run loop terminates when `session.status` becomes `idle` ... cast-agent
> should treat the idle transition **(or process exit)** as end-of-delegation."

That parenthetical "(or process exit)" is a tell — the doc does NOT assert the
opencode process actually EXITS on idle. If `opencode run --format json`
transitions to idle but keeps the process alive (waiting on a server
connection, a lingering Bun event-loop handle, an open stdin, etc.), then:

- `child.wait()` never resolves,
- the wall-clock `timeout` fires,
- **every successful run is misclassified as `timed_out` (exit 3)** — a total
  inversion of the happy path.

The supervisor design's `drop(stdin)` (EOF) is the right instinct and
`Child::wait` also drops stdin on wait, which HELPS if opencode is blocked on
stdin EOF — but is insufficient if it lingers for any other reason. This must
be checked against the real binary, not assumed.

## How to verify

In the cast `agents` devshell (real `opencode` binary available):

1. Run a short headless delegation and observe process lifecycle:
   ```
   echo "say hello and nothing else" | opencode run --format json ; echo "EXIT=$?"
   ```
   - Does the process return to the shell prompt on its own (self-exit)?
   - Time from last JSON-lines event to process exit — is it prompt, or does it
     hang?
2. Confirm the terminating event: capture the JSON-lines and check whether the
   process exits right after the final `text` / an `idle`-status event, or
   whether it stalls with the stream open.
3. Test with stdin closed vs stdin held open, to isolate whether stdin-EOF is
   what triggers exit (informs whether our `drop(stdin)` is load-bearing).
4. Repeat with a prompt that triggers a tool call (longer session) to make sure
   the exit behavior is not specific to trivial runs.

## Acceptance / done when

- Documented evidence (captured stream + observed exit code + timing) showing
  whether opencode self-exits on completion in `--format json` headless mode.
- If it DOES self-exit: record it as a verified grounding fact; the
  `child.wait()`-based supervisor stands.
- If it does NOT: STOP and escalate — the architecture needs an idle-detection
  fallback (parse `session.status == idle` from the stream and tear down), which
  is currently deferred/out-of-scope and would be a scope change.

## Note

`extract_result`'s "return the LAST `text` event" rule (plan `phase-1-mvp.md:97`)
is a RELATED unverified assumption worth confirming in the same session: verify
that the last `text` event actually carries the delegation answer (a trailing
`text`/`step_finish` after the substantive answer would break "last"). Capture
the same stream to check both at once.
