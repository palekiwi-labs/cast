# Project Log

## [d580f04] Research task created: herdr service pivot feasibility

Created research task `herdr-service-pivot` and moved the herdr
implementation guide from the master context into this task's trace store
(`trace/1787497876-d580f04/herdr-as-a-service.md`).

Session-start orientation against cast source (`crates/cast/src/dev/exec.rs`,
`crates/cast/src/commands/cli.rs`) framed the feasibility question: the
herdr-as-a-service model needs repeated one-shot `docker exec` calls into one
long-lived named container, but `cast exec` always provisions a fresh
ephemeral container (`docker run --rm` with a per-invocation name token).
Preliminary answer to the user's question is therefore "no, not without
adaptations", pending full verification of the remaining integration points
(Nix devShell harness provisioning vs herdr `agent start`, MCP server
placement, image changes, vendored herdr fork parity).

- **Found:** cast exec always starts a fresh container (docker run --rm) and cannot target a running container - the primary gap for the herdr service model
- **Found:** Reference report was verified against upstream herdrdev/herdr, but local checkout is a vendored fork at ~/code/vendor/ogulcancelik/herdr - parity unverified
- **Found:** herdr agent start requires the agent binary discoverable inside the container, while cast provisions harnesses via Nix devShells selected at run time - integration point needs design
- **Decided:** Task context herdr-service-pivot created (kind: research, in-progress)
- **Decided:** herdr reference trace moved from .cue/master into the task context
- **Open:** Q1: minimal cast adaptations for a PoC branch (docker exec into existing container, service lifecycle, image/env/volume changes)
- **Open:** Q2: how herdr should launch Nix devShell-provisioned harnesses
- **Open:** Q3: where the built-in cast MCP server fits when agents start on demand inside a service container
- **Open:** Q4: verify vendored herdr fork matches the reference trace behavior
- **Open:** Q5: product/command surface and version target (v0.3.0?)

## [d580f04] Redesign vs minimal PoC: evidence and recommendation

Operator reframed Q1: minimal PoC vs full redesign of cast around the
service model. Operator's two theses: (1) after the global-devshell
provisioning pivot, hardcoded agent commands no longer make sense and cast
should become an agent-agnostic sandbox; (2) multiplexing should be
first-class (operator works in tmux; herdr chosen). Asked whether we can
design a brand new approach instead of a minimal PoC.

Evidence gathered from code supporting the redesign thesis:
- Dockerfile.dev is already harness-free; agent identity survives only in
  the clap enums (RunAgent/ShellAgent/ExecAgent), Agent trait impls, and
  per-agent port allocation. global_shell is already a free string and a
  universal devShell (all harnesses) exists in the global flake.
- herdr is already provisioned in commonInputs of the global flake, so it
  is on PATH in every cast shell; ecosystem has converged on it.
- herdr ships 20+ agent detection manifests (amp, codex, gemini, grok,
  qwen, ...) vs cast's 3 hardcoded agents - maintaining a cast-side agent
  taxonomy duplicates herdr's detection layer.
- MCP already decoupled: cast run only injects CAST_MCP_URL env; the
  server is standalone via `cast mcp start`. concepts.md claim that run
  starts the server is stale.

Agent recommendation (reported to operator, awaiting confirmation):
redesign as the primary track, paired with a standalone de-risk spike
(docker compose against the existing cast dev image, no cast code changes)
to validate empirical unknowns (nix-develop-wrapped agent detection, herdr
as PID 1 service, state persistence, alt-screen caveats). A minimal PoC
branch inside cast is the worst middle ground: it would build transitional
API surface that the service model makes semantically impossible
(`cast run <agent>` has no per-agent container to target).

- **Found:** herdr already in commonInputs of the global cast flake - on PATH in every cast shell
- **Found:** global flake has a universal devShell bundling opencode, pi, and claude-code harnesses, selected via global_shell
- **Found:** Dockerfile.dev already states 'No harness binaries are baked in' - the image is already agent-agnostic
- **Found:** herdr upstream ships 20+ agent detection manifests vs cast's 3 hardcoded clap enum agents
- **Found:** cast run does not start the MCP server; it only injects CAST_MCP_URL - concepts.md doc claim is stale
- **Decided:** Q2 resolved (operator): herdr does not care about binary provenance, only PATH; provisioning is the user's/nix's responsibility
- **Decided:** Q3 resolved (operator + code): MCP stays a single host HTTP process; verified run only injects CAST_MCP_URL - stale doc claim in concepts.md to fix
- **Decided:** Q4 resolved (operator): upstream clone at ~/code/vendor/herdrdev/herdr/ is the source of truth; ogulcancelik fork obsolete
- **Decided:** Q5 deprioritized (operator): aspirational v0.2.0, not research-critical
- **Decided:** Task card updated with new design questions D1-D4 and empirical unknowns for the spike
- **Open:** Operator confirmation of the two-track recommendation (redesign + standalone spike)
- **Open:** D1-D4 design questions to be answered in a design task if redesign confirmed
- **Open:** Empirical spike: herdr agent detection through nix develop -c wrappers is the highest-risk unknown

