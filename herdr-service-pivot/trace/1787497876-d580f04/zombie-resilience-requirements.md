---
refs: ../../master/task/herdr-service-pivot.md
status: open
---
# Zombie-resilience requirements for the herdr service pivot

Date: 2026-08-25. Design input from the cast-rust-compilation-error
investigation (palekiwi workspace, trace/1787504610-fee5b72/, Opus
adjudication v3). The user asked: does this pivot solve the recurring
zombie-PID-exhaustion failure, and what should be added so it does.

## Background (one paragraph)

Every incident to date: opencode (or its wrapper) sits at PID 1 and
never reaps. opencode's snapshot subsystem leaks exited git children
under a live session (~6/sec during cold-start catch-up; Rust worktree
churn is the asymmetry driver), saturating pids.max (512 default /
1024 in this workspace) in ~3 minutes from a cold container. Zombies
are unreapable by anyone but their parent; cgroup2 is mounted ro
(verified) so pids.max cannot be raised in place; the only recovery is
a container restart. rustc/bash failures are downstream canaries.

## Does the pivot solve it? Conditionally yes - and it is the right shape

The pivot's real contribution is RECOVERY GRANULARITY. Today, opencode
IS the container's purpose: its zombies persist for the container's
lifetime, so leak == container death. Under the service model, opencode
sessions are restartable children of a long-lived service: a leaking
session can be killed, its zombie backlog releases to the reaper, and
the container keeps running. Accumulation becomes session-scoped
instead of container-fatal.

Key subtlety (verified against zombie semantics): a reaping init at
PID 1 does NOT reap zombies held by a LIVE opencode session (they are
opencode's children, not orphans). The backlog clears when the session
exits. So the full fix is exactly the pivot's shape: reaping PID 1
(releases backlog at session exit) + cheap session restart (bounded
exposure) + telemetry (know when to restart).

## Verified gaps in the current design materials

1. herdr upstream (~/code/vendor/herdrdev/herdr) has NO
   PR_SET_CHILD_SUBREAPER and NO waitpid(-1) anywhere. It reaps only
   the children it knows (pty/session pids: vendor/portable-pty
   SIGCHLD handling, src/app/input/terminal.rs:934, per-child waitpid
   in ghostty-vt Command.zig). Orphaned subtrees below a session (an
   opencode tree killed non-exhaustively) would still accumulate under
   a herdr PID 1. herdr alone is NOT a reaping init.
2. The card specifies "herdr server under an init at PID 1" but the
   composed CMD is `nix develop <global>#<shell> -c herdr server`.
   What ACTUALLY becomes PID 1 is unverified: if `nix develop -c`
   forks-and-waits rather than execs, PID 1 is the nix process - a
   non-reaping parent again, init never reached. Spike must verify
   with ps -p 1 inside the service container.

## Requirements to fold into the pivot design

- R1 (cast, ship NOW, independent of pivot): add `--init` to docker
  run args (crates/cast/src/docker/args.rs build_run_args). tini as
  PID 1 reaps every reparented process in today's per-agent
  containers. Bounds any leak to the opencode process lifetime; would
  have prevented every incident to date from being container-fatal.
- R2 (pivot): the "init at PID 1" slot must be a REAPING init
  explicitly (docker --init, or tini wrapping the composed CMD, or an
  init that execs). Resolve the nix-wrapper-PID-1 question (gap 2) in
  the bring-up spike; if nix forks, put tini in front.
- R3 (herdr upstream proposal): herdr server as an agent-harness
  service should set PR_SET_CHILD_SUBREAPER and sweep waitpid(-1,
  WNOHANG) on SIGCHLD/idle, reaping orphaned session descendants it
  did not spawn. Natural fit for herdr's agent lifecycle tracking.
  Until upstream takes it, R2 covers the container.
- R4 (pivot, operations): session restart is the zombie-recovery
  action. Surface pids.current + zombie count (ps -eo stat= | grep -c
  '^Z') in service status (`cast status` / mux client), warn at a
  threshold, and - leveraging herdr's idle/working/blocked/done
  tracking - identify the leaking session and offer/perform restart.
  Agent-side detection recipe for the docs: on EAGAIN / fork-retry /
  rustc WouldBlock ICE, count zombies; hundreds means restart the
  session, not the container.
- R5: keep pids_limit as a backstop only. Raising it changes
  saturation time (~167s at 1024 observed), not the outcome.

## What this does NOT fix (accepted residual)

The leak source itself - opencode/Bun not reaping snapshot git
children under a live session - keeps leaking; it stays bounded by
session lifetime and is being tracked for an upstream opencode issue
(separate thread in the palekiwi task context). The opencode bash-tool
orphan path (detached:true + child-only kill on timeout/cancel) also
remains, but with R1/R2 its orphans are reaped harmlessly.
