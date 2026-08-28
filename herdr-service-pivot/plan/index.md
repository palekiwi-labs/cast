---
status: open
refs:
- spec/index.md
- todo/1787759808-71a6343/port-vs-identifier.md
- doc/worktree-service-ux.md
---
# Herdr service-pivot spike plan

## Goal

Build the smallest coherent service-model version of `cast` needed to answer
the spike questions. Work proceeds on `spike/herdr-service-pivot` in
`./worktrees/spike-herdr-service-pivot`.

Use vertical TDD slices for behavior that can run in the Nix sandbox. Docker
behavior is exercised by the operator in real Git worktrees, with observations
captured as task traces.

## Phase 1: establish the spike CLI boundary

- [x] Add a CLI-level test proving `cast run` is no longer accepted.
- [x] Remove `run` from the public command surface while avoiding speculative
      deletion of underlying agent code.
- [ ] Add CLI parsing tests for `up`, `down`, `status`, `exec`, and `shell`,
      including the shared `--name` option and `exec` flags.
- [ ] Implement only enough command dispatch for each parsing slice to pass.
- [ ] Run formatting, linting, and the relevant test suite.

## Phase 2: resolve worktree-scoped service context

- [ ] Add a behavior test showing commands from the root and a subdirectory of
      one Git worktree resolve to the same service.
- [ ] Resolve the canonical Git worktree root and relative invocation cwd.
- [ ] Add a behavior test showing two linked worktrees of one repository resolve
      to distinct service identities.
- [ ] Derive deterministic default and named container names from the worktree
      identity.
- [ ] Add a behavior test and clear error for non-Git directories.
- [ ] Define isolated per-worktree multiplexer runtime and persistent-state
      paths.
- [ ] Keep service identity independent from host-port allocation; publish no
      ports in the spike.

## Phase 3: start and inspect the service

- [ ] Add a command-construction test for a detached service container using
      `--init`, `--stop-signal SIGINT`, existing security/resource/env flags,
      and no published ports.
- [ ] Add mount-construction tests for the selected worktree, Git common
      directory, global flake, Nix access, and isolated multiplexer state.
- [ ] Implement `cast up` using existing image and Nix-daemon preparation.
- [ ] Make `up` idempotent when the selected service is already running.
- [ ] Launch the configured devshell command as
      `nix develop <flake>#<shell> -c herdr server`.
- [ ] Implement `cast status` to distinguish absent, stopped, running, and
      multiplexer-unhealthy states.
- [ ] Operator checkpoint: run `up` and `status` in a real worktree and record
      startup, PATH inheritance, liveness, mounts, and resource observations.

## Phase 4: execute commands in the service

- [ ] Add a behavior test for devshell-wrapped `cast exec <cmd...>`.
- [ ] Add behavior tests for `--raw`, `--headless`, and named-service routing.
- [ ] Implement clear failure behavior when the service is not running.
- [ ] Implement `cast shell` as the interactive, devshell-wrapped shell
      convenience command.
- [ ] Operator checkpoint: verify interactive and headless commands, invocation
      cwd mapping, harness availability, and direct multiplexer UI attachment.

## Phase 5: stop, restart, and persistence

- [ ] Add command-construction tests proving `cast down` targets only the
      selected worktree/name service and is idempotent when absent.
- [ ] Implement graceful stop and removal through the configured SIGINT stop
      signal.
- [ ] Operator checkpoint: verify graceful shutdown, restart behavior, isolated
      state across two linked worktrees, and session persistence.
- [ ] Verify named services coexist without runtime or persistent-state
      collisions.

## Phase 6: answer the research questions

- [ ] Test whether panes inherit the devshell PATH.
- [ ] Test passive agent detection versus explicit agent start.
- [ ] Test alternate-screen output and scrollback behavior.
- [ ] Measure idle and active resource usage.
- [ ] Capture each Docker experiment as a trace with commands, environment, and
      observed results.
- [ ] Update the durable synthesis with answers, failures, and implications for
      the future design task.
- [ ] Resolve or explicitly transfer every remaining task todo.

## Validation and commit discipline

- [ ] Follow one observable behavior per red-green cycle.
- [ ] Commit each green or refactor milestone as an atomic commit.
- [ ] Before each commit, run formatter, linter, build, and relevant tests.
- [ ] Keep `.cue/` artifacts out of Git commits.
- [ ] Log every commit and significant experiment in this task context.