## [d580f04] Task placement: spike belongs to research, not design

Operator asked whether the spike belongs on this research task or on
the upcoming design task. Answered: the spike belongs here.

Rationale (per cue kind contracts): research answers open questions and
tests hypotheses, producing evidence (traces, synthesis docs); design
drafts the target specification and consumes that evidence. The spike is
hypothesis testing (detection through nix wrappers, service lifecycle,
persistence) with no cast feature code, which is squarely within the
research boundary. Placing it in design would contaminate the design task
with evidence gathering and force D2 decisions (herdr at PID 1: baked vs
nix-provisioned) to be made on untested assumptions - risk of rework.

Sequencing decided: spike in this task -> traces filed here -> synthesis
doc here -> research complete -> spawn design task referencing this task.
Task card updated with explicit completion criteria.

- **Decided:** Spike stays on herdr-service-pivot (research); design task spawned only after research completes
- **Decided:** Completion criteria for this task: spike executed, traces filed, synthesis doc written

## [d580f04] Spike execution model: collaborative branch, operator-run docker

Operator rejected the runbook/scratch-compose execution model (agent has
no docker access; spike needs real directories) and proposed: develop the
spike as real code on a `spike` branch in the cast repo, operator runs all
docker-requiring steps by executing the spike binary via `nix run` in real
projects, reporting findings back. Production cast stays in daily use
throughout.

Agent assessment: strictly better than the runbook plan because (1) cast's
job is docker orchestration, so the spike binary performs the docker work
natively; (2) exercises real code paths (build_docker_run_flags,
nix-daemon ensure, workspace resolution) so evidence transfers directly to
the redesign; (3) ecological validity via real projects; (4) branch is
disposable, zero production risk.

Mechanics agreed/pending: worktree at ./worktrees/spike (branch `spike`,
forked from master) since main checkout is mid-flight on
build-repo-defined-shells; separate binary target `cast-spike` so the
production CLI surface is untouched and there is no ambiguity about which
binary runs. Existing worktrees checked: main, .cue, feat-cast-agent-mvp -
no conflicts.

Spike surface: up / status / agent / attach / restart / down, mapped to
phases 1-4.

Design insight recorded: CMD = `nix develop <global-flake> -c herdr
server` would put the herdr server inside the devshell so every pane
inherits the devshell PATH - D2's nix-provisioned option solving the pane
PATH problem in one move. Wrinkles: interactive bash PATH resets,
IN_NIX_SHELL semantics. Fallback: mount/bake the herdr closure.

- **Decided:** Spike = real code on a `spike` branch (disposable, no production obligation), separate `cast-spike` binary target
- **Decided:** Worktree at ./worktrees/spike per global convention; main checkout untouched
- **Decided:** Operator runs docker-requiring steps via `nix run ./worktrees/spike#cast-spike` in real projects and reports back
- **Open:** Operator confirmation: branch base = master (vs current feature branch)
- **Open:** Confirm command surface up/status/agent/attach/restart/down
- **Open:** Verify how cast enters the global devshell inside the container (build_command) before wiring CMD

## [d580f04] Branch created; devshell mechanics mapped; surface design pending

Operator corrections adopted: (1) branch named for the pivot, not
generic `spike` -> `spike/herdr-service-pivot`, worktree
`./worktrees/spike-herdr-service-pivot`, based on master (created); (2) no
separate binary target - modify `cast` itself on the branch so code
transfers directly if the pivot graduates to production; (3) the command
surface must be designed collaboratively BEFORE implementation - no
scaffolding yet.

Delegated @explore to map the global-devshell entry path. Key findings:
- `cast run opencode` argv inside container is
  `nix develop /home/{u}/.config/cast/nix#opencode -c opencode`
  (build_command.rs; only -c, no --impure/--accept-flake-config)
- DevShell selection: config.global_shell if set, else agent name
- Global flake dir mounted wholesale rw into container
  (universal/volumes.rs:83-92)
