---
status: in-progress
priority: high
refs:
- .cue/master/task/cast-agent-mvp.md
- .cue/cast-agent-mvp/spec/index.md
- .cue/cast-agent-mvp/plan/1785127643-8ca9cdd/phase-1-mvp.md
---
# cast-agent MVP — QA verification pass

Manual verification steps to close out the cast-agent MVP. These are the human-
attested criteria blocking `task/cast-agent-mvp.md` from `complete`.

**Prerequisite — get `opencode` on PATH.** `opencode` is NOT in this repo's
`flake.nix` (that flake only provides the Rust toolchain). It lives in the
separate "global flake" scaffolded from
`crates/cast/assets/global-flake-template/` — i.e. the `agents` devshell that
`cast run` provisions. Enter that devshell (or otherwise ensure `opencode` is
on `$PATH`) before starting. Also build cast-agent once:

```
cargo build -p cast-agent --release    # binary at target/release/cast-agent
```

**Conventions for every step below:**
- Run from the cast repo root unless noted.
- Capture the run-dir path cast-agent prints to **stderr** at startup — every
  later check references files inside it. Set it as a shell var:
  `RD=/path/from/stderr/line`.
- `echo $?` immediately after each cast-agent invocation to grab the exit code.

---

## Step 1 — Happy-path smoke (AC 3)

Goal: confirm cast-agent drives a real `opencode` end-to-end and returns the
final assistant message.

```
target/release/cast-agent run --harness opencode "Reply with exactly: PONG"
```

**Pass when:**
- stdout contains the assistant's final text (plain text, not JSON).
- stderr's first line names the run-dir.
- `echo $?` is `0` (completed).

**Record:** the stdout text + exit code. If opencode never starts or the run
hangs, see Step 7 (the `drop(stdin)` / field-shape diagnostics).

## Step 2 — Artifact bundle inspection (AC 7)

Goal: confirm the run-dir bundle is complete and `result.json` is well-formed.

```
ls -la "$RD"
cat "$RD/result.json" | jq .
cat "$RD/prompt.txt"
head -3 "$RD/stream.jsonl"
```

**Pass when all five exist:**
- `prompt.txt` — contains the literal prompt from Step 1.
- `stream.jsonl` — non-empty, one JSON object per line.
- `stderr.log` — exists (may be empty).
- `cast-agent.pid` — contains cast-agent's own PID.
- `result.json` — `jq .` shows:
  - `outcome: "completed"`
  - `final_message`: non-null, matches stdout
  - `exit.code: 0`
  - `log_path` and `prompt_path` point at files that exist
  - `harness: "opencode"`, `event_count` > 0, `duration_ms` populated

## Step 3 — Field-shape verification (closes the UNVERIFIED open item)

Goal: confirm whether opencode `text` events nest content at `part.text` or
top-level `text`. The extractor tolerates both; this records which is real.

```
grep '"type":"text"' "$RD/stream.jsonl" | head -1 | jq .
```

**Record:** paste the raw `text` event JSON. Note which field holds the content
(`.part.text` vs `.text`). This unblocks removing the dual-shape fallback in
`src/harness/opencode.rs` if desired later (or confirms it should stay).

## Step 4 — Live streaming verification (tail -f)

Goal: confirm `stream.jsonl` flushes per line (not buffered) so an operator can
`tail -f` a running delegation.

Open **two terminals**. In T1, run a prompt that does real work (tool use):

```
target/release/cast-agent run --harness opencode \
  "List the files in the current directory using your tools, then summarize."
```

In T2, the moment T1 prints the run-dir, start tailing:

```
tail -f "$RD/stream.jsonl"
```

**Pass when:** new JSON lines appear in T2 **incrementally** while the run is in
progress — not all at once at the end. Stop the tail with Ctrl-C when done.

## Step 5 — Wall-clock timeout (AC 4 manual confirmation)

Goal: confirm a wall-clock timeout kills the whole group and classifies as
`timed_out`.

Use a prompt that will run longer than the timeout:

```
target/release/cast-agent run --harness opencode --timeout 5 \
  "Write a very long essay about the history of computing. Be exhaustive."
```

