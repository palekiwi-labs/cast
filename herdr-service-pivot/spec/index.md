---
status: open
---
# Spike specification: service-model cast

## Purpose

Validate the service-container model empirically before committing to a
redesign of cast. The spike is an instrument, not a product: it exists to
answer the open questions in the task card, on a disposable branch.

This document specifies the SPIKE. It is not the target specification for
the redesigned cast — that belongs to the design task, written after this
research task completes.

## Scope

Branch `spike/herdr-service-pivot`, worktree
`./worktrees/spike-herdr-service-pivot`, based on master. The `cast`
binary itself is modified (no separate target) so working code transfers
if the pivot graduates.

### In scope

- `cast up`, `cast down`, `cast status` — service container lifecycle
- `cast exec` — run a command in the service container
- `cast shell` — interactive convenience over exec

### Out of scope (deliberately deferred)

- `--mux` flag — deferred until spike evidence shows how the
  multiplexer's passive agent detection behaves
- Port model changes — see `todo/port-vs-identifier.md`; the spike
  publishes no host ports at all
- MCP auto-start — keep injecting `CAST_MCP_URL` only, as today
- Production migration and backwards-compatibility guarantees
- Multiplexer plurality — herdr only, via a config default
- `Agent` trait rework, config-directory union rework (derived issues)

### Removed from the spike surface

- `cast run` — removed because its per-agent, ephemeral-container model
  contradicts the service model being evaluated. Retaining it would produce
  ambiguous evidence about the intended lifecycle. Removing the command does
  not require speculative cleanup of every underlying agent abstraction.

## Command surface

### `cast up [--name <n>]`

Starts a long-lived, detached service container for the workspace.

- Ensures the dev image and the Nix daemon, reusing existing machinery
- Container command: `nix develop <global-flake>#<shell> -c <mux server>`
  where `<shell>` comes from existing devShell selection config and
  `<mux server>` from the configured multiplexer (default herdr)
- Runs detached with an init process so signals are forwarded
- Stop signal set to SIGINT (see Constraints)
- Reuses the existing security, resource, mount, and env regime from
  `build_docker_run_flags`, including `CAST_MCP_URL` injection
- Idempotent: if the service is already running, report and exit
  successfully rather than starting a second container

### `cast down [--name <n>]`

Stops and removes the service container. Must not fail loudly when the
service is not running.

### `cast status [--name <n>]`

Reports whether the service container exists, is running, and whether
the multiplexer server inside it answers a liveness check.

### `cast exec [--name <n>] [--headless] [--raw] <cmd...>`

Runs a command inside the running service container via `docker exec`.

- Devshell-wrapped by default so harnesses are on PATH; `--raw` skips
  the wrapping
- Interactive with a TTY by default; `--headless` disables the TTY for
  scripts, CI, and piped output
- Fails with a clear message when the service is not running
- Note: exec'd processes do NOT inherit the multiplexer server's
  devshell environment, because the server was launched under
  `nix develop -c`. Each exec re-enters the devshell itself.

Attaching the multiplexer UI requires no dedicated verb: it is
`cast exec <mux-binary>` (e.g. `cast exec herdr`). Session navigation
happens inside the container and is not part of cast's API.

### `cast shell [--name <n>]`

Convenience equivalent to `cast exec bash`, devshell-wrapped,
interactive.

## Container model

- Name: `cast-{basename}-{workspace-id}` for the default service and
  `cast-{basename}-{workspace-id}-{name}` for named services. The
  workspace id is derived from the canonical Git worktree path, so linked
  worktrees of the same repository receive distinct service identities and
  state.
- Named services exist for interference isolation: long-running work
  that must not be disrupted by unrelated activity in another container.
- One service per Git worktree is the expected norm; multiple named
  services must coexist without interfering.
- Commands invoked from a worktree subdirectory target that worktree's
  service and preserve the corresponding relative working directory inside
  the container.
- Non-Git directories are rejected with a clear error; the spike does not
  silently fall back to cwd-based identity.
- Each worktree service has isolated multiplexer runtime and persistent state
  so simultaneous services cannot collide through shared XDG paths, sockets,
  or session data.
- The selected worktree and the Git common directory must both be available
  inside the container. The spike will determine the narrowest reliable mount
  topology without exposing unrelated sibling worktrees by default.
- Service state (multiplexer sessions and workspaces) should survive
  container restarts.

## Constraints and invariants

- No vendor name appears in cast's API surface. The multiplexer is named
  only as a config default, and by the user's own fingers when they type
  `cast exec herdr`.
- No agent taxonomy enters cast. cast must not learn agent kinds; the
  multiplexer owns agent identity and detection.
- `cast run` is not exposed on this branch. The spike tests one coherent
  service lifecycle rather than preserving the old ephemeral lifecycle for
  side-by-side use. Existing port derivation need not be redesigned because
  the service publishes no host ports.
- The multiplexer's graceful shutdown is on SIGINT, but `docker stop`
  sends SIGTERM. The container must set its stop signal to SIGINT so
  shutdown is graceful.
- Reuse existing cast machinery wherever it exists rather than building
  a parallel regime, so evidence transfers to the redesign.

## Questions the spike must answer

1. Does `nix develop <global>#<shell> -c <mux server>` work as a
   container command without a TTY, with the Nix daemon reachable, and
   at acceptable startup latency?
2. Do panes spawned by the multiplexer inherit the devshell environment,
   making harnesses available on PATH inside sessions?
3. Does the multiplexer passively detect agents started in panes, or is
   explicit agent-start required? This determines whether a future
   `--mux` flag can stay generic or would drag agent kinds into cast.
4. Does the service survive `docker stop` and restart with session state
   intact? Is state persisted by the existing shared volumes or does it
   need its own?
5. Do alternate-screen harnesses lose scrollback in a way that breaks
   reading agent output?
6. What is the resource profile of a long-lived service container?
7. What minimal worktree-plus-Git-common-directory mount topology keeps Git
   functional without exposing unrelated sibling worktrees?
8. Which isolated XDG/runtime/persistent paths does each worktree service need
   for collision-free multiplexer operation and restart persistence?

## Non-goals

- Production quality, backwards compatibility, or migration guarantees
- Automated tests for docker-dependent paths; those are validated by the
  operator running the branch in real projects. Pure logic (naming,
  argument construction, flag handling) is unit-tested and must remain
  compatible with the sandboxed Nix build.

## References

- Task: `master/task/herdr-service-pivot.md`
- Reference trace: `trace/1787497876-d580f04/herdr-as-a-service.md`
- Deferred problem: `todo/1787759808-71a6343/port-vs-identifier.md`
