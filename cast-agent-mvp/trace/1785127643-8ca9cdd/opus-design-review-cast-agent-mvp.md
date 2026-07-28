---
refs:
- .cue/cast-agent-mvp/spec/index.md
- .cue/cast-agent-mvp/plan/index.md
- .cue/cast-agent-mvp/plan/1785127643-8ca9cdd/phase-1-mvp.md
- .cue/cast-agent-mvp/note/recovery-outcome-contract.md
- .cue/cast-agent-mvp/doc/streaming-supervisor-design.md
- .cue/cast-agent-mvp/todo/1785127643-8ca9cdd/reviewer-prep-checklist.md
---
# Opus Design Review — cast-agent MVP (consultation trace)

Consultant: Opus (consultant-opus). Input package: the reviewer-prep checklist
plus spec/index.md, note/recovery-outcome-contract.md,
doc/streaming-supervisor-design.md, architecture excerpt, opencode-headless
contract, and the task card. This is the raw finding set, saved for the record
and for independent verification by a second Opus pass.

## (A) Overall verdict

Strong, well-reasoned package. The core bet — reader-tasks + a control-plane
`select!` that only holds cheap futures — is CORRECT and is the single most
important decision. It is the difference between a supervisor that works and one
that hangs or fails to time out silent children. The team correctly identified
where cast-agent must diverge from exec.rs (streaming, not wait_with_output).

The scope line is mostly right with one over-reach (adapter breadth). The
biggest risks are NOT in the parts the team is anxious about (signals, process
groups are solid). They are in three places the prep barely touches:
1. grandchildren inheriting the stdout pipe write-end, which breaks the
   "readers hit EOF on group-kill" teardown invariant;
2. child.wait() cancellation-safety and the double-wait problem when
   interleaving signals with the wait;
3. flush-per-line durability being weaker than claimed under host crash, and
   the performance cost being real for chatty harnesses.

"Readers reach EOF naturally on group-kill" is the load-bearing assumption of
the entire teardown model and is only CONDITIONALLY true.

## (B) Answers to the 7 questions

### 1. Scope — keep outcome/artifacts/signals in the MVP
Yes; the bet is correct (seams cheap now, expensive to retrofit once callers
depend on the stdout/exit contract). The recovery loop genuinely is the
differentiating value. BUT push back the other direction: three adapters
(opencode + claudecode + pi) is OVER-scoped. The real risk is the supervisor,
not the adapters; claudecode (--verbose?) and pi (event shape) are UNVERIFIED.
Cut to opencode-only MVP; land claudecode/pi as fast-follow slices (1-file each
via the trait, zero retrofit cost). Defend `prompt.txt` always-persisted — it is
the point of the recovery loop.

### 2. Concurrency — model correct; three under-specified pitfalls
- Starvation correctly avoided (timeout isolated around child.wait()).
- Pipe-buffer deadlock correctly avoided (stderr own task). Necessary: opencode/
  claude emit substantial stderr; a full 64KB pipe with no reader blocks the
  child's write -> stalls stdout.
- (a) child.wait() is cancel-safe, but after a signal arm cancels it you must
  call child.wait().await AGAIN post-killpg to reap and classify code-vs-signal
  for result.json.exit. The sketch only shows the timeout path. Naive impl
  leaks a zombie or reads a status select! threw away. Make it an explicit
  invariant.
- (b) child.id() -> PGID has a lifetime hazard: child.id() returns Option<u32>
  and becomes None once Tokio's background reaper reaps the child. Capture PGID
  ONCE, immediately post-spawn, into a plain i32; never re-read child.id().
  exec.rs port does this right; preserve the ordering in the streaming version.
- (c) stdin.write_all(prompt).await before spawning readers can deadlock on a
  large prompt if the child reads stdin lazily / interleaves, because the stdin
  pipe buffer fills before readers exist. Also handle BrokenPipe: a harness that
  exits immediately (bad args) closes stdin -> write_all().await? Errs and bails
  BEFORE any result.json is written — exactly the "no result.json" failure mode
  you want to avoid, for a case that is not a hard-kill. Fix: spawn readers
  before writing stdin, or write stdin in its own task.

### 3. Signals — sound; kill_now/take() is exactly the right fix
- exec.rs is SIGKILL-only; cast-agent wants SIGTERM-then-SIGKILL. The guard
  needs TWO behaviors: graceful two-phase on the interrupt/timeout arms, and a
  SIGKILL-only, NON-BLOCKING backstop on Drop. Do NOT make Drop do the 3s grace
  sleep — Drop is sync and must not block a runtime worker for 3s. Make the
  split explicit; most likely bug site.
- Grace 3s defensible; lean to 5s named constant (provider socket mid-request).
  On timeout, decide explicitly whether you SIGTERM+grace or SIGKILL-immediate;
  DOC 2 "any arm -> same teardown" implies timed-out runs take deadline + grace.
  State it.
