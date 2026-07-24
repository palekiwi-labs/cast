# Container identity vs. shell selection — current state (post nix-native pivot)

Snapshot at `9224622` (feat/universal-container). Documents how `cast run` /
`cast shell` name and select containers after the nix-native-harnesses pivot,
and where the "universal" devShell fits (or does not).

## Summary

The pivot delivered **one shared image, N per-agent containers**. It did not
collapse containers to one. The agent name now does double duty:

- **container identity key** — `cast-{agent}-{basename}-{port}`
- **default devShell selector** — the `#<shell>` fragment on the global flake

These two roles were implicitly fused when there was one harness per image; the
pivot separated the *image* and the *shell* but left *container identity* keyed
on the agent name.

## Code facts (citations)

- `dev/shell.rs:20` — `resolve_port(config, agent.name())`; `:23` —
  `resolve_container_name(config, agent.name(), cwd_basename, port, None, None)`.
  `cast shell <agent>` attaches (via `docker exec -it`) to the agent-named
  container; bails if it is not running.
- `dev/shell.rs:44-45` — non-raw shell wraps `/bin/bash` through
  `build_command(config, &opts, "/bin/bash", agent.name(), vec![])`, entering
  that agent's devShell.
- `dev/container_name.rs:43` — default container name
  `cast-{agent}-{basename}-{port}`; `:42` — config override `{name}-{port}`.
- `dev/port.rs:18` — port seed is `"{pwd}:{agent_name}"`, so the port is
  **agent-specific**. Name AND port differ per agent → three agents coexist
  per project with no collision.
- `dev/build_command.rs:32-33` — shell fragment = `config.global_shell` if set,
  else `agent_name`; global flake ref becomes
  `/home/{user}/.config/cast/nix#{shell}`.

## The "universal" devShell

- `assets/global-flake-template/flake.nix:55` — `universal = mkShell "universal"
  [ opencode pi claudecode ]`: all three harnesses on PATH in one environment.
- No `universal` **agent** exists, so `cast run universal` is not a valid
  command. The only way to reach the universal shell today is
  `global_shell = "universal"` set on a real agent.
- Consequence: `cast run opencode` with `global_shell = "universal"` produces a
  container **named** `cast-opencode-...` that actually contains all three
  harnesses — the name no longer reflects the contents.
- Net: the universal shell ships in the template but has no coherent invocation
  path or container-identity home.

## Container-identity models in play

- **(a) Per-agent containers (current default):** identity = agent. Strong for
  parallel independent sessions; weak for one-box multi-harness.
- **(b) One container per project:** identity = project/workspace; harness
  chosen per exec via devShell. Maximizes multi-harness-in-one-box and resource
  efficiency; loses parallel isolation and simple re-attach.
- **(c) Named sessions:** identity = user-chosen label; harness orthogonal. Most
  flexible, most work.

The pivot leaves us implicitly in (a) while the spec's `cast-agent` motivation
(one harness invoking another) points at (b)/the universal shell.

## Trade-off assessment

Pros of current per-agent model:

1. Parallel independent sessions (three harnesses, three ports, three process
   trees on one project).
2. Stable, human-meaningful re-attach (`cast shell opencode`).
3. Blast-radius isolation between harnesses.
4. Zero migration — naming/port/lifecycle code untouched and already tested.
5. Per-agent port stability for forwarding/editor integrations.

Cons / tensions:

1. Spec's "one container exposes any/all harnesses" is only half-realized — no
   first-class container represents "all agents".
2. Redundant resource cost: N containers from an identical image.
3. State fragmentation (per-container /tmp, servers, process state).
4. Conceptual drift: `agent.name()` overloads identity + shell selection; with
   `global_shell` able to override the shell, the container name can lie about
   its contents.
5. The universal shell has no natural container home (see above).

## Decision held (this conversation)

Keep the per-agent container model as the default — the independent-parallel-work
use case is real and valued. Open question deferred to the linked note: whether
to ship the `universal` shell at all, and if so how to give it a coherent
invocation / container-identity story.
