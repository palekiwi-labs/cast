---
title: 'cast-agent: monitor/surface harness stderr'
status: closed
priority: normal
refs:
- /home/pl/code/palekiwi-labs/cast/.cue/cast-agent-mvp/spec/index.md
- /home/pl/code/palekiwi-labs/cast/.cue/cast-agent-mvp/trace/1785256711-2b25028/opencode-run.log
---
# cast-agent: monitor/surface harness stderr

Examine how `cast-agent` should monitor and surface the harness child's
**stderr**, rather than merely draining it to `stderr.log`. Today the
supervisor captures child stderr to `<run-dir>/stderr.log` (deadlock-prevention
drain) but nothing inspects it, so operationally-significant warnings that
opencode writes to stderr — and deliberately omits from the `--format json`
stream — are invisible to the calling agent and to `result.json`.

The deliverable is a design decision (and, if warranted, an implementation)
for: which stderr conditions cast-agent should detect, how to surface them
(a `result.json` field, a distinct/annotated outcome, a stderr tail, and/or a
warning on cast-agent's own stderr), and how to keep it harness-agnostic
(opencode first) without re-introducing the scope the MVP deliberately
deferred (the "fancy detectors": silence watchdog, in-stream error sensing).

## Source

- Spec: `.cue/cast-agent-mvp/spec/index.md` (Outcome + artifacts contract; the
  open question "whether child stderr tees to BOTH `stderr.log` and
  cast-agent's own stderr, or file only").
- Rationale trace: `.cue/cast-agent-mvp/trace/1785256711-2b25028/opencode-run.log`

## Rationale — swallowed invalid-`--agent` warning

The `--agent` flag (commit 2b25028) exists so a caller can faithfully delegate
to a constrained persona (e.g. a read-only Explore subagent). It was added on
the belief that opencode hard-fails an unknown/invalid `--agent` (surfacing as
a failed/crashed verdict). That belief is FALSE.

Observed (confirmed by the user, including under `--format json`): opencode does
NOT fail an invalid/ineligible `--agent`. It writes a warning to **stderr** and
silently downgrades to the default (full-access) primary agent, exiting 0. The
warning is NOT part of the JSON stream:

- `--agent Explore` (not found):
  `!  agent "Explore" not found. Falling back to default agent` -> runs default,
  exit 0.
- `--agent explore` (a subagent, not a primary):
  `!  agent "explore" is a subagent, not a primary agent. Falling back to
  default agent` -> runs default, exit 0.
- `--agent plan` (valid primary): runs `plan`, exit 0.

End-to-end, `cast-agent run --harness opencode --agent Explore "..."` returns
the answer and exits 0 with NO indication the requested persona was never used.
Since the warning is on stderr and absent from the JSON stream, cast-agent
classifies `completed` and the calling model believes it delegated to a
constrained agent while opencode actually launched the default full-access
agent — the exact silent permission regression `--agent` was meant to prevent.

This is the motivating case: cast-agent already has the stderr bytes on disk;
the gap is that nothing monitors them. A general stderr-monitoring capability
would let cast-agent detect this fallback (and similar harness warnings) and
surface it in the verdict.

## Acceptance Criteria

1. **A design decision is recorded** for how cast-agent monitors/surfaces
   harness stderr (which conditions, how surfaced, harness-agnostic seam, and
   its relationship to the deferred "fancy detectors").
   - Verify by: human attestation of the design (doc/note in the cast-agent
     context).
   - Evidence: (who / when)

2. **The invalid-`--agent` fallback is detectable/surfaced** (if the design
   opts to implement): an invalid or ineligible `--agent` no longer yields a
   silent `completed`/exit-0 verdict — cast-agent surfaces the fallback
   (verdict field, annotated outcome, or equivalent).
   - Verify by: `cast-agent run --harness opencode --agent Explore "..."`
     surfaces the fallback rather than a clean success.
   - Evidence: (paste output / test)

3. **Tests pass** for any implemented behavior.
   - Verify by: `cargo test -p cast-agent`
   - Evidence: (paste exit code)
