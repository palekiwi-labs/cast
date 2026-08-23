# Project Log

## [8ca9cdd] Branched feat/cast-agent-mvp; authored Phase 1 MVP executive plan

Checked out feat/cast-agent-mvp from master and created the Phase 1 executive plan (plan/1785127643-8ca9cdd/phase-1-mvp.md) covering slices 1a-1d, grounded in a source-verification pass.

- **Found:** Workspace members only lists cast + cast-mcp-client (Cargo.toml:3-6) — new crate must be added there
- **Found:** exec.rs uses wait_with_output (buffered); cast-agent needs line-by-line streaming, so the tokio::select! loop must differ (reader loop vs timeout sleep)
- **Found:** opencode contract: no -p/--print; JSON is strictly --format json; final result = last `text` event
- **Found:** flake.nix:48-73 has one buildRustPackage per crate with cargoBuild/TestFlags=[-p <crate>] — add a cast-agent block
- **Found:** claudecode stream-json may need --verbose; pi JSON event shape both unverified — must confirm against design-context refs before writing extractors
- **Decided:** Single executive plan covers all of Phase 1 (slices 1a-1d) with TDD red-green-refactor per adapter/orchestrator
- **Decided:** Port ProcessGroupGuard + kill_process_group from exec.rs rather than reinvent
- **Open:** claudecode headless required flags (--verbose?) and pi JSON final-event field names need verification before 1b claude/pi extractors

## [8ca9cdd] Streaming supervision confirmed in-scope for MVP; buffered approach rejected

- **Decided:** Live incremental run-log streaming is a hard MVP requirement (operator must tail -f a running delegation)
- **Decided:** Buffered wait_with_output approach rejected — cannot reuse exec.rs output path verbatim
- **Decided:** Reuse only exec.rs's process-group kill/guard machinery, not its buffered output collection
- **Open:** Concurrency loop shape reconciling live line-reading + concurrent stderr + wall-clock timeout group-kill + no pipe-buffer deadlock — tracked in todo/1785127643-8ca9cdd/streaming-supervision-investigation.md before Slice 1c

## [8ca9cdd] Streaming supervisor design resolved via delegated investigation

Delegated to explore agent (repo source scan) + gemini-pro consultant (tokio concurrency design). Consolidated into doc/streaming-supervisor-design.md. Marked the investigation todo complete and rewrote Slice 1c of the phase-1 executive plan to follow the resolved design.

- **Found:** No existing streaming/BufReader/mpsc subprocess I/O anywhere in the cast repo — genuinely new pattern
- **Found:** tokio 1.52.3 workspace-wide; io-util feature is NOT currently enabled anywhere and is required for BufReader/AsyncBufReadExt
- **Found:** libc 0.2 already a cast dep (crates/cast/Cargo.toml:19), cfg(unix)
- **Found:** kill_on_drop(true) only SIGKILLs the direct child, not grandchildren — explicit killpg is mandatory
- **Found:** test fs convention: route to std::env::temp_dir() (crates/cast/tests/cli_test.rs:6-7)
- **Decided:** Use dedicated reader-task (actor) pattern, NOT a mega tokio::select! — avoids silent-child timeout starvation and stderr pipe-buffer deadlock
- **Decided:** Parent awaits ONLY timeout(deadline, child.wait()); readers are separate tasks joined after group-kill (EOF) so partial log/events survive
- **Decided:** Per-line flush().await on the run-log for live tail -f
- **Decided:** Refine ported guard with kill_now() that take()s pgid to avoid double-kill on drop
- **Decided:** Reuse only exec.rs kill/guard machinery, not its buffered wait_with_output path
- **Open:** Run-log path/naming scheme + tokio::fs (needs fs feature) vs sync BufWriter+spawn_blocking — implementation-time choice

## [8ca9cdd] Recovery/signal contract for cast-agent MVP nailed down

Drilled the recovery-oriented outcome contract + signal handling over three discussion rounds with the user. Captured in note/recovery-outcome-contract.md. Signal handling is now confirmed IN scope for the MVP; fancy detectors (silence watchdog, in-stream error sensing) deferred.

- **Decided:** stdout = final message; result.json = structured harness-agnostic verdict (outcome, final_message, exit, log_path, prompt_path, metadata); distinct exit code per outcome
- **Decided:** Exit-code table: 0 completed, 1 failed, 2 usage, 3 timed_out, 4 interrupted, 5 crashed
- **Decided:** result.json REFERENCES stream.jsonl via log_path; never embeds the trace inline
- **Decided:** Signal handling IS in MVP: SIGINT+SIGTERM -> same graceful teardown (SIGTERM child group -> 3s grace -> SIGKILL); double-signal escalates; RAII guard backstops
- **Decided:** process_group(0) does double duty: killpg the subtree AND insulate child from terminal Ctrl-C so cast-agent orchestrates teardown and writes result.json
- **Decided:** Run-dir per run; base precedence --run-dir > CAST_AGENT_RUN_DIR > ${TMPDIR:-/tmp}/cast-agent/runs/; timestamp-first subdir; unifies test+prod paths (Nix-safe)
- **Decided:** Run-dir bundles cast-agent.pid, prompt.txt, stream.jsonl, stderr.log, result.json; path printed to stderr at startup = addressable control+recovery handle
- **Decided:** Continuation = pure trace-as-context (hand run-dir to a fresh subagent); NO native resume
- **Decided:** In-stream error detection is passive-only for MVP; design an optional stream_error_rx control arm seam but do not implement
- **Open:** Final result.json field list + per-harness populatability (opencode first)
- **Open:** stderr: tee to both stderr.log and cast-agent stderr, or file only
- **Open:** Fold resolved MVP scope into spec/index.md + plan/index.md + phase-1-mvp.md and add the interrupt QA as an acceptance criterion

## [8ca9cdd] Folded recovery/signal contract into spec, plans, and task card for review

Updated spec/index.md, plan/index.md, the phase-1-mvp executive plan, and the master task card to reflect the expanded MVP scope (outcome + artifacts + signal interruption). Added reviewer-oriented scope-change flags and cross-refs to note/recovery-outcome-contract.md and doc/streaming-supervisor-design.md so an external consultant has the full rationale and research trail. Note kept in-progress (not closed) pending the review.

