---
title: Pivot cast to a herdr-managed container service model
status: in-progress
priority: low
kind: research
tag: 0.3.0
---

# Pivot cast to a herdr-managed container service model

## Problem context

Today `cast` models the world as _per-agent containers_: `cast run opencode`,
`cast shell opencode`, and `cast exec opencode <cmd>` each provision a
container tied to one agent identity. The proposed pivot redefines the
project container as an **ongoing service** that can be started and stopped
independently of agent sessions, with agent sessions freely created and
attached inside it.

The service would be managed by `herdr`, a modern terminal multiplexer
built for agent-harness workflows (server/client split, agent lifecycle
tracking: idle/working/blocked/done). herdr runs _inside_ the container as
a headless service (`herdr server` under an init at PID 1); the host drives
it with one-shot `docker exec ... herdr <subcommand>` calls and attaches the
TUI only to observe. herdr's richer harness features are not needed for this
first step but are expected to matter later in the agent sandbox.

A general (not cast-specific) implementation guide for "container as herdr
service" already exists and has been filed as the reference trace in this
task: `trace/.../herdr-as-a-service.md`.

## Objectives

1. ~~Determine whether current `cast` features (notably `cast exec`) allow
   testing this approach today~~ — answered: no; `cast exec` always
   provisions a fresh container and cannot target a running one.
2. ~~Define minimal PoC adaptations on a cast branch~~ — reframed after
   operator discussion: the question is now **thorough redesign of cast
   around the service model vs minimal PoC**. Operator leans redesign;
   see "Direction (operator input)" below.
3. Map integration points between the herdr service model and cast's
   existing machinery (image, Nix harness provisioning, ports, volumes).
4. ~~Version target~~ — resolved (operator decision, 2026-08-25):
   explicitly deferred to **0.3.0**; tagged out of the 0.2.0
   release. The pivot's API surface is not yet decided and its scope
   exceeds what 0.2.0 can freeze.

## Early orientation findings (session start)

- `cast exec` **cannot** target a running container: `dev::exec` always
  starts a fresh one (`docker run --rm`, unique per-invocation name token).
  The herdr pattern instead requires repeated `docker exec` into one
  long-lived named container. This is the primary gap.
