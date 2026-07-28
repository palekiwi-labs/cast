---
refs:
- .cue/cast-agent-mvp/todo/1785127643-8ca9cdd/verify-opencode-self-exit-on-idle.md
- .cue/cast-agent-mvp/doc/opencode-headless-run-contract.md
- .cue/cast-agent-mvp/doc/streaming-supervisor-design.md
---
# Trace: Empirical verification of opencode self-exit on idle (finding C6)

Go/no-go check for the cast-agent streaming-supervisor architecture. Run against
the **real** `opencode` binary from the `palekiwi#agents` devshell.

- Binary: `/nix/store/vamdy4jqih3l0rv0gzlv3zmxxqisrrbb-opencode-1.18.4/bin/opencode`
- Invocation: `nix develop /home/pl/code/palekiwi/palekiwi#agents -c opencode run --format json`
- Auth/config: available in-sandbox — runs completed with real model calls (no blocker).

## VERDICT

- **Self-exit on completion: GO.** `opencode run --format json` self-exits with
  `EXIT=0` on every run, tens of ms after the final JSON event. `child.wait()`
  resolves on its own; the wall-clock timeout will NOT misfire on the happy path.
  The `child.wait()`-based supervisor design stands.
- **stdin-EOF is LOAD-BEARING (critical caveat).** opencode blocks reading stdin
  to EOF *before it does anything*. Without EOF it hangs indefinitely. The
  design's `drop(stdin)` (and `Child::wait` dropping stdin) is therefore
  mandatory, not merely defensive. Confirmed empirically (see run4).
- **`extract_result` "last `text` event" rule: CONFIRMED valid.** The last
  `text`-typed event carries the substantive answer; the terminal event is a
  `step_finish` (reason `stop`), not a trailing `text`, so a type-filtered
  "last text" selection returns the answer correctly.

## Evidence

### Run 1/2 — trivial prompt, stdin piped (`echo ... | opencode run --format json`)

```
EXIT=0
WALL_MS=7386                # dominated by the model call
LAST_EVENT_TS=1785166057785
PROC_EXIT_TS=1785166057837
LAST_EVENT_TO_EXIT_MS=52    # self-exit ~52ms after final event
```

Stream (`run1.jsonl`, 3 lines):
```
step_start
text     -> "Hello."
step_finish (reason:"stop")
```
Process returned to the shell on its own. Terminal event = `step_finish`, emitted
after the `text`. Self-exit is prompt (~50ms), not a hang.

### Run 4 — stdin held open (prompt via ARG, stdin fed by `sleep 20 | opencode`)

Isolates whether stdin-EOF gates exit:
```
START_MS=1785166125339
EXIT=0  WALL_MS=26394
FIRST_EVENT_OFFSET_MS=26191   # <-- NO events until stdin closed at ~20s
LAST_EVENT_OFFSET_MS=26191
LAST_EVENT_TO_EXIT_MS=203
```
Decisive: even with the prompt supplied as a CLI argument, opencode produced
**zero** output until the `sleep 20` pipe reached EOF at ~20s, then ran the model
(~6s) and exited 203ms after the last event. opencode reads stdin to EOF up-front
and blocks on it. (Run3, `sleep 30 |`, corroborated: WALL_MS=36387.)

Implication: if the supervisor kept stdin open, opencode would never even start,
`child.wait()` would never resolve, and the timeout would misclassify the run as
`timed_out`. `drop(stdin)` after writing the prompt is required.

### Run 5 — tool-calling prompt (non-trivial, multi-step)

`"list the files in the current directory using your tools, then tell me how many there are"`, stdin piped:
```
EXIT=0  WALL_MS=14673
LAST_EVENT_TO_EXIT_MS=79
```
Stream (`run5.jsonl`, 6 lines):
```
step_start
tool_use    (read tool -> directory listing)
step_finish (reason:"tool-calls")
step_start
text        -> "Files in ...\n**Total: 12 entries** ..."   (the answer)
step_finish (reason:"stop")
```
Self-exit confirmed for a longer, tool-using session too (EXIT=0, 79ms after last
event). Exactly one `text` event, and it is the substantive answer; the run ends
with a `step_finish`.

## extract_result detail

- The final assistant answer arrives as a single consolidated `text` event per
  answering step. In both a trivial run and a tool-using run, the LAST `text`
  event held the full answer.
- The stream's final line is always a `step_finish` (not a `text`), so filtering
  events to `type == "text"` and taking the last is correct — there is no
  trailing `text`/`step_finish` that would break the rule.
- Minor caveat to carry forward (not a blocker): if a single final message ever
  streams as multiple `text` parts, "last text" would grab only the last chunk.
  Not observed here (one consolidated part per step); revisit only if longer
  generations split text parts.

## Timing summary

- Last-event -> process-exit: 52ms / 79ms / 203ms across runs. Effectively
  immediate; a modest supervisor deadline comfortably covers teardown.
- Total wall time is entirely the model round-trip (7-15s here), independent of
  exit latency.

## Reproduction

All captures under `/tmp/nix-shell.UFUe4V/opencode/run{1..5}.jsonl` (ephemeral).
Command form:
`nix develop /home/pl/code/palekiwi/palekiwi#agents -c bash -c '... opencode run --format json ...'`

## Bottom line

C6 resolves GO. Proceed with the `child.wait()` streaming supervisor (Slice 1c).
Treat `drop(stdin)` as a load-bearing invariant and cover it with a test: a fake
harness that reads stdin to EOF before emitting output must still be supervised
to a clean Success, proving the supervisor closes stdin. No idle-detection
fallback / scope change is needed for the MVP.