- `cast shell` = docker exec -it into the stable-named container running
  `nix develop <global>#<agent> -c /bin/bash` - exec-into-existing
  precedent exists in production code
- Stable container name `cast-{agent}-{basename}-{port}` is the existing
  deterministic re-attachable identifier (container_name.rs:19)
- Headless cast run already runs nix develop -c without a TTY - the
  service CMD hypothesis `nix develop <global>#universal -c herdr server`
  rides existing plumbing; breakage candidates all assessed benign
  (accept-flake-config=false irrelevant for herdr; daemon socket via ro
  /nix mount is the existing mechanism; global shellHook only echoes)

- **Found:** cast run argv: nix develop /home/{u}/.config/cast/nix#{shell} -c {cmd}; global_shell config overrides devShell name
- **Found:** cast shell uses docker exec -it into stable-named container - pattern precedent for service attach
- **Found:** Headless nix develop -c already works TTY-less, making the herdr-server-as-CMD hypothesis low-risk
- **Decided:** Branch spike/herdr-service-pivot created at ./worktrees/spike-herdr-service-pivot (from master)
- **Decided:** Reuse the `cast` binary on the branch; no cast-spike target
- **Decided:** Implementation blocked on collaborative API/UX design discussion
- **Open:** API/UX design discussion: verb naming (bare vs noun-group), wrap vs passthrough for agent ops, service devShell choice (universal?), fate of run/shell/exec

## [d580f04] Design session 4: mux abstraction, verb approvals, exec rework opening

Fourth design session with the operator. Approvals: bare top-level verbs
(cast up/down/attach); passthrough direction for session ops. Strong
operator objection on naming: hardcoding `herdr` in the API (`cast herdr
...`) repeats the harness-hardcoding mistake we are dismantling — "we
just decided to abandon hard-coding agent harnesses, now we want to
repeat that mistake with herdr?"

Resolution adopted (agent proposal, operator to confirm verb name):
role-named passthrough `cast mux <args>` forwarding to the CONFIGURED
multiplexer client inside the container; config default herdr. Internal
Multiplexer abstraction (server cmd for CMD, client cmd for attach,
liveness check) mirroring the Agent trait pattern. Principle recorded:
vendor names allowed as config defaults, never in API surface/semantics.
Service CMD composes: nix develop <global>#<shell> -c <mux server cmd>.

DevShell selection deferred: config-driven, reworked on the orthogonal
build-repo-defined-shells branch; no design impact here.

Operator input on run/shell/exec rework: shell is very useful (entering
the container); exec never used — existed only because run hardcodes a
container name; imagines headless harness prompts targeting either the
running service container or a fresh one; asks for a generalized,
streamlined API instead of separate commands.

Operator input on naming/isolation: one service per workspace is the
norm, but users must be able to isolate processes (e.g. long-running work
that should not be interrupted by unrelated container disruption) —
motivates named services (cast up --name <n>) alongside the default
service.

- **Decided:** Bare verbs approved: cast up / down / attach (+ status)
- **Decided:** Passthrough for session ops approved; vendor named only as config default, never in API
- **Decided:** Multiplexer abstraction internally (Agent-trait symmetry); service CMD = nix develop <global>#<shell> -c <mux server cmd>
- **Decided:** DevShell selection deferred to build-repo-defined-shells branch (config-driven)
- **Open:** Passthrough verb name: `mux` proposed, alternatives session/ctl — operator call
- **Open:** run/shell/exec rework: generalized exec (service default, --fresh ephemeral) vs keeping run as sugar over mux agent start
- **Open:** Named services (--name) and name formula without agent slot
- **Open:** MCP auto-start on `up`: keep URL injection only for spike

## [3ad3310] Design session 5: exec use cases, mux-as-flag, naming resolved

Session 5 design analysis: operator challenged --fresh (dislikes it),
questioned exec use case, and proposed mux as a flag rather than a
standalone verb.

Container naming: port.rs encodes CRC32("{pwd}:{agent_name}"). For the
service model, drop agent_name from the seed → CRC32("{pwd}") alone.
Port becomes per-workspace, still unique across different absolute paths
(e.g. ~/code/acme/webapp vs ~/code/corp/webapp). One-line change in
calculate_port. Name formula: cast-{basename}-{workspace-port} default,
cast-{basename}-{workspace-port}-{name} for named services.

