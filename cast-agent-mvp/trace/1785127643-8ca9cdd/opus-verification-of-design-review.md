---
refs:
- .cue/cast-agent-mvp/trace/1785127643-8ca9cdd/opus-design-review-cast-agent-mvp.md
- .cue/cast-agent-mvp/plan/1785127643-8ca9cdd/phase-1-mvp.md
- .cue/cast-agent-mvp/doc/streaming-supervisor-design.md
- .cue/cast-agent-mvp/note/recovery-outcome-contract.md
---
# Opus Adversarial Verification of the Design Review

A SECOND, independent Opus reviewer verified the first Opus design review
(saved in trace/1785127643-8ca9cdd/opus-design-review-cast-agent-mvp.md). The
second reviewer read the actual ground-truth artifacts AND the vendored Tokio
1.52.3 source (process/mod.rs, signal/unix.rs) plus crates/cast/src/mcp/exec.rs
to independently confirm/refute each cited mechanism. Verdicts below.

## Overall

The first review is strong, technically honest, and its two flagship findings
(C1 grandchild-stdout-EOF hang, C6 opencode self-exit) are the right
load-bearing risks — both CONFIRMED correct on the underlying Unix/harness
semantics. Its thesis (risk is in the reader-EOF invariant and the wait/signal
seams, not in the process-group/signal machinery, which is soundly ported) is
correct. Weaknesses are calibration, not existence: some mechanisms imprecise,
two findings under-weighted, five gaps missed. No finding is a pure false alarm.

## Per-finding verification

- B1 (scope; cut to opencode-only): PARTIALLY CORRECT; "keep seams" echoes a
  decision the design already made; "cut adapters" is a PM call, not a
  correctness defect. Real sharpening: claudecode/pi headless FLAGS are guessed
  (the plan admits it) — that is the risk, not "three adapters."
- B2a (re-wait after signal arm to classify exit): CORRECT mechanism. Child::wait
  is FusedChild-cached => cancel-safe, re-callable. But "leaks a zombie" is
  OVERSTATED — kill_on_drop + Tokio's background orphan reaper reap it best-effort.
  The REAL defect is an unclassifiable outcome (can't fill result.json.exit =
  AC7), not zombie hygiene. Reframe accordingly.
- B2b (child.id() -> None after reap; capture PGID once as i32): CORRECT
  (confirmed process/mod.rs:1222, "None to avoid PID-reuse confusion"). But the
  reference exec.rs AND the sketch already do this — it's a "don't regress"
  guardrail, not a found defect.
- B2c (stdin write-before-readers deadlock; BrokenPipe bails before result.json):
  CORRECT and UNDER-stated. Two real modes: (i) prompt > 64KiB pipe buffer +
  interleaving child => classic write-write deadlock; (ii) child exits early =>
  EPIPE on write_all().await? => returns Err BEFORE any teardown => NO result.json
  => violates AC "feedback on every path." Elevate: defeats the core value prop on
  plausibly-common input. Fix: write stdin in its own task (also dissolves the
  deadlock).
- B3 (split guard graceful-vs-Drop; Drop non-blocking; double-signal; coalescing;
  keep streams alive): MOSTLY CORRECT.
  - Two guard behaviors + Drop must not sleep 3s: CORRECT and important.
  - Double-signal via select! on sigint.recv() DURING grace: CORRECT and CONFIRMED
    it works — signal/unix.rs: coalescing is before poll, but once poll is called
    "a further signal is guaranteed to be yielded," so a 2nd SIGINT during the
    grace select IS reliably delivered. A bare sleep(grace) would miss it.
  - recv() coalesces; "any signal during grace escalates" not a precise count:
    CORRECT (unix.rs 316-322).
  - "Keep Signal streams alive or miss signals": OVERSTATED/imprecise mechanism.
    Handler persists process-wide even after the Signal is dropped; coalesced
    notifications survive a poll gap. Real constraint: REGISTER before the signal
    can arrive (before the child work begins), else default terminate disposition
    kills cast-agent. Correct intent, wrong mechanism.
  - Grace 3s->5s: taste/tuning, not correctness. Named constant = good hygiene.