- **Decided:** spec: added an Outcome + artifacts contract section and a signal-interruption subsection; scope boundary now includes run-dir/result.json/signals and explicitly defers the 'fancy detectors'
- **Decided:** master plan: added a streaming-supervisor + recovery design summary, run-dir location/layout, exit-code table; revised key design decisions (dual output, signals MVP, live-flush hard requirement, passive-only) and open questions
- **Decided:** executive plan: Slice 1c rewritten into input+run-dir / supervisor / finalize / tests subsections; added AC6 (interrupt) + AC7 (artifacts) mapping and risks
- **Decided:** task card: scope + 2 new acceptance criteria (AC6 signal interruption, AC7 artifact bundle + result.json)
- **Open:** External consultant critique of the expanded scope line (is signal handling + result.json rightly in the MVP, or should any be deferred?)
- **Open:** Final result.json field list + per-harness populatability
- **Open:** child stderr: tee to both stderr.log and console, or file only
- **Open:** run-dir retention/cleanup policy

## [8ca9cdd] Two-pass Opus review of cast-agent MVP design; C1+C5+C6 confirmed load-bearing

Consulted Opus on the expanded cast-agent MVP plan (spec + streaming-supervisor design + recovery contract), then had a SECOND independent Opus adversarially verify the first review against ground-truth artifacts and vendored Tokio 1.52.3 source. Both saved as traces. Verification largely upheld the first review's priorities but recalibrated severity and added five missed gaps.