exec: drop --fresh (operator dislikes; ephemeral new container use case
unclear enough to defer). cast exec = docker exec into service container,
optionally devshell-wrapped. Use case: inspection, maintenance scripts,
headless one-shot harness runs (cast exec opencode run 'prompt...')
without agent lifecycle tracking.

Note: docker exec into service container does NOT inherit the herdr
server's devshell env automatically; exec'd processes need their own
nix develop invocation. Same mechanism as current exec. Expected latency.

attach vs mux as flag: operator proposes --mux on cast exec. Resolution
analyzed: cast exec <cmd> = raw in container; cast exec --mux [args] =
via configured mux client; cast exec --mux (no args) = TUI attach.
cast attach stays as top-level alias for discoverability (most common
human workflow). Flag name candidates: --mux (names the role abstractly)
vs --session (completely technology-agnostic). Decision pending.

- **Decided:** run deferred to spike evidence per operator
- **Decided:** Drop --fresh from exec; ephemeral container use case deferred
- **Decided:** Named services with --name flag: cast up --name <n>; all service verbs accept --name
- **Decided:** Port shifts to per-workspace: CRC32(pwd) only, drop agent from seed; addresses org/repo subdir collision
- **Open:** Flag name for mux routing on exec: --mux vs --session (operator confirmation needed)
- **Open:** Whether cast attach stays as top-level alias or folds into exec --mux entirely

## [71a6343] Design session 6: attach and --session rejected, docker-conventional surface

Session 6: operator rejected `cast attach` and the `--session` flag, and
proposed a docker-conventional surface: `cast up` starts the project
container, `cast exec <cmd>` runs commands in it with `--headless` and
`--mux` flags.

Agent analysis of the rejections (both accepted):
- `--session` was wrong. Operator: "if there are different sessions in
  the container and I want to navigate them, shouldn't I enter the shell
  first?" Correct — session navigation is the multiplexer's domain, not
  cast's. cast is the infrastructure plane; once you are inside the
  container you are in the mux's world.
- `attach` dies as a consequence, and the result is cleaner than the
  abstraction it replaces: attaching the UI is just
  `cast exec <mux-binary>` (e.g. `cast exec herdr`). The user names their
  multiplexer with their own fingers; cast's API never does. This is
  stronger agnosticism than the Multiplexer-abstraction-behind-a-verb
  design from session 4 — that abstraction now only needs to cover the
  container CMD (server command) and optionally the `--mux` flag.