- Double-signal escalation race: after the 1st SIGINT you are sleep(GRACE)-ing;
  to catch the 2nd you must select! on sigint.recv() DURING the grace sleep, not
  block on the sleep. tokio Signal::recv() COALESCES signals — do not build
  correctness on a precise count; make it "any signal during the grace window
  escalates" (naturally race-safe).
- tokio signal caveats: create BOTH Signal streams before entering select! and
  keep them alive the whole run, or early signals are missed. Requires the
  signal feature + a runtime with the signal driver. The "insulate via own
  process group so cast-agent is sole recipient" reasoning is correct.

### 4. result.json schema — mostly good; reliability cracks
- final_message reliability depends entirely on extract_result, the least-
  verified part. opencode emits MULTIPLE text parts (streaming deltas vs final).
  Pin the exact rule (last text event? concat? text at session.status==idle?).
  Keep extract_result -> Option so an exit-0 run whose extraction fails degrades
  to failed-with-trace.
- error_detail is the weak field: the draft example ("provider rate limit from
  stream error event") CANNOT be produced by the passive MVP (in-stream error
  detection is deferred). MVP sources are stderr-tail (failed) and signal name
  (interrupted/crashed) only. Do not ship a field whose documented example the
  MVP cannot produce.
- exit{code,signal}: correct. ExitStatus::code() is None when signalled;
  .signal() via ExitStatusExt gives the number. Map to a name for readability
  but ALSO keep the raw integer (names are not portable).
- Add: a schema `version` integer (the contract will evolve when detectors land;
  version now, it is free) and the child PID/PGID (post-mortem correlation).
  Optionally the exact argv (reproduce a failed run, cheap). started_at as
  RFC3339 UTC.
- Over-specified: nothing egregious; event_count is fine.

### 5. Exit-code table — explicit 0-5 is right; do NOT use 128+signal
Callers are programs branching on OUTCOME (timed_out vs crashed vs failed), not
on how a process died. 128+signal conflates "child crashed" with "cast-agent
itself was signalled." Mapping is clean (2=usage matches convention). Gaps: (a)
reserve exit 4 (interrupted) carefully — cast-agent exits 4 via process::exit(4)
on its own graceful writeback; do not let the default SIGTERM disposition race
you. (b) internal panic / unexpected io::Error (e.g. cannot create run-dir) has
NO mapped code today, and the sketch's .expect("open run-log") is a latent panic
that aborts with no result.json — violates "feedback on every exit path."

### 6. Run-dir — precedence + TMPDIR rationale correct
Unifying test+prod via std::env::temp_dir()/TMPDIR is a genuinely good Nix-
sandbox decision; keep it. Weak spot is retention: "keep everything, no GC" on a
RAM-backed tmpfs means an orchestrator spawning many subagents accumulates
unbounded stream.jsonl firehoses -> can OOM the container. No GC needed for MVP,
but: (a) state the unbounded-tmpfs-growth risk; (b) make naming collision-safe —
second-granularity timestamp + PID can collide on a recycled PID in a container;
add sub-second precision or a short random suffix; (c) write result.json
ATOMICALLY (write .tmp, fsync, rename) so a tailing orchestrator never sees a
half-written verdict — the "log exists, no result.json = hard-killed" heuristic
depends on all-or-nothing result.json.

### 7. Test matrix — well-chosen; coverage holes
The 7 cases target real risks and the printf/sh + tempfile approach is correctly
Nix-compatible. Holes:
- No signal-path test (the headline feature is UNTESTED). Add: spawn supervise,
  send SIGTERM/SIGINT to cast-agent's own process, assert outcome=interrupted,
  exit 4, result.json exists.
- No double-signal escalation test.
- No grace-window test: a harness that ignores SIGTERM (trap '' TERM; sleep 100)
  must be SIGKILLed after GRACE — classic bug site.
- No grandchild-with-inherited-stdout test (see C1).
- Test 4 (live flush) is racy: "watcher asserts line in run-log before supervise
  returns" can flake. Use a deterministic barrier (child prints, blocks on a
  FIFO/stdin byte the test controls; test asserts log then releases child).
  Flaky flush tests get #[ignore]'d and the durability guarantee silently rots.
- Test 7 (future-drop): also assert grandchildren die, not just the direct
  child.

## (C) Gaps the team did NOT ask about