**Pass when:**
- `echo $?` is `3` (timed_out).
- `result.json`: `outcome: "timed_out"`, `exit.signal` references SIGKILL (9).
- `stream.jsonl` holds the partial trace (Step 4's live flush survives the kill).
- No orphaned opencode/node processes: `pgrep -af opencode` returns nothing
  after cast-agent exits.

## Step 6 — Interrupt path: Ctrl-C (AC 6 manual)

Goal: confirm SIGINT triggers graceful teardown and a recoverable verdict.

Start a long run in T1:

```
target/release/cast-agent run --harness opencode --timeout 300 \
  "Write a very long essay about the history of computing. Be exhaustive."
```

Once it's producing output (verify via `tail -f` in T2), hit **Ctrl-C** in T1.

**Pass when:**
- `echo $?` is `4` (interrupted).
- `result.json`: `outcome: "interrupted"`, `exit.signal` references SIGTERM or
  SIGKILL.
- `stream.jsonl` holds the partial trace up to the interrupt.
- No orphaned processes: `pgrep -af opencode` returns nothing after cast-agent
  exits. (The whole process group should be torn down.)

## Step 7 — Double-Ctrl-C escalation (optional, confirms the grace window)

Goal: confirm a second signal during the grace window escalates immediately to
SIGKILL (rather than waiting the full 3s).

Repeat Step 6's setup, but hit **Ctrl-C twice rapidly**. A harness that traps
SIGTERM would normally ride out the 3s grace; the second signal should bypass
it.

**Pass when:** cast-agent exits promptly (well under 3s after the second
signal) with exit code `4` and `outcome: "interrupted"`.

## Step 8 — Failed-run verdict (AC 7 failed shape)

Goal: confirm a non-zero harness exit classifies as `failed`.

This is hard to trigger deterministically with a real harness. Easiest reliable
method — put a fake `opencode` shim first on PATH that exits non-zero, so
cast-agent resolves and spawns it via the real `base_command()` -> PATH path:

```
mkdir -p /tmp/cast-fake-harness
cat > /tmp/cast-fake-harness/opencode <<'EOF'
#!/bin/sh
echo oops >&2
exit 7
EOF
chmod +x /tmp/cast-fake-harness/opencode

PATH="/tmp/cast-fake-harness:$PATH" \
  target/release/cast-agent run --harness opencode "anything"
```

(The release binary has NO env-var override for the harness anymore — the
historical `CAST_AGENT_FAKE_CMD` test seam was removed because it shipped an
attacker-influencable silent-substitution path in release and made
`result.json.harness` lie. PATH-substitution is the supported way to run a
custom command under supervision: it exercises the same `base_command()` ->
PATH lookup the real harness uses, and `result.json.harness` still truthfully
reports `"opencode"` since that is what was declared and resolved.)

**Pass when:**
- `echo $?` is `1` (failed).
- `result.json`: `outcome: "failed"`, `exit.code: 7`, `final_message: null`.
- `stderr.log` contains `oops`.

Clean up afterward: `rm -rf /tmp/cast-fake-harness`.

## Step 9 — Clean-state build sanity (AC 5 human confirmation)

Goal: confirm the published Nix package builds from clean, not just locally.

```
nix build .#cast-agent
result/bin/cast-agent run --help
```

**Pass when:** the build succeeds and `run --help` renders the opencode-only
CLI surface (`--harness`, `--file`, `--timeout`, `--run-dir`, positional
prompt).

---

## Sign-off

Once Steps 1, 2, 6 pass (the three load-bearing manual checks) and ideally 3-5
and 7-9 as well:

1. Hand the recorded evidence back to the agent (stdout sample, the `text`
   event field shape, exit codes, orphan-process checks, and your
   attestation of the Ctrl-C run).
2. The agent will paste it into the Evidence fields of AC 3 + AC 6 in
   `task/cast-agent-mvp.md`, then set the task `status: complete`.

## If something breaks

- **opencode never starts / run hangs / misclassified `timed_out`:** most likely
  a `drop(stdin)` regression or the field-shape extractor missed — capture
  `stream.jsonl` + `stderr.log` and report; do not patch blindly.
- **orphans survive after Ctrl-C/timeout:** this is the highest-severity
  failure (the whole point of process-group supervision). Capture
  `pgrep -af <harness>` output and the run-dir before reporting.
- **`result.json` missing on any exit path:** should be impossible (atomic
  tmp+rename on every outcome) — if seen, capture how the run ended.