`--mux` retained (operator's own proposal) with semantics to pin down:
run inside a mux pane (tracked, attachable, survives disconnect) vs bare
docker exec (bound to caller's terminal, dies with it). Open spike
question: herdr passively detects agents in panes, so a generic
`pane run` may yield lifecycle tracking without cast ever calling
`agent start` — which would keep kind taxonomy out of cast entirely.

Port/identifier conflict captured as a todo per operator instruction
(todo/port-vs-identifier.md) rather than resolved now: with named
services, two containers in one workspace derive the same hash-based
number, which is fine as an identifier but conflicts as a port. Options
recorded: seed with service name, explicit --port on `cast up`, or
decouple identity from ports (operator leans explicit).

- **Decided:** cast attach dropped; attaching = cast exec <mux-binary>, cast never names a multiplexer in its API
- **Decided:** --session flag retracted: session navigation belongs inside the container
- **Decided:** Surface follows docker convention: cast up + cast exec [--headless] [--mux] [--raw] <cmd>
- **Decided:** Port/identifier conflict deferred to todo/port-vs-identifier.md, not blocking the spike
- **Open:** --mux semantics: does pane run + herdr passive detection give tracking for free, or is explicit agent start needed (taxonomy risk)?
- **Open:** Whether --mux is needed for the spike at all, or deferrable like run

## [71a6343] Spike specification written, pending operator review

Operator called the checkpoint: specification before implementation.
Wrote `spec/index.md` in the task context covering the spike surface
(up/down/status/exec/shell), container model, constraints, and the six
questions the spike must answer.

Boundary kept explicit in the document: this is the SPIKE spec (what we
build to learn), not the target spec for the redesigned cast. The latter
belongs to the design task, per the research/design split agreed in
session 2. Without that separation the research task silently becomes the
design task.

Decisions made while drafting that need operator review:
- Leave `calculate_port` untouched so `cast run` keeps working exactly as
  today; service naming gets its own workspace-level id derivation. Lets
  old and new models run side by side for comparison.
- Spike publishes no host ports at all, which sidesteps the
  port-vs-identifier todo instead of pre-deciding it.
- Container stop signal must be SIGINT: herdr's graceful shutdown is on
  SIGINT while `docker stop` sends SIGTERM. Without this, every stop is
  effectively a kill.
- herdr session state may persist for free via the existing shared
  `~/.local` volume (run.rs already mounts shared cache/local volumes);
  spike question 4 verifies rather than assuming.
- `cast up` specified as idempotent (report and succeed if already
  running) because the mux server refuses to double-bind its socket.

- **Found:** herdr graceful shutdown is SIGINT but docker stop sends SIGTERM - container needs --stop-signal SIGINT or every stop is a kill
- **Found:** cast already mounts shared ~/.cache and ~/.local volumes, so herdr session state may persist without new volumes
- **Decided:** Spike spec written at spec/index.md in the task context
- **Decided:** Spec scope is the spike instrument only; target cast spec deferred to the design task
- **Decided:** calculate_port untouched so cast run keeps working; service naming uses separate workspace-level derivation
- **Decided:** Spike publishes no host ports, sidestepping the port-vs-identifier todo
- **Open:** Operator review of the spike spec before implementation begins
- **Open:** Executive plan with checkboxes to follow once spec is approved

## [79e8960] Service-Pivot Research Completed

Completed research on service-pivot UX. Analyzed path discovery, Docker mounts, and naming logic. Proposed a Git-aware pivot strategy to stabilize project containers across worktrees and subdirectories. Identified risks related to zombie processes and Git dependency.

- **Found:** Current discovery is purely based on std::env::current_dir(), causing fragmentation.
- **Found:** Linked worktrees lack access to the main .git folder in the container.
- **Found:** opencode leaks zombies which makes long-lived containers risky without --init.
- **Decided:** Pivot project root using git rev-parse --git-common-dir.
- **Decided:** Stabilize port and container name on the main project root.
- **Decided:** Always mount the main project root to ensure Git common directory availability.
- **Open:** How to handle non-Git projects gracefully? (Fallback to current behavior planned).

## [79e8960] Analyzed worktree-compatible service UX

Synthesized the requested worktree analysis in doc/worktree-service-ux.md. The recommended design separates stable repository service identity from the invoking worktree cwd, so root and linked worktrees share one service but commands operate in their selected checkout.

- **Found:** Current workspace resolution is the literal cwd, so subdirectories and linked worktrees receive distinct names, ports, approvals, and mounts.
- **Found:** A linked worktree's .git file references the primary checkout's common Git directory; mounting only that worktree breaks Git.
- **Found:** The existing home-relative container path mapping can break absolute gitdir pointers for repositories outside the host home directory.
- **Found:** Configuration loading and approval are cwd-scoped today and require an explicit service-scoping decision.
- **Decided:** No implementation decision recorded; report recommends repository-scoped service identity and invocation-scoped cwd.
- **Open:** Whether service-shaping configuration is repository-root-only or branch-local configurations must be conflict-checked.
- **Open:** How to expose external worktrees created after the service starts without silently operating on an unmounted checkout.
- **Open:** Whether a service name should be a canonical-path digest independent of optional ports.

## [79e8960] Examined worktree-scoped services

Re-evaluated the earlier repository-scoped worktree recommendation after the operator identified configuration scope as a decisive concern. Updated doc/worktree-service-ux.md with a worktree-service alternative and its mount/state requirements.

- **Found:** One service per worktree preserves branch-local cast configuration, local overrides, project flakes, and existing approval scope without cross-worktree conflicts.
- **Found:** Git works with a narrow topology: mount the selected worktree and its git common directory; mounting the full primary checkout is sufficient but exposes sibling worktrees unnecessarily.
- **Found:** Existing shared ~/.local data volumes cannot safely host two herdr services because XDG/socket/session state could collide; mux state must be worktree-keyed and isolated.
- **Found:** The existing port decision is documented in todo/port-vs-identifier.md: the spike has no published ports, and identity/port separation remains deliberately unresolved.
- **Decided:** No final service-boundary decision recorded. Worktree-scoped services are recommended for evaluation as the direct solution to branch-specific configuration.
- **Decided:** Non-Git fallback recommendation withdrawn: cast should reject non-Git directories clearly.
- **Open:** Whether the pivot's stated one-container-per-project norm should become one-container-per-worktree.
- **Open:** Whether worktree services mount only their checkout plus the common Git directory, or the full primary checkout tree.
- **Open:** Exact per-service herdr XDG state-volume topology.
- **Open:** Port allocation policy when a worktree service eventually publishes host ports.