- **Found:** Core design bet (reader-tasks + control-plane select! holding only cheap futures) is CONFIRMED correct; the risk is the reader-EOF invariant + wait/signal seams, not the process-group/signal machinery
- **Found:** C6 (verify opencode actually EXITS on session.status==idle in --format json) is the TOP go/no-go unknown: if it lingers, child.wait() never returns and every happy path is misclassified timed_out. Contract hedges 'idle transition (OR process exit)'
- **Found:** C1 (grandchildren inherit a duplicate stdout pipe write-end; a setsid'd grandchild => reader never EOFs => stdout_task.await hangs forever) CONFIRMED load-bearing. Same hang applies to stderr. Fix: bound BOTH reader joins with a timeout, fall back to on-disk stream.jsonl
- **Found:** C5 (child env unspecified; a naive env_clear() port from exec.rs drops HOME/API keys => 100% of real harness runs fail auth) UNDER-weighted by first review; tied top-tier. Inherit parent env by default
- **Found:** B2c (writing prompt to stdin before spawning readers) can deadlock on >64KiB prompts and EPIPE-bails before any result.json is written => breaks 'feedback on every exit path'. Fix: write stdin in its own task
- **Found:** B2a: after a signal arm cancels child.wait(), must call wait()/try_wait() AGAIN post-killpg to classify exit (fills result.json.exit / AC7); 'leaks a zombie' overstated (kill_on_drop reaps it) - real cost is unclassifiable outcome
- **Found:** Confirmed Tokio facts: Child::wait is FusedChild-cached (cancel-safe, re-callable); child.id()->None after reap (PID-reuse); Signal::recv coalesces but a signal arriving while already polling recv() is guaranteed yielded (double-signal-during-grace works); flush() is write(2) not fsync (survives process death not host crash)
- **Found:** atomic result.json (tmp+rename) is load-bearing: the 'log exists but no result.json = hard-killed' recovery heuristic breaks on a torn write
- **Decided:** Trust the first review's priorities (C1 + C6 are exactly the right load-bearing risks); tighten its imprecise mechanisms; elevate C5 and B2c to top-tier severity
- **Decided:** Load-bearing must-fixes before implementation: C6 verify-opencode-self-exit (FIRST, empirical), C5 inherit-env, C1 bounded reader-joins (both pipes), B2c stdin-in-own-task, B2a re-wait-to-classify
- **Open:** Five gaps the first review missed, now on record: (1) extract_result 'last text' is itself unverified, (2) stderr-side grandchild hang symmetry, (3) control-plane select! is blind to reader-task death/panic, (4) 'persist run-dir/prompt.txt/pid before spawn' ordering invariant, (5) macOS entirely unexamined (killpg/SIGCHLD reaping differ; ~16KiB pipe buffer makes B2c deadlock more likely)
- **Open:** Empirical verification still owed: does opencode run --format json self-exit on idle? claudecode --verbose requirement + pi JSON event shape still guessed
- **Open:** Decision pending: fold these findings into spec/plan/phase-1-mvp + task ACs, or open discrete todos for the verification items first

## [8ca9cdd] Folded resolved review + verification findings into phase-1-mvp plan and supervisor design

Folded the two empirical verification traces (opencode self-exit, run-log flush benchmark) and the load-bearing must-fixes from the two Opus review passes into plan/1785127643-8ca9cdd/phase-1-mvp.md (new 'Pre-implementation findings' section + Slice 1c invariants + revised notes/risks) and doc/streaming-supervisor-design.md (a 'Post-review updates' block overriding the pre-review sketch + resolved open items). Marked reviewer-prep-checklist, verify-opencode-self-exit-on-idle, and measure-flush-per-line-cost todos complete.

- **Found:** C6 GO: opencode run --format json self-exits EXIT=0 ~50-200ms after final event; child.wait() supervisor stands, no idle-detection fallback needed
- **Found:** drop(stdin) is LOAD-BEARING: opencode blocks reading stdin to EOF before starting; open stdin => never starts => misclassified timed_out
- **Found:** extract_result last-text rule CONFIRMED valid (terminal event is always step_finish, not trailing text)
- **Found:** Run-log writer benchmark: sync std::fs::BufWriter on blocking thread flush-per-line = 2.1M lines/sec vs tokio::fs 86k (24x); tokio::fs flush().await is a near no-op; no fs feature needed
- **Decided:** Slice 1c invariants baked in: C5 inherit parent env (no env_clear), C1 bounded reader-joins on BOTH pipes w/ disk fallback, B2c stdin in own task, B2a re-wait to classify, capture PGID once as i32, split guard graceful-vs-nonblocking-Drop, double-signal select during grace, persist-before-spawn, atomic result.json (tmp+rename)
- **Decided:** Run-log writer = sync BufWriter blocking thread flush-per-line; tokio fs feature dropped from Cargo.toml deps
- **Decided:** Added tests: grandchild-holds-pipe (both pipes), stdin-EOF invariant, double-signal escalation, trap-SIGTERM grace, de-raced live-flush barrier
- **Open:** claudecode --verbose + pi JSON event shape still UNVERIFIED (consider opencode-only MVP + fast-follow)
- **Open:** macOS second platform unexamined: killpg/SIGCHLD reaping differ, ~16KiB pipe buffer makes B2c deadlock likelier
- **Open:** run-dir retention/unbounded-tmpfs-growth policy (keep-everything for MVP)

## [8ca9cdd] Scoped MVP to opencode-only, Linux-container-only (macOS dropped)

User made two scope calls before implementation begins: (1) the MVP ships the opencode adapter only; claudecode + pi adapters are deferred to fast-follow since their JSON event shapes / required flags remain unverified. (2) No macOS work is needed at all — cast-agent runs exclusively inside a Linux container, so the killpg/SIGCHLD/pipe-buffer cross-platform concerns are moot for this project.

- **Decided:** MVP = opencode adapter only; claudecode/pi deferred to fast-follow (their JSON shapes/flags stay unverified for now)
- **Decided:** macOS is out of scope permanently: cast-agent runs exclusively inside a Linux container, so cfg(unix)/Linux is the only target and the macOS killpg/SIGCHLD/pipe-buffer open item is closed
- **Decided:** Harness trait + dispatch still built to accept more adapters later, but only OpenCode is wired for the MVP

## [c650dfc] Slice 1a complete: cast-agent crate scaffolded (AC 1)

Added crates/cast-agent to workspace members; authored Cargo.toml modeled on cast-mcp-client with tokio features process/signal/io-util/time/sync and cfg(unix) libc; stubbed main.rs with a clap-derive `run` subcommand (--harness required enum, --file, --timeout default 300s, --run-dir, positional prompt). HarnessKind is opencode-only per the scope decision. cargo build/clippy/fmt clean; `run --help` renders. Committed c650dfc.

- **Decided:** clap HarnessKind enum ships opencode-only (single variant) — honest CLI surface for the MVP; extend the enum when claudecode/pi land
- **Decided:** run body is todo!() until orchestrator wired in Slice 1c
- **Decided:** --timeout is a plain u64 seconds with default 300

## [f0c4177] Slice 1b complete: Harness trait + opencode adapter (AC 2)

Defined the Harness trait (name/base_command/headless_args/extract_result + None-returning flag-mapper defaults) in src/harness/mod.rs and implemented the OpenCode adapter via TDD. Added a lib target (lib.rs) so modules are unit/integration testable. headless_args = [run, --format, json]; extract_result filters type==text events and takes the last, driven against a JSON-lines fixture (tests/fixtures/opencode-run.jsonl) plus no-text and multiple-text cases. Two commits (headless_args green, extract_result green).

- **Found:** opencode source at /home/pl/code/anomalyco/opencode is NO LONGER on disk, and the self-exit trace captures event TYPES but not the raw JSON field name for text content — could not re-verify the exact shape
- **Found:** The opencode contract doc states tool_use 'carries the full part object', implying event data nests under `part`; so the text event most likely nests content under part.text
- **Decided:** extract_result is robust to BOTH shapes: tries part.text first, falls back to top-level text — avoids guessing a single field name we cannot currently verify
- **Decided:** Added a lib.rs (lib+bin crate) so harness modules are testable from tests/ and reusable by the orchestrator
- **Open:** UNVERIFIED: exact opencode `text` event JSON field name (part.text vs text). Extractor tolerates both, but confirm against a real `opencode run --format json` capture during the Slice 1c smoke test before claiming AC 3.

## [39d9b63] Slice 1c progress: prompt resolution + run-dir modules (TDD green)

Built the two pure orchestrator pieces first. prompt.rs: choose_prompt precedence resolver (--file > stdin > positional; blank/whitespace stdin treated as absent) + resolve_prompt IO wrapper using isatty to decide whether to drain stdin. rundir.rs: resolve_base precedence (flag > CAST_AGENT_RUN_DIR > ${TMPDIR:-/tmp}/cast-agent/runs) + resolve_base_from_env + create_run_dir making a sortable timestamp-first subdir (secs+nanos-harness-pid) that is collision-resistant. Both fully unit-tested (temp-dir fs, Nix-safe). Two commits.

- **Decided:** Run-dir subdir prefix uses epoch secs+zero-padded-nanos (numeric, sortable) rather than the cosmetic 20260727T153000Z calendar format from the master plan — dependency-free (no chrono), still sortable, and the nanos component gives the collision-resistance the plan asked for. Minor deviation, noted.
- **Decided:** Prompt precedence: whitespace-only stdin is treated as absent so a positional still applies
- **Open:** Next: the supervisor (reader-task pattern) — the load-bearing part. Then finalize/result.json + main.rs wiring + failure-mode test matrix.

## [39d9b63-dirty] Slice 1c: supervisor implemented; 8/9 tests green, group-kill test failing (handoff)

Implemented the streaming reader-task supervisor (src/supervisor.rs) faithful to streaming-supervisor-design.md + recovery-outcome-contract.md: process_group(0) child, prompt-to-stdin-in-own-task then EOF, dedicated stdout reader (live per-line flush via a sync BufWriter on a std thread fed over a channel + parsed-event collection), dedicated stderr drain to stderr.log, control-plane select! over child.wait()/wall-clock timeout/SIGINT/SIGTERM, split ProcessGroupGuard (terminate SIGTERM / kill_now SIGKILL+disarm / non-blocking Drop), two-phase graceful teardown with double-signal escalation during the grace window, B2a re-wait-to-reap, C1 bounded reader joins with on-disk fallback, C5 inherit env. Wrote a 9-test failure-mode matrix; 8 pass. The AC-4 analogue (timeout_kills_entire_process_group) fails: killpg(pgid,0) still reports the group alive 150ms after kill_now. Full diagnosis + hypotheses + recommended fix in the trace. Supervisor code is UNCOMMITTED (RED phase) pending the fix.

- **Found:** 8/9 supervisor tests green: normal completion, no-trailing-newline, silent-hang timeout, partial-log survival, stderr firehose (deadlock prevention), stdin-EOF invariant, live-flush-before-return, non-zero exit
- **Found:** Failing: timeout_kills_entire_process_group — group still alive 150ms after SIGKILL. Likely reap/settle-timing or dash exec-optimization changing the group leader vs out.pgid; the bash repro attempt was invalid (sh inherited bash's group)
- **Decided:** Run-log writer implemented as the benchmark-backed sync std::fs::BufWriter on a dedicated OS thread fed via std::sync::mpsc, flush-per-line — reused for BOTH stream.jsonl and stderr.log
- **Decided:** supervisor module gated #[cfg(unix)] (Linux-container-only scope)
- **Open:** FIX FIRST: re-model the group-kill test on the proven exec.rs test (child writes $$ to a tempfile; assert killpg(pgid,0)==ESRCH; 400ms deadline/100ms settle) to isolate supervisor reap-ordering vs the kill primitive
- **Open:** Then: finalize/result.json (atomic tmp+rename) + exit-code map + persist-before-spawn (prompt.txt, cast-agent.pid) + stderr run-dir print
- **Open:** Then: wire main.rs run end-to-end + e2e stream test (AC 3) + interrupt/double-signal/trap-SIGTERM tests (AC 6)
- **Open:** Then: Slice 1d flake.nix cast-agent package + docs README + nix build/QA smoke

## [fe555b4] Slice 1c supervisor committed; group-kill test failure diagnosed (sandbox zombie-reap)

Committed fe555b4: streaming reader-task supervisor + 9-test failure-mode matrix (all green). Diagnosed and resolved the previously-failing timeout_kills_entire_process_group test.

- **Found:** Root cause was NOT a supervisor bug. The supervisor correctly SIGKILLs the whole process group. In this container/nix sandbox pid 1 is not a reaping init, so backgrounded grandchildren reparented to pid 1 linger as ZOMBIES (state Z) indefinitely.
- **Found:** killpg(pgid,0) returns 0 for dead-but-unreaped zombies, so it cannot prove teardown here; zombies even accumulated across test runs (pgid 3717,3742,3916...).
- **Found:** exec.rs's analogue passes only because its shell exec's into a single foreground sleep (the direct child, which we reap) with no orphaned grandchild; the 'process tree' name is aspirational.
- **Found:** The failing 150ms settle AND a 3s settle both failed on killpg because the zombies never get reaped by pid 1 in this environment.
- **Decided:** Fixed the test, not the supervisor: assert no LIVE (non-Z) members remain in the group by parsing ps -eo pid,pgid,stat,comm, polled up to 3s. Deterministically verifies the real requirement (no live orphan escapes) regardless of init reaping.
- **Decided:** Added live_group_members() + wait_group_dead() helpers; removed killpg from the death assertion.
- **Open:** finalize/result.json module (atomic tmp+rename) + exit-code map + persist-before-spawn (prompt.txt, cast-agent.pid) + stderr run-dir print
- **Open:** wire main.rs run end-to-end + e2e stream test (AC 3)
- **Open:** interrupt/double-signal/trap-SIGTERM tests (AC 6) - need main.rs + result.json first
- **Open:** explicit grandchild-holds-pipe bounded-join test (C1)
- **Open:** Slice 1d flake.nix cast-agent package + docs README + nix build/QA smoke

## [eeb4c18] Slice 1c: finalize/result.json module (GREEN, committed eeb4c18)

Added src/finalize.rs: Outcome enum + exit-code table, classify(EndReason)->(Outcome,ExitInfo), build_verdict, atomic write_result_json. 6 unit tests green.

- **Decided:** classify maps EndReason::Exited via status.code()/signal(): 0->Completed, nonzero->Failed, no-code(WIFSIGNALED)->Crashed; TimedOut->signal SIGKILL; Interrupted->the received signal
- **Decided:** result.json written atomically (write .json.tmp then rename) per the torn-write recovery heuristic
- **Decided:** exit codes: 0 completed,1 failed,3 timed_out,4 interrupted,5 crashed (2 usage emitted by CLI layer, not here)
- **Decided:** unit-tested classify without spawning via ExitStatus::from_raw (from_raw(7<<8)=code7, from_raw(9)=SIGKILL)
- **Open:** wire main.rs run end-to-end (resolve prompt, persist-before-spawn prompt.txt+cast-agent.pid, print run-dir to stderr, supervise, extract_result, write result.json, print final_message to stdout, exit with code)
- **Open:** e2e stream test (AC 3) with scripted fake harness
- **Open:** interrupt/double-signal/trap-SIGTERM tests (AC 6)
- **Open:** Slice 1d flake.nix + docs README + nix build/QA

## [d39b279] Slice 1c: orchestrator wired end-to-end (GREEN, committed d39b279)

Added src/run.rs (orchestrate + RunLayout + limits_from_timeout) and rewrote main.rs as a thin tokio CLI shell. 3 e2e orchestrate tests green (completed/failed/timed_out). Full suite 32 tests pass; clippy/fmt clean; run --help renders opencode-only.

- **Decided:** orchestrate takes fully-formed exe+args (prod: base_command()+headless_args(); tests inject sh fake) plus &dyn Harness for extract_result+identity — lets AC3 e2e run without a real harness binary
- **Decided:** extract_result called ONLY on Outcome::Completed; final_message null otherwise
- **Decided:** persist-before-spawn writes prompt.txt + cast-agent.pid before supervise
- **Decided:** main prints run-dir + pid to stderr at startup, final_message to stdout, exits with outcome code; usage errors (bad prompt / run-dir create fail) exit 2 before any run
- **Decided:** grace window fixed 3s via limits_from_timeout
- **Open:** interrupt tests (AC 6): spawn cast-agent binary as child, send SIGINT/SIGTERM, assert result.json outcome=interrupted + no live group members; plus double-signal escalation + trap-SIGTERM-grace
- **Open:** explicit grandchild-holds-pipe bounded-join test (C1)
- **Open:** manual smoke vs real opencode (tail -f + Ctrl-C)
- **Open:** Slice 1d flake.nix cast-agent package + docs README + nix build

## [00c6ca1] Slice 1c: signal-interruption tests green (AC 6, committed 00c6ca1)

Added CAST_AGENT_FAKE_CMD test seam to main.rs and tests/interrupt_test.rs (3 tests). Full suite now 35 tests green; clippy/fmt clean. Slice 1c is functionally complete (supervisor + finalize + orchestrator + interrupt).

- **Decided:** CAST_AGENT_FAKE_CMD env override runs `sh -c <value>` in place of the harness binary so the binary-level supervision/signal path is testable without a harness installed; production always takes the harness branch
- **Decided:** interrupt test asserts via fake-recorded child PGID + no-live-members poll (killpg unusable due to non-reaping pid 1); checks exit code 4 and result.json outcome=interrupted + exit.signal
- **Decided:** trap-SIGTERM test verifies the 3s grace elapses before SIGKILL
- **Open:** manual smoke vs real opencode run --format json (tail -f + Ctrl-C) — needs devshell with opencode
- **Open:** explicit grandchild-holds-pipe bounded-join C1 test (optional; covered indirectly)
- **Open:** Slice 1d: flake.nix cast-agent buildRustPackage + crates/cast-agent/docs/README.md + nix build/flake check
- **Open:** update phase-1-mvp plan checkboxes + task card ACs

## [3ada0d3] Slice 1d complete: nix package + docs README (AC 5, committed 3ada0d3)

Added cast-agent buildRustPackage to flake.nix (nativeCheckInputs bash/coreutils/procps) and crates/cast-agent/docs/README.md. `nix build .#cast-agent` succeeds with all tests passing in the reproducible sandbox.

- **Found:** nix build .#cast-agent runs the full test suite (lib + all integration tests) in the sandbox and reaches installPhase => AC 5 satisfied; the fake-harness (sh) and process-group (ps) tests work under nativeCheckInputs bash/coreutils/procps
- **Found:** no src-filter change needed: cast-agent embeds no docs (unlike cast which special-cases /crates/cast/docs)
- **Decided:** Slice 1d done. AC 1-7 met in code; only the manual opencode smoke (AC 3 human attestation, needs a devshell with opencode) remains outstanding
- **Open:** manual smoke vs real opencode run --format json (tail -f live stream + Ctrl-C interrupt) — requires opencode-provisioning devshell; human-attested
- **Open:** update phase-1-mvp plan checkboxes + master task card AC evidence
- **Open:** verify UNVERIFIED opencode text-event field (part.text vs text) against a real capture during smoke

## [3ada0d3] Plan bookkeeping: phase-1-mvp checkboxes + task card AC evidence synced to commits

After commit 3ada0d3 closed Slice 1d, the executive plan (phase-1-mvp.md) still carried unchecked items for work already committed, and the master task card's AC evidence fields were all blank. Reconciled both against git history and a fresh test run (cargo test -p cast-agent = 35 tests green, exit 0).

Executive plan updates:
- Slice 1b `harness_from_kind` item -> [x], noting it shipped as HarnessKind::adapter() in main.rs:55-61 (d39b279).
- Slice 1c Tests block -> 4 of 5 items [x] with commit refs (fe555b4 supervisor matrix, 00c6ca1 interrupt, eeb4c18 finalize, d39b279 e2e). Only Manual smoke stays [ ] (outstanding).
- Slice 1d -> all 3 items [x] (3ada0d3): flake.nix:72-78, nix build reaches installPhase, docs README.

Task card (cast-agent-mvp.md) AC evidence filled for AC 1-7. Status stays in-progress: AC 3 + AC 6 automated portions are met, but each has a MANUAL smoke run (real opencode + Ctrl-C) that requires human attestation and is explicitly flagged OUTSTANDING. Per cue rules an agent cannot self-attest a human criterion, so the task cannot move to complete yet.

No git commit made: all changes are inside .cue/ which agents must not commit.

- **Found:** cargo test -p cast-agent: 35 tests across 9 suites (6 lib + 3 orchestrate + 4 opencode + 3 orchestrate-wait + 5 prompt + 5 rundir + 9 supervisor), exit 0
- **Found:** HarnessKind::adapter() is the implemented form of the planned harness_from_kind bridge (main.rs:55-61)
- **Found:** flake.nix:72-78 cast-agent buildRustPackage with nativeCheckInputs bash/coreutils/procps; nix build .#cast-agent reaches installPhase
- **Found:** AC 4 + AC 6 group-death assertions use live_group_members() ps poll, not killpg, because this container's pid 1 does not reap zombies (killpg returns 0 for dead-but-unreaped)
- **Decided:** Task remains status: in-progress, not complete, because AC 3 manual opencode smoke and AC 6 manual Ctrl-C smoke require human attestation and are outstanding
- **Decided:** Executive plan stays status: open because the Manual smoke checkbox is still unchecked
- **Decided:** Did NOT git commit: all edits are inside .cue/, which agents must not commit per the cue skill
- **Open:** Manual smoke run vs real opencode run --format json (tail -f stream.jsonl + Ctrl-C) — the only blocker for AC 3 + AC 6 human attestation and thus task completion
- **Open:** Verify opencode text-event JSON field (part.text vs top-level text) against a real --format json capture during the smoke

## [2fbba68] Removed CAST_AGENT_FAKE_CMD from release; interrupt tests moved to PATH shim

Committed 2fbba68. Acted on the Opus consultation (option 3) that confirmed CAST_AGENT_FAKE_CMD was a redundant trigger-seam that shipped in release on an attacker-influencable channel and — critically — silently made result.json.harness lie about what ran, breaking the audit-integrity guarantee cast-agent exists to provide.

Removed the env-var branch from main.rs entirely (unconditional harness.base_command() + headless_args() now). Converted tests/interrupt_test.rs spawn_agent helper to write a #!/bin/sh shim named 'opencode' (mode 0o755) into a tempdir and prepend that dir to PATH for the spawned cast-agent child. All 3 interrupt tests still green; full suite 35 tests green; clippy + fmt clean. Updated QA todo Step 8, task card AC 6 evidence, and phase-1-mvp plan to reflect the new mechanism (PATH-substitution is now the documented way to run a custom command under supervision).

PATH-shim gotchas handled per Opus: prepend (not replace) PATH so the shim resolves sh/printf/sleep/ps; shim is a real exec target (#! file, not exec-into-subshell) so $$ inside the script == the signaled child PID; tempdir under std::env::temp_dir() (Nix-safe); /bin/sh shebang known-present. process_group(0)/kill_on_drop(true) interaction unchanged (they act on whatever cast-agent spawned, PATH or literal).

- **Found:** CAST_AGENT_FAKE_CMD read was unconditional in main.rs:108 (no #[cfg(test)] gate) — present in release binary
- **Found:** The override was silent on every output channel: no stderr line, no result.json field, result.json.harness still reported 'opencode' (finalize.rs:146 unconditional)
- **Found:** The seam was redundant — orchestrate() already takes exe/args as a parameter seam and library tests (orchestrate_test.rs, supervisor_test.rs) already inject fakes through it without any env var
- **Found:** PATH-substitution exercises MORE of the real spawn path than the env branch did: the env branch skipped base_command() + PATH lookup entirely
- **Found:** All 3 interrupt tests still green after conversion; full suite 35 tests pass; clippy -D warnings clean; fmt --check clean
- **Decided:** Option 3 (delete env var; PATH-substitution shim) over option 1 (#[cfg(test)] — dead end, integration tests run release binary across process boundary where #[cfg(test)] is unset) and option 2 (keep + harden — treats symptom, keeps disease, still owes answer to 'why does shipping binary run sh -c $ENV')
- **Decided:** Shim is a #!/bin/sh file with mode 0o755 in a tempdir, single-process (no exec into subshell) so $$ == signaled child PID — preserves the existing group-death assertion semantics
- **Decided:** PATH is prepended not replaced so the shim script body can still resolve sh/printf/sleep/ps
- **Decided:** Updated QA todo Step 8 to document PATH-substitution as the supported way to run a custom command under cast-agent supervision (since release no longer has any override mechanism)
- **Decided:** Did NOT pursue keeping a hardened override (explicit --flag + result.json.substituted field): Opus confirmed the right move is to eliminate the surface, not manage it; can be re-added as a real feature later if a legitimate 'run custom command under supervision' use case emerges
- **Open:** Manual smoke run (AC 3 + AC 6 human attestation) still outstanding — unaffected by this refactor
- **Open:** If a future 'run custom command under supervision' feature is wanted, the minimum hardening set per Opus is: compile out of release by default (cfg feature), explicit CLI flag (argv not env — moves off attacker channel), result.json must tell truth (harness: 'override' + substituted: true + override_command), stderr warning, execvp not sh -c

## [2fbba68] Code review (Opus + GLM) of feat/cast-agent-mvp: B1 writer-join hang is the must-fix

Saved branch.diff (.cue/cast-agent-mvp/tmp/1785214540-2fbba68/branch.diff) and dispatched diff-reviewer-opus + diff-reviewer-glm in parallel. Both reviews saved as traces under .cue/cast-agent-mvp/trace/1785214540-2fbba68/. Both independently converged on the same critical finding: the writer-thread join at supervisor.rs:279-283 is UNBOUNDED and hangs supervise() forever in exactly the C1 escaped-grandchild scenario the code claims to handle, defeating both the C1 fallback and the recovery contract's 'always write result.json' guarantee. GLM reproduced it out-of-tree with an 8s watchdog; opus flagged it as Important (I1) rather than Blocking but described the identical mechanism. Severity disagreement noted (opus: important; glm: critical/blocking) but the fix is the same.

- **Found:** B1 (CONVERGENT, both reviewers): writer-thread join at supervisor.rs:279-283 is unbounded. When the stdout reader join times out (C1 case: a setsid/double-forked grandchild holds the stdout pipe write-end after group SIGKILL), tokio::time::timeout drops the JoinHandle which DETACHES (not cancels) the reader task. The reader still owns stdout_tx, so rx.recv() in the writer thread blocks forever, writer.join() never returns, spawn_blocking never resolves, supervise() never returns, result.json is never written. The code comment at supervisor.rs:276-278 asserts 'do not block indefinitely if a reader was abandoned' but the assertion is false. Reproduced by GLM out-of-tree.
- **Found:** Test coverage gap (CONVERGENT): NO test exercises an ESCAPED grandchild (setsid). All supervisor/interrupt tests keep grandchildren IN-GROUP (sleep 100 & sleep 100) so group SIGKILL reaches them, reader EOFs normally. This is why B1 survived AND events_from_disk fallback is entirely untested.
- **Found:** GLM I1 (high): the Err path out of supervise (cmd.spawn()? at supervisor.rs:180, child.wait() Err at supervisor.rs:237) propagates ? up through orchestrate to main which prints 'run failed' and exits EXIT_USAGE(2) WITHOUT writing result.json. Conflates runtime I/O failure / missing-harness-binary with usage error. Violates 'every run yields a verdict'.
- **Found:** Opus I2 / GLM I2 (convergent mechanism, divergent severity): prompt-writer task errors fully swallowed (opus) and reader-task death not observable in control plane (glm). Opus rated low; glm rated medium. Both about silent error/panic paths.
- **Found:** GLM N5: Interrupted records the WRONG thing in exit.signal — set to the SIGINT/SIGTERM cast-agent received, not the child's actual terminating signal. Inconsistent with TimedOut (which correctly records SIGKILL). A recovery reader misinfers the child's death cause.
- **Found:** Both confirmed N6: live_group_members ps-based workaround is SOUND — does not mask a bug. Operational caveat: deployments should run a reaping init (tini/dumb-init) as pid 1.
- **Found:** Both confirmed N7: PATH-shim test mechanism is SOUND — exercises real execvp path, CAST_AGENT_FAKE_CMD fully gone, no attacker surface.
- **Found:** Nits both: dead code EndReason::child_signal; tracing/tracing-subscriber deps declared but never used (no subscriber init, all diagnostics via eprintln!); GLM N8 wants a comment on the trap-SIGTERM test explaining ignored-signal-disposition inheritance across fork/execve.
- **Decided:** Saved both reviews as traces: .cue/cast-agent-mvp/trace/1785214540-2fbba68/code-review-opus.md + code-review-glm.md
- **Decided:** Did NOT fix anything yet — awaiting user instruction on whether to address B1 + the test gap now or defer
- **Open:** Pre-merge must-fix: B1 writer-thread unbounded join (supervisor.rs:279-283). Two fix options from GLM: (a) bound the writer join symmetrically with tokio::time::timeout, or (b) drop the writer JoinHandles without joining (cleaner, matches 'non-blocking backstop' philosophy; per-line flush already preserved durability). Plus add the escaped-grandchild regression test that would have caught B1 AND cover events_from_disk.
- **Open:** Address GLM I1: error-path-no-verdict + exit-2 misclassification. Map supervisor Err to an Outcome (Crashed/Failed) and still write result.json; give spawn/IO failures a distinct non-usage exit code.
- **Open:** Address GLM N5: Interrupted exit.signal records the trigger not the child disposition — decide field semantics.
- **Open:** Decide policy on the medium/low items: reader-death-in-control-plane (I2), prompt-write error swallowed (opus I2), unused tracing deps.

## [7ac2ff7] Slice R1 done: bounded writer join fixes C1 escaped-grandchild hang (B1)

Implemented Slice R1 of the review-fixes plan via TDD. The convergent BLOCKING finding from both code reviews (unbounded writer-thread join at supervisor.rs:279-283) is fixed and regression-tested. Committed.

- **Found:** RED repro required care: an instant child-exit + our own group SIGKILL RACES the grandchild's setsid() and reaps it before it escapes, masking B1. Deterministic repro needs (a) a two-statement grandchild body so the shell does not exec-optimize away the fd1-holder, and (b) a brief `sleep 0.3` in the direct child so setsid() completes and the grandchild firmly escapes the group before the kill fires.
- **Found:** Single-command `setsid sh -c 'sleep 30'` gets exec-optimized and does NOT reliably hold the pipe; `setsid sh -c 'sleep 30; echo x'` does.
- **Found:** Confirmed RED: unfixed supervise() hangs the full 8s test watchdog.
- **Decided:** Fix = abort() the reader task on the join timeout (not just drop its JoinHandle) so the abandoned task releases its line-writer Sender, unblocking the writer thread's recv(); PLUS bound the writer-thread join with timeout(READER_JOIN_TIMEOUT) as a defensive backstop. Used tokio::select! over `&mut task` so the handle stays owned/abortable.
- **Decided:** Added pkgs.util-linux to cast-agent nativeCheckInputs in flake.nix (setsid lives there, needed for the regression test under nix build).
- **Decided:** events fall back to events_from_disk after abort (per-line flush already made the on-disk trace authoritative).
- **Open:** R2: error-path verdict (supervise Err -> Crashed + still write result.json, distinct exit code, not usage-2)
- **Open:** R3: exit.signal reflects child disposition + new interrupt_signal field
- **Open:** R4: cleanup nits (dead code, unused tracing deps, error logging, duration_ms u64, robust extract_result)
- **Open:** Manual opencode smoke still outstanding (unrelated to review fixes)

## [e22b370] Slice R2 done: spawn/supervise failure yields a crashed verdict (GLM I1)

Implemented Slice R2 via TDD. Runtime failures (missing harness binary, post-spawn I/O error) now produce a result.json instead of exiting 2 with no receipt. Committed.

- **Decided:** Added EndReason::SpawnFailed(String) + SuperviseFailed(String); both classify to Outcome::Crashed with ExitInfo{code:None,signal:None} and exit code 5.
- **Decided:** supervise() returns Ok(SpawnFailed) on cmd.spawn() error rather than propagating; orchestrate maps a supervise Err to SuperviseFailed and STILL writes result.json.
- **Decided:** Added optional Verdict.error_detail (serde skip_if none) carrying the spawn/supervise failure message.
- **Decided:** main.rs Err arm is now pre-run-setup-only (persist-before-spawn write failures) -> exit 2 'setup failed'.
- **Open:** R3: exit.signal reflects child disposition + interrupt_signal field
- **Open:** R4: cleanup nits

## [789f300] Slice R3 done: exit.signal reflects child death; new interrupt_signal field (GLM N5)

Implemented Slice R3 via TDD. result.json exit now reflects the child's real disposition for both timed_out and interrupted; the trigger signal moved to a separate interrupt_signal field. Committed.

- **Decided:** EndReason::TimedOut and Interrupted now carry Option<ExitStatus> child_status, filled by the post-teardown re-wait (replacing the discarded `let _ = child.wait()`).
- **Decided:** Interrupted's signal field renamed to `trigger`; classify derives exit from child_status via exit_info_from_status (code if exited, else s.signal(), else fallback SIGKILL).
- **Decided:** Verdict gained optional interrupt_signal (serde skip-if-none) = the trigger; exit.signal = child death.
- **Decided:** Interrupt test contract: plain-sleep child honoring graceful stop dies by SIGTERM; trap-TERM child dies by SIGKILL after grace; trigger recorded separately.
- **Open:** R4: cleanup nits (dead code EndReason::child_signal, unused tracing deps, error logging, duration_ms u64, robust extract_result, comments)
- **Open:** recovery-outcome-contract.md note needs a follow-up update for the new exit/interrupt_signal semantics (.cue, not committed)

## [aa2e6c8] Slice R4 done: cleanup, unused deps, error logging (review nits)

Implemented Slice R4 (final slice of review-fixes plan). All review-actionable items now addressed. 38 tests green, clippy -D warnings clean, fmt clean. Committed.

- **Decided:** Removed dead EndReason::child_signal + unused ExitStatusExt import in supervisor.rs.
- **Decided:** Dropped tracing + tracing-subscriber deps from Cargo.toml (no subscriber ever installed; eprintln! everywhere).
- **Decided:** Prompt-writer and run-log line-writer errors now logged via eprintln! instead of swallowed.
- **Decided:** Verdict.duration_ms narrowed u128 -> u64.
- **Decided:** opencode::extract_result uses rev().filter().find_map() so a malformed trailing text event falls back to an earlier valid one.
- **Decided:** Added N8 comment (ignored-signal disposition inherited across fork/execve) + I4 reader-task-panic-free invariant comment.
- **Open:** recovery-outcome-contract.md note update for exit/interrupt_signal semantics (.cue, human-committed)
- **Open:** Manual opencode smoke (AC 3 + AC 6 human attestation) still outstanding
- **Open:** Optional: run nix build .#cast-agent to confirm util-linux nativeCheckInput works in sandbox

## [aa2e6c8] cast-agent MVP complete: manual QA attested, all ACs met, task closed

User attested Manual QA passes for the cast-agent MVP (real `opencode run --format json` smoke: live `tail -f stream.jsonl` streaming confirmed; Ctrl-C interrupt confirmed yielding `result.json(outcome=interrupted)` with no orphaned processes; also covers the previously-UNVERIFIED opencode `text` event `part.text` vs top-level `text` field shape). With the human-attested criteria satisfied, all seven Acceptance Criteria now have evidence. Reconciled the cue bookkeeping: filled AC 3 + AC 6 human-attested evidence in the master task card, transitioned the task status in-progress -> complete, checked the last outstanding Manual smoke checkbox in the phase-1-mvp executive plan (status open -> complete), and fixed two stale R3 RED/GREEN checkboxes in the review-fixes plan (the work was committed at 789f300 but the boxes were never flipped). No git commit made — all edits are inside .cue/, which agents must not commit.

Final verified state: cargo test -p cast-agent = 38 tests green across 7 suites (exit 0); cargo clippy -D warnings clean; cargo fmt --check clean. The review-fixes plan (R1-R4) is fully implemented and verified against source; the phase-1-mvp plan has zero unchecked items remaining.

One doc follow-up remains (in .cue/, human-committed): note/recovery-outcome-contract.md should be updated for the new exit/interrupt_signal semantics introduced in R3 (exit.signal now reflects the child's actual death disposition; the trigger moved to a separate interrupt_signal field). This is documentation-only; the code and tests already reflect the new contract.

- **Found:** User attestation received: manual QA passes (real opencode smoke + Ctrl-C interrupt) — satisfies the human-attested portions of AC 3 and AC 6
- **Found:** Manual smoke also closes the previously-open opencode text-event field verification (part.text vs text) — extractor tolerates both shapes
- **Found:** Verification re-run: 38 tests green, clippy -D warnings clean, fmt --check clean; all 4 review-fixes commits present (7ac2ff7/e22b370/789f300/aa2e6c8)
- **Decided:** Task cast-agent MVP -> status: complete (all 7 ACs evidence-filled; human attestation for AC 3 + AC 6 received in-conversation)
- **Decided:** phase-1-mvp executive plan -> status: complete (zero unchecked items; Manual smoke human-attested)
- **Decided:** Fixed stale R3 RED/GREEN checkboxes in review-fixes.md to match the committed work (789f300)
- **Decided:** Did NOT git commit: all edits inside .cue/, which agents must not commit per the cue skill
- **Open:** Doc-only follow-up (in .cue/, human-committed): update note/recovery-outcome-contract.md for the R3 exit/interrupt_signal semantics change

## [aa2e6c8] Closed stale recovery-outcome-contract note with R3 exit/interrupt_signal update

Addressed the stale design note note/recovery-outcome-contract.md for the R3 contract change. The note is a historical design-conversation record, so rather than rewriting the trail I: (1) added a prominent "UPDATE (post-implementation, R3)" banner at the top documenting the current truth — exit.signal/code now ALWAYS reflect the child's reaped death disposition, the trigger moved to a separate interrupt_signal field (interrupted-only), and error_detail is spawn/supervise failures only (in-stream detection deferred); (2) annotated the three concrete stale spots inline — the draft result.json JSON example (added interrupt_signal field + clarifying note), the Ctrl-C teardown Sequence (now shows re-wait to reap + the exit/interrupt_signal split), and the result.json fields list (added interrupt_signal + error_detail, marked exit as "child's reaped disposition"); (3) closed the note — its content has dissolved into the now-complete spec/plan/task per the cue note lifecycle, and corrected the stale intro ("not yet folded into spec/plan"). No git commit — .cue/ edits only.

- **Decided:** Treated the note as a historical design trail, not current truth: added a superseding R3 banner at top + inline annotations on the 3 stale spots rather than rewriting history
- **Decided:** Closed the note (status in-progress -> closed): content dissolved into the now-complete spec/plan/task per the cue note lifecycle
- **Decided:** Corrected the stale intro claim 'not yet folded into spec/plan' to reflect it has been folded in and shipped

## [2b25028] Added --agent flag to cast-agent (todo closed, committed 2b25028)

Implemented the deferred `--agent <name>` flag on `cast-agent run` via TDD, closing the agent-flag-mvp-dependency todo. The Layer-2 opencode task drop-in in cue-plugins needs this to faithfully forward `subagent_type` to the correct persona; without it the model believed it delegated to a read-only persona while actually launching the default full-access agent.

- **Found:** Harness::agent_args already existed as a None-returning trait default; only OpenCode override + CLI wiring were missing
- **Decided:** OpenCode::agent_args returns Some([--agent, name]) — passthrough; opencode itself validates unknown names (surfaces as failed/crashed verdict, never a silent default fallback)
- **Decided:** main.rs rejects --agent with EXIT_USAGE(2) if the harness's agent_args returns None, rather than silently dropping the arg (the quiet permission regression the todo warned about)
- **Decided:** Reworded doc comment to satisfy clippy::doc_lazy_continuation
- **Open:** Full harness build + real opencode smoke of --agent explore not run in this session (human/devshell)
- **Open:** Plugin-side counterpart (cue-plugins task.ts:41-42) still needs to pass --agent through to cast-agent

