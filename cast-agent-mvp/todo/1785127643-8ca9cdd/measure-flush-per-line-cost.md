---
status: complete
priority: normal
refs:
- .cue/cast-agent-mvp/trace/1785127643-8ca9cdd/opus-verification-of-design-review.md
- .cue/cast-agent-mvp/doc/streaming-supervisor-design.md
- .cue/cast-agent-mvp/trace/1785127643-8ca9cdd/runlog-writer-flush-benchmark.md
---
> **RESOLVED.** Benchmark (200k lines): `tokio::fs` + per-line flush ~86k
> lines/sec; sync `std::fs::BufWriter` on a blocking thread + per-line flush
> ~2.12M lines/sec (~24x). C3 confirmed: the `tokio::fs` `flush().await` is a
> near no-op (already unbuffered through spawn_blocking). DECISION: sync
> `BufWriter` on a dedicated blocking thread, flush per line (coalesces
> line+\n into one write(2); sub-ms tail latency; SIGKILL partial-survival
> PASS). Durability scope stays "survives process death, not host crash" — do
> NOT regress to fsync-per-line. See trace `runlog-writer-flush-benchmark.md`.

# TODO: Measure flush-per-line cost before finalizing the run-log writer

**Priority: normal. Do during/before Slice 1c when the run-log writer is
implemented; resolve the open "tokio::fs vs sync BufWriter" choice WITH a
measurement, not a guess.**

## Why (findings C3 + C4, confirmed by two Opus passes)

The streaming-supervisor design flushes the run-log per line
(`doc/streaming-supervisor-design.md:155-160`): for each stdout line it does
`write_all(line)` + `write_all("\n")` + `flush().await`. This is the live
`tail -f` observability + crash-durability substrate, so per-line flush is a
hard requirement — BUT:

- **C4:** each line is multiple `write(2)` syscalls, and under `tokio::fs` each
  is a `spawn_blocking` round-trip (task hop). A chatty harness
  (opencode/claude verbose/streaming) can emit hundreds of small
  `text`/`reasoning` delta events per second. That overhead is plausibly hot.
- **C3 nuance (from the verification pass):** a `tokio::fs::File` write already
  goes through `spawn_blocking` to an unbuffered `std::fs::File::write`, so the
  per-line `flush().await` may be a **no-op on top of already-written bytes** —
  meaning we may be paying the task-hop cost for zero durability benefit.
- **C3 scope discipline:** `flush()` is `write(2)` to the kernel page cache, NOT
  `fsync`. It correctly survives cast-agent PROCESS death (SIGKILL) but NOT host
  crash / power loss. This is the CORRECT behavior for our threat model — do NOT
  let anyone "harden" it into `fsync`-per-line (catastrophic for a chatty
  stream, and buys durability the disposable trace does not need). Record the
  claim precisely: "survives cast-agent process death, not host crash."

## What to measure

1. Throughput of the run-log writer under a high-line-rate synthetic stream
   (e.g. a fake harness printing N thousand small JSON lines as fast as
   possible), comparing:
   - `tokio::fs` + per-line `flush().await` (the sketch as written);
   - sync `std::fs::BufWriter` + explicit `flush()` on a short interval or on
     event boundaries (e.g. `step_finish`), on a blocking thread /
     `spawn_blocking`.
2. Confirm the live-`tail -f` latency for each option (the flush interval must
   still make lines visible promptly — the observability requirement).
3. Confirm partial-log-survival still holds (kill mid-stream, assert the log
   has all lines written up to the kill point) for whichever option is chosen.

## Done when

- A measured comparison recorded (lines/sec + tail latency) for at least the two
  writer strategies above.
- A decision recorded for the run-log writer (per-line flush vs
  buffer-flush-on-boundary), justified by the numbers.
- The durability claim in the design docs scoped to "survives process death,
  not host crash" so nobody later regresses it into fsync-per-line.