- The dev image is already harness-free (`Dockerfile.dev`: "No harness
  binaries are baked in"); agent identity in cast survives only in the
  three hardcoded clap enums (`RunAgent`/`ShellAgent`/`ExecAgent`), the
  `Agent` trait impls, and per-agent port allocation — while the actual
  selection mechanism (`global_shell`) is already a free string, and a
  `universal` devShell with all harnesses exists.

## Direction (operator input, session 2)

The operator is convinced cast needs a rework and questions whether a
minimal PoC is even the right frame:

- After the pivot to global-devshell harness provisioning
  (`~/.config/cast/nix/flake.nix`), hardcoded agent-specific commands
  (`cast run opencode`) no longer make sense. Goal: generalize cast as an
  agent-agnostic sandbox. The `Agent` trait and hardcoded mounts are a
  derived issue, not primary.
- Multiplexing should become a first-class citizen (operator works in tmux
  full-time); herdr is the chosen modern alternative.

## Key research questions

- Q1 (reframed): redesign vs minimal PoC — recommend a strategy. Agent
  analysis: the codebase supports the redesign thesis (image already
  harness-free; agent identity survives only in the CLI enums, `Agent`
  trait impls, and per-agent port allocation; `global_shell` is already a
  free string; herdr is already in `commonInputs` of the global flake and
  ships 20+ agent manifests vs cast's 3 hardcoded).
- Q2 (resolved by operator): herdr does not care where agent binaries come
  from as long as they are on PATH; provisioning them is the user's/nix's
  responsibility.
- Q3 (resolved, verified in code): the MCP server is already a standalone
  host HTTP process (`cast mcp start`); `cast run` only injects
  `CAST_MCP_URL=http://host.docker.internal:{port}/mcp`. The claim in
  `docs/concepts.md` that `cast run` starts the server is stale. No
  redesign blocker; minor doc fix needed.
- Q4 (resolved by operator): use the fresh upstream clone at
  `~/code/vendor/herdrdev/herdr/` as herdr source of truth (fork at
  ogulcancelik is obsolete; the reference trace was verified against
  upstream anyway).
- Q5 (deprioritized): product/version target, aspirational v0.2.0.

## New design questions (for the redesign track)

- D1: Command surface — which verbs own container lifecycle (service
  start/stop/status), how attach works, and whether cast wraps herdr
  subcommands or herdr remains the direct session interface.
- D2: herdr binary at PID 1 — baked into the dev image (breaks the
  harness-free principle; herdr is infra, not a harness) vs nix-provisioned
  wrapper as CMD (keeps image clean; adds startup latency and nix-daemon
  dependency at service start).
- D3: Port model — deterministic per-agent ports become obsolete if there
  is one service container per workspace; what still needs host ports?
- D4: Migration — semantics of `run`/`shell`/`exec` invert from "fresh
  container per invocation" to "target the running service"; deprecation
  path.

## Empirical unknowns (de-risk spike candidates)

- Does herdr agent detection work on harnesses launched through
  `nix develop -c <agent>` (devshell wrapper between herdr and the binary)?
- herdr server lifecycle under `docker stop`/restart with cast's
  volume/nix-daemon regime; state persistence across restarts.
- Alt-screen/scrollback caveats with opencode inside herdr panes.
- herdr resource profile as a long-lived PID-1 service.

## Design decisions (collaborative, session 4)

Approved:

- Verb style: bare top-level verbs (`cast up`, `cast down`, `cast attach`)
  — the service is the primary object; auxiliary groups (nix-daemon, mcp,
  config) keep noun prefixes.
- Session ops via passthrough, not cast-side wrapping: cast must not
  rebuild an agent taxonomy.

Operator objection adopted (advances D1): naming the vendor in the API
(`cast herdr ...`) repeats the harness-hardcoding mistake. Working
resolution: role-named passthrough verb (`cast mux <args>` — final name
open) forwarding to the configured multiplexer's client inside the
container; multiplexer selected in config (default herdr). Internally a
small Multiplexer abstraction (server cmd for container CMD, client cmd
for attach, liveness check) — symmetry with the existing Agent trait.
Vendor names may appear as config defaults, never in the API surface or
semantics. The service CMD becomes composed:
`nix develop <global>#<shell> -c <mux server cmd>`.

Deferred as out of scope here: service devShell selection (config-driven;
reworked on the build-repo-defined-shells branch).

Open in discussion (operator input from sessions 4-6):

- Surface follows docker convention (operator): `cast up` starts the
  project's default container; `cast exec <cmd>` runs a command in it,
  with flags `--headless` (no TTY) and `--mux` (run inside the
  multiplexer). `--raw` (skip devshell wrapping) carries over from the
  existing exec.
- `cast attach` REJECTED by operator — no convincing case. Consequence:
  attaching the multiplexer UI is simply `cast exec <mux-binary>` (e.g.
  `cast exec herdr`), which the user types themselves. cast never names
  a multiplexer in its API.
- `--session` flag REJECTED by operator: session navigation belongs
  inside the container (enter it, use the mux there), not in cast's API.
  Retracted.
- `--mux` semantics to pin down: run the command inside a mux pane
  (tracked, attachable, survives disconnect) vs bare docker exec (bound
  to the caller's terminal, invisible to the mux). Spike question:
  whether a generic `pane run` plus herdr's passive agent detection
  gives lifecycle tracking for free, or whether explicit `agent start`
  semantics are required (the latter risks reintroducing kind taxonomy
  into cast).
- `shell` survives as convenience (operator values it): effectively
  `cast exec bash`, devshell-wrapped.
- `run` deferred to spike evidence.
- Container naming: drop agent slot; port shifts to per-workspace
  (CRC32 of absolute path alone), addressing org/repo subdir collisions.
  Open problem captured as todo `port-vs-identifier.md`: the number
  still means a port, so two services in one workspace would collide —
  options include seeding with the service name, explicit `--port` on
  `cast up`, or decoupling identity from ports entirely.
- MCP: keep injecting CAST_MCP_URL only (no auto-start) for the spike.



## Remaining scope / completion criteria

Execution model (operator decision, session 3): spike is developed on
branch `spike/herdr-service-pivot` (worktree
`./worktrees/spike-herdr-service-pivot`, based on master) by modifying
`cast` itself on that branch - no separate binary target. Rationale: the
branch is isolated, and code written directly in `cast` transfers to
production when the pivot graduates. The operator runs docker-requiring
steps by executing the branch's cast via `nix run` in real projects and
reports findings back. Production cast remains in daily use throughout.

The command surface is NOT yet decided: the operator requires a
collaborative API/UX design discussion before implementation begins.

Verified mechanics available for the spike (explore report, session 3):

- `cast run opencode` container argv:
  `nix develop /home/{user}/.config/cast/nix#opencode -c opencode`
- Global flake mounted wholesale rw into the container
  (`universal/volumes.rs`: `~/.config/cast/nix:/home/{u}/.config/cast/nix:rw`)
- DevShell selection: `global_shell` config if set, else agent name
- `cast shell` already uses `docker exec -it` into the stable-named
  container - the exec-into-existing pattern has precedent
- Stable container name `cast-{agent}-{basename}-{port}` is the existing
  deterministic, re-attachable identifier
- Headless `cast run` already runs `nix develop -c` without a TTY -
  strong evidence the service CMD
  `nix develop <global>#universal -c herdr server` is viable
- Breakage candidates: accept-flake-config=false (benign for herdr),
  daemon socket via ro /nix mount (existing mechanism), shellHook TTY
  (global flake's hook only echoes; benign)

This task is done when:

1. API/UX design agreed; spike implemented on the branch and exercised
   by the operator in at least one real project (bring-up, agent spawn,
   attach, restart).
2. Operator reports and command outputs filed as traces under `trace/`
   in this task context.
3. Findings synthesized into a durable `doc/` (answers to the unknowns
   above, implications for D1-D4).

The design task is spawned after this task completes, referencing this
task's traces and synthesis doc.

## Resources

- Reference trace (filed in this task): `trace/.../herdr-as-a-service.md`
- herdr source (upstream clone, source of truth):
  `~/code/vendor/herdrdev/herdr/`
- cast source: `crates/cast/src/dev/exec.rs`, `crates/cast/src/dev/run.rs`,
  `crates/cast/src/commands/cli.rs`, `crates/cast/assets/Dockerfile.dev`
- Global flake: `~/.config/cast/nix/flake.nix` (devShells, `herdr` already
  in `commonInputs`, `universal` shell)