- B4 (schema): CORRECT, well-targeted. Multiple text parts confirmed (contract
  71-72); plan already resolves "last text" but that rule ITSELF is unverified
  (see missed-issue 1). error_detail example is UNREACHABLE in the passive MVP —
  genuine schema-vs-scope inconsistency. Raw signal int alongside name: correct.
  Atomic result.json (tmp+rename) is genuinely LOAD-BEARING: the design's own
  "log exists, no result.json = hard-killed" heuristic breaks on a torn write;
  rename(2) is atomic on POSIX.
- B5 (explicit 0-5; internal-failure has no code; .expect latent panic): CORRECT.
  Mid-run run-dir write failure has no home in the 0-5 table; .expect("open
  run-log") aborts with no result.json => violates feedback-on-every-path.
- B6 (TMPDIR precedence correct; tmpfs OOM; ts+pid collision; atomic write):
  CORRECT; tmpfs-OOM already flagged as an open risk in the plan (minor/knob);
  second-granularity ts + recycled PID collision is low-probability but real for
  the future parallel-spawn use case — cheap random suffix.
- B7 (test holes): CORRECT and the biggest gap is real. BUT half-unfair on "no
  signal-path test": the executive plan already adds an interrupt test (AC6). The
  genuinely-missing net-new tests are double-signal-escalation and
  trap-SIGTERM-forcing-SIGKILL-grace. Test 4 racy (polling) -> deterministic
  barrier: correct. Test 7 assert whole GROUP dead (killpg==ESRCH), precedented by
  exec.rs test: correct.
- C1 (grandchild inherits stdout write-end; setsid'd grandchild => reader never
  EOFs => stdout_task.await hangs FOREVER; bound the join): CORRECT, LOAD-BEARING,
  the single best finding. EOF requires ALL write-ends closed; killpg saves you
  only if every grandchild stays in the group; setsid/setpgid escapes it; plus a
  kill-vs-fd-close race even in-group. Mitigation timeout(2s, join) + fall back to
  fully-flushed on-disk stream.jsonl is exactly right and cheap. ENDORSED, rank #1
  with C6.
- C2 (events Vec is O(run) RAM; fold): CORRECT but OVERSTATED — can't fully fold
  to O(1) without coupling the harness extractor into the generic reader; and
  stream.jsonl already holds the full record. Real but non-urgent; nice-to-have.
- C3 (flush is write(2) not fsync; survives process death not host crash): CORRECT
  and valuable. The design's "survives SIGKILL" claim is correct AS WRITTEN
  (process death, not host crash). Value = defensive: stop a future "fix" to
  fsync-per-line. NUANCE: tokio::fs::File write goes through spawn_blocking to an
  already-unbuffered std write, so the per-line flush() is arguably a NO-OP on top
  of already-written bytes — which STRENGTHENS C4.
- C4 (flush-per-line = syscall/spawn_blocking per line; measure): CORRECT; aligns
  with the design's own open "tokio::fs vs sync BufWriter" item. Measure-then-tune.
- C5 (child ENV unspecified; exec.rs env_clear() drops HOME; zealous port =>
  every real run fails auth; inherit): CORRECT and IMPORTANT — UNDER-weighted by
  the first reviewer. Verified exec.rs:80 env_clear() + test asserts HOME NOT
  inherited (correct for the MCP tool, WRONG for a harness needing API keys +
  HOME/config). Docs are SILENT on child env. 100% of real runs fail without it.
  Container is already the sandbox boundary => no isolation argument to clear.
  Tied with C1/C6 for severity.
- C6 (verify opencode EXITS on session.status==idle; else every success =>
  timed_out): CORRECT, the TOP load-bearing unknown. The contract hedges: "idle
  transition (OR process exit)" — does NOT assert the process exits. If it lingers,
  child.wait() never resolves, timeout fires, every happy path misclassified as
  exit 3. Child::wait drops stdin on wait (helps if blocked on stdin EOF; the
  design's drop(stdin) is the right instinct) but insufficient if it lingers for
  another reason. GO/NO-GO for the whole architecture; verify empirically FIRST.
- C7 (shared TMPDIR base; per-run subdirs fine; don't add shared mutable state):
  CORRECT but trivial/speculative — no shared mutable state in the MVP.

## False alarms / overstatements
- B2a "leaks a zombie": overstated reason (right conclusion). Real cost =
  unclassifiable outcome.
- B2b: not a defect, a don't-regress; design already does it right.
- B3 "keep streams alive or miss signals": imprecise mechanism (register-before-
  arrival is the true constraint).
- B7 "no signal-path test": half false alarm — interrupt test already in the plan.
- C2 "fold to O(1)": overstated (extractor coupling).
- B1 cut-adapters, B3 grace-5s: opinions, not correctness.
- No finding invents a non-issue.

## Issues the FIRST reviewer MISSED
1. extract_result "last text part" is itself an UNVERIFIED assumption (a trailing
   text/step_finish after the answer could break "last"), not merely a
   multiplicity problem — sits next to C6 as an opencode-contract unknown.
2. stderr has the SAME grandchild-inheritance hang as stdout — the bounded-join
   mitigation (C1) must wrap BOTH reader tasks, not just stdout.
3. The control-plane select! is BLIND to reader-task DEATH. If the stdout task
   panics (e.g. the .expect at 154), the parent keeps waiting on the child; the
   trace silently stops and cast-agent runs to timeout with a truncated log and no
   error. Observability gap distinct from "fix the .expect."
4. "Persist artifacts before you can fail" ORDERING invariant: run-dir/prompt.txt/
   cast-agent.pid must be created+flushed BEFORE spawn, or an instant child
   crash/EPIPE loses the very artifacts the recovery contract promises. B2c
   touches the symptom; the general invariant is missed.
5. macOS is ENTIRELY unexamined (target is Linux + macOS). process_group/killpg
   and Tokio's SIGCHLD-based child reaping differ (kqueue vs signalfd); macOS
   default pipe buffer (~16KiB) is SMALLER, making the B2c stdin deadlock MORE
   likely. No finding addresses the second platform.

## Final confidence — genuinely load-bearing vs hygiene

LOAD-BEARING (broken MVP without them):
- C6 verify opencode self-exits on idle — #1, go/no-go, verify empirically FIRST.
- C5 inherit child env — tied #1 blast radius; every real run fails auth; docs
  silent; a naive exec.rs port WILL get it wrong. Cheap fix.
- C1 bound reader-join with timeout + fall back to disk — must wrap BOTH readers.
- B2c write stdin in its own task — breaks feedback-on-every-path for large
  prompts / fast-failing children.
- B2a second wait()/try_wait() after signal-arm kill — fills result.json.exit on
  the headline signal path (AC6/AC7).

LOAD-BEARING-LITE (narrow guarantee, cheap):
- B5 fix latent .expect panics + B4 atomic result.json (tmp+rename) — protect
  feedback-on-every-path and the hard-killed recovery heuristic.
- B7 net-new tests: double-signal + trap-SIGTERM-grace (interrupt test already
  planned).

HYGIENE / DEFERRABLE:
- B1 opencode-only cut (PM call); B3 grace 5s + named constant (the double-signal-
  during-grace SELECT is load-bearing, the 5s value is not); B4 schema
  version/PID/argv/raw-signal-int; B6 tmpfs note + run-dir suffix; C2 event-Vec
  fold; C3 scope-the-durability-claim (doc discipline); C4 measure flush; C7
  shared-state caution.

Bottom line: trust the first review's PRIORITIES (C1 + C6 are exactly right),
tighten its mechanisms (B2a/B2b/B3-keep-alive), elevate C5 and B2c to top-tier,
and add the five missed gaps (last-text assumption, stderr-side hang symmetry,
reader-death observability, persist-before-spawn ordering, macOS).
