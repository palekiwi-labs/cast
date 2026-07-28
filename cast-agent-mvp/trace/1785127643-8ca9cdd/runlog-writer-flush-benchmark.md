---
refs:
- .cue/cast-agent-mvp/todo/1785127643-8ca9cdd/measure-flush-per-line-cost.md
- .cue/cast-agent-mvp/doc/streaming-supervisor-design.md
---
# TRACE: run-log writer flush-per-line benchmark (findings C3 + C4)

Resolves the open "tokio::fs vs sync BufWriter" writer choice in
`todo/.../measure-flush-per-line-cost.md` and `doc/streaming-supervisor-design.md:155-160`
WITH a measurement, not a guess.

## Setup

- Disposable standalone crate (NOT committed, NOT wired into cast crates):
  `/tmp/nix-shell.UFUe4V/opencode/flushbench` (throwaway scratch).
- tokio resolved to **1.53.1** (cast pins `1.52.3`; cargo picked the latest
  compatible patch in an isolated lockfile). Same 1.x line; the fs/io write
  path — `spawn_blocking` -> unbuffered `std::fs::File::write` — is identical,
  so numbers are representative. edition 2024, `--release`, rustc 1.94.1.
- Built/run in cast's devshell (`nix develop /home/pl/code/palekiwi-labs/cast`).
- Workload: N=200,000 synthetic small JSON delta lines (~123 bytes/line,
  `message.part.updated` text/reasoning deltas), emitted as fast as possible
  by a single writer. This is ~100-1000x more chatty than the real harness
  (opencode/claude emit "hundreds of events/sec"), i.e. a deliberate stress.

## Throughput (lines/sec)

    sync std::fs::File   (per-line, unbuffered)        216.3 ms      924,642 lines/sec
    sync BufWriter       (per-line flush)               94.2 ms    2,122,042 lines/sec
    sync BufWriter       (flush every 16 lines)         12.0 ms   16,708,256 lines/sec
    tokio::fs            (per-line flush) [the sketch] 2316.0 ms       86,355 lines/sec
    tokio::fs            (per-line, NO flush)          2254.9 ms       88,697 lines/sec

## Tail -f visibility latency (emit -> byte visible in file, microseconds)

    tokio::fs per-line flush:    p50=617   p99=1992   max=2224
    tokio::fs per-line NO flush: p50=628   p99=2003   max=2081

(Latency floor is dominated by the 200us watcher poll interval + `metadata()`
stat cost; treat as "sub-millisecond either way". The point is flush vs
no-flush are statistically identical.)

## Partial-log survival (real SIGKILL of a child mid-stream)

Child process writes JSON lines forever with a sync `BufWriter`, flushing every
16 lines; parent SIGKILLs it after 150 ms, then re-reads the file:

    file bytes          = 47,239,466
    complete JSON lines = 1,559,696
    torn trailing line  = 0   (allowed <=1: kill can land mid-write(2))
    contiguous seq 0..N = true
    RESULT              = PASS — all *flushed* lines up to the kill survived
                          cast-agent process death (SIGKILL).

## Findings

- **C3 CONFIRMED — the `flush().await` is a near no-op under `tokio::fs`.**
  86,355 (flush) vs 88,697 (no flush) lines/sec = ~2.6% difference. A
  `tokio::fs::File` write already goes through `spawn_blocking` to an
  *unbuffered* `std::fs::File::write`, so each `write_all` has already hit the
  kernel page cache; the extra `flush().await` adds a third task hop per line
  for essentially zero durability or visibility benefit. Tail latency for
  flush vs no-flush is identical (p50 617us vs 628us), proving the bytes are
  already visible before the flush.

- **C4 CONFIRMED — `tokio::fs` per-line is the slowest by ~24x.** The sketch
  does 3 `spawn_blocking` round-trips/line (write line, write "\n", flush).
  At 86k lines/sec it is *still* far above the real ~hundreds/sec workload, so
  it would not bottleneck correctness — but it burns 2-3 blocking-pool task
  hops per line, contending with the rest of the runtime's blocking work, for
  no gain.

- **`BufWriter` per-line flush is 2x faster than an unbuffered `File`** (2.1M
  vs 0.92M) purely by coalescing the `line` + `"\n"` into a single `write(2)`
  instead of two, while preserving one `write(2)` per line = identical
  process-death durability.

- **Boundary flush (every 16) is 8x faster still (16.7M)** but trades
  durability: up to `FLUSH_BOUNDARY-1` lines sit in the userspace buffer,
  never `write(2)`'d, and are LOST on process death. Survivors remain intact &
  contiguous (verified), but this weakens both the process-death guarantee and
  tail-f promptness (visibility delayed by up to a batch).

## RECOMMENDATION

**Use a synchronous `std::fs::BufWriter` on a dedicated blocking thread (or a
single `spawn_blocking`), fed lines over a channel from the async stdout
reader, and `flush()` PER LINE.**

Justification by the numbers:
1. ~2.1M lines/sec — 24x the `tokio::fs` sketch (86k) — with a single `write(2)`
   per line (line+newline coalesced). The real harness rate (~hundreds/sec) is
   nowhere near either ceiling, but this removes per-line `spawn_blocking`
   task-hop pressure on the shared blocking pool entirely (one long-lived
   thread instead of 3 hops/line).
2. Per-line flush = one `write(2)` per line to the page cache -> survives
   cast-agent PROCESS death (SIGKILL); partial-survival test PASSED.
3. Sub-millisecond tail-`f` visibility — satisfies the live-observability
   requirement.
4. No `fs` tokio feature needed; the writer is plain sync `std::fs`.

Acceptable lighter-touch alternative if we want to keep `tokio::fs`: **drop the
per-line `flush().await`** (C3 shows it buys nothing — 89k vs 86k lines/sec and
identical tail latency). This is a one-line change to the sketch and stays
correct, but still pays 2 task hops/line and consumes the blocking pool, so the
dedicated-thread `BufWriter` is strictly better.

Do NOT use boundary/interval flush for the MVP: per-line flush is trivially
affordable (2.1M capacity vs hundreds/sec load) and gives zero-loss on process
death; boundary flush only trades that durability + promptness away for speed
we do not need.

## DURABILITY CLAIM — scope discipline (do not regress)

`flush()` here is `write(2)` to the **kernel page cache**, NOT `fsync`. The
run-log therefore:
- **SURVIVES** cast-agent process death (SIGKILL / crash of our process) —
  verified by the partial-survival test.
- **DOES NOT survive** host crash / power loss.

This is the CORRECT threat model for a disposable run-log. Do NOT "harden" this
into `fsync`-per-line: it would be catastrophic for a chatty stream and buys
durability the disposable trace does not need.

## Reproduce / cleanup

Crate at `/tmp/nix-shell.UFUe4V/opencode/flushbench` (scratch, disposable —
delete freely; nothing committed, cast production source untouched).
Run: `nix develop /home/pl/code/palekiwi-labs/cast -c cargo run --release`
(from the crate dir), or `./target/release/flushbench`.