### C1 (THE BIG ONE) — grandchildren inherit the stdout pipe write-end
Teardown rests on: SIGKILL group -> child stdout fd closes -> reader hits EOF ->
stdout_task joins instantly. But grandchildren (opencode/claude spawn tool
subprocesses, MCP servers, language servers, sh -c tool calls) inherit a
DUPLICATE of the stdout pipe write-end. If a grandchild lingers holding it, the
reader NEVER sees EOF and stdout_task.await BLOCKS FOREVER. killpg on the whole
group saves you only if every grandchild is in the group. Failure modes: (i) a
tool that setsid()/setpgid()s out of the group keeps the pipe open; (ii) a race
between SIGKILL delivery and the kernel closing fds. MITIGATION: never
stdout_task.await unbounded during teardown — wrap the reader-join in a short
timeout (e.g. timeout(2s, stdout_task)); on elapse, take the on-disk events and
proceed (partial-log survival already gives you the events). More likely to bite
in production than anything in the signals section; unmentioned in the package.
Add a test: sh -c 'sleep 100 | cat &' grandchild holding stdout; assert
supervise returns promptly after kill.

### C2 — stdout task holds ALL events in a Vec<Value> in RAM
Long run (thousands of tool calls, large tool outputs) accumulates the entire
parsed event stream in memory ON TOP of on-disk stream.jsonl. extract_result
only needs the final message — O(run) memory for an O(1) result. At least note
it; ideally fold over the stream keeping only the last text event + a running
count. Interacts with C1 (less to lose on a hung join).

### C3 — durability claim is overstated
flush().await on a Tokio File pushes the userspace buffer to the kernel via
write(2); it does NOT fsync. Protects against PROCESS death (SIGKILL of
cast-agent — bytes in page cache survive) but NOT machine/container crash or
power loss (unsynced pages lost). The actual threat model is SIGKILL of
cast-agent, for which write-without-fsync is exactly right and fsync-per-line
would be a perf disaster. Behavior correct; SCOPE the claim: "survives
cast-agent process death, not host crash" so nobody later "fixes" it into
fsync-per-line.

### C4 — flush-per-line performance is real for chatty harnesses
A flush().await per line is a syscall per line. opencode/claude verbose/streaming
can emit hundreds of small text/reasoning deltas/sec. Tolerable but not free; the
.await yields the reader each line. MEASURE against a real opencode run; if hot,
use BufWriter with flush on a short interval or on event boundaries (e.g. flush
on step_finish). Resolve the "tokio::fs vs sync BufWriter" open item WITH a
measurement, not a guess.

### C5 — child environment is unspecified
exec.rs does env_clear() then re-adds PATH/TMPDIR + configured vars. cast-agent
spawning a HARNESS needs the harness's FULL env (API keys, provider config, HOME
for config dir). A cleared env fails auth. A zealous env_clear() port makes every
real harness run fail auth with a confusing error. Decide explicitly: inherit
the parent env by default for the harness child.

### C6 — opencode session.status==idle vs process exit
DOC 5 says the run loop terminates when session.status==idle OR process exit.
CONFIRM opencode actually EXITS the process on idle in --format json headless
mode. If it goes idle but keeps the process alive, child.wait() never returns and
EVERY successful run is reported as timed_out — a correctness disaster. #1 thing
to verify empirically against a real opencode run BEFORE writing supervisor code.
The "completed" path depends on the child self-exiting.

### C7 — concurrent-run collisions in the default base dir
Multiple cast-agents share $TMPDIR/cast-agent/runs/. Fine (per-run subdirs), and
cast-agent.pid / stderr PID are per-run. Just don't add shared mutable state
(a "latest" symlink, shared index) without locking.

## (D) Prioritized changes before implementation

1. Empirically verify opencode self-exits on completion in --format json
   headless mode (C6). Gates everything. Cheapest, highest-leverage. Do first.
2. Bound the reader-task join with a timeout during teardown (C1); fall back to
   on-disk events. Add grandchild-holds-stdout test. Closes the most likely
   production hang.
3. Make the child environment explicit (C5): inherit parent env by default; do
   NOT blindly port env_clear() from exec.rs.
4. Specify the double-wait/reap invariant (2a): after any non-wait arm ->
   killpg (two-phase for interrupt/timeout) -> child.wait().await to reap and
   classify. Split ProcessGroupGuard into graceful-two-phase vs SIGKILL-only-
   non-blocking Drop (3).
5. Cut adapters to opencode-only for the MVP (Q1); claudecode/pi as fast-follow.
6. Add signal-path, double-signal, ignore-SIGTERM-grace tests (Q7); de-race
   test 4 with a barrier.
7. Tighten schema (Q4): add version + PID/argv; scope error_detail to MVP-
   available sources; pin the opencode final_message extraction rule; write
   result.json atomically (tmp+rename).
8. Fix the sketch's latent panics (.expect("open run-log"), write_all(prompt)?):
   a failed run-dir/stdin write must still produce a result.json or a mapped
   exit code (Q5).
9. Scope the durability claim to "survives process death, not host crash";
   measure flush-per-line against a real opencode run (C3, C4).
10. Bump grace to 5s named constant; add sub-second/random suffix to run-dir
    names; note the unbounded-tmpfs-growth risk (Q3, Q6).

Two seams are where an otherwise-correct design will actually break in
production: C1 (fd-inheritance teardown) and C6 (opencode-exit assumption).
