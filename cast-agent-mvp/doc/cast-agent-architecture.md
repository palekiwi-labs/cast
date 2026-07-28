---
refs:
- /home/pl/code/palekiwi-labs/cast/.cue/cast-agent-mvp/spec/index.md
- .cue/cast-agent-mvp/plan/index.md
- .cue/cast-agent-mvp/doc/opencode-headless-run-contract.md
---
# cast-agent — Architecture & Design

Authoritative design reference for the `cast-agent` binary. Injected from the
`cast-agent-design` context in the `palekiwi/palekiwi` workspace. `plan/index.md`
holds the phased roadmap; this document is the design reference.

## Intent

`cast-agent` launches any supported agent harness (`opencode`, `claudecode`,
`pi`) as a **separate, supervised child process**, primarily in **headless mode
with JSON-lines streaming**, so the caller retains control: monitor the stream,
enforce timeouts, kill cleanly, and return a properly formatted result. It is
the process-isolated alternative to harnesses' in-process subagent/task tools.

It runs inside the same cast container as the calling agent (all three harness
binaries are provided by the workspace `agents` devshell). The container is the
sandbox boundary; isolation from the parent is at the OS **process-tree** level
(process group + SIGKILL).

## The three orthogonal axes

`cast-agent` composes three independent axes. Only the first touches the
`Harness` trait; the other two are pure orchestrator concerns.

1. **Execution mode** — `headless` (consume the JSON-lines stream, return the
   final assistant message) vs `interactive` (attachable TTY). *Touches the
   trait* (different invocation args per mode). MVP is headless-only.
2. **Placement / lifecycle** — `foreground-supervised` vs `detached` (via a
   multiplexer). *Orchestrator.* MVP is foreground-only.
3. **Workspace** — current directory vs a git worktree (named or ephemeral).
   *Orchestrator.* MVP runs in cast-agent's own cwd.

Interesting combinations that emerge:

- headless + foreground + cwd = the MVP
- headless + detached + ephemeral worktree = async delegated subagent, isolated
- interactive + detached + worktree = fresh-context primary handoff a human can
  attach to

## Separating principle: `Harness` trait vs orchestrator

The `Harness` trait encapsulates **only what genuinely differs** between
opencode / claudecode / pi. Everything harness-agnostic lives in the
orchestrator. This is the rule that keeps the trait small and honest.

**`Harness` trait (harness-specific):**

- identity + executable (`name`, `base_command`)
- the mode-specific invocation (`headless_args`, later `interactive_args`)
- per-flag mappers that differ across harnesses (model, permission escalation,
  system-prompt, agent) — each returns `Option<Vec<String>>`
- per-harness result extraction from the JSON stream (`extract_result`)

**Orchestrator (harness-agnostic):**

- **prompt input** (`--file` / stdin / positional) — all three harnesses read
  stdin, so cast-agent reads the file and pipes it to the child's stdin
  uniformly. Never a per-harness mapping.
- **git worktree** create / run / conditional cleanup — pure git.
- **working directory** of the child (`--cwd`). opencode's native `--dir` is at
  most an adapter-level optimization detail, not the primary mechanism.
- **multiplexer / detachment** — where/how the child is spawned.
- **supervision** (wall-clock timeout, silence watchdog, process-group kill).
- **run-log passthrough** of the raw JSON-lines stream.

### The `Harness` trait (proposed shape)

Separate and independent from cast's existing `Agent` trait
(`crates/cast/src/dev/agent.rs:18`), which handles container/config mounting for
`cast dev` — a different concern.

```rust
pub trait Harness {
    /// Identity, e.g. "opencode" | "claudecode" | "pi".
    fn name(&self) -> &'static str;

    /// Executable, e.g. "opencode" | "claude" | "pi".
    fn base_command(&self) -> &'static str;

    /// Headless invocation baseline (subcommand + JSON-lines format flags),
    /// e.g. ["run", "--format", "json"] for opencode.
    /// Named for the mode explicitly; `interactive_args` joins it later.
    fn headless_args(&self) -> Vec<String>;

    // Per-flag mappers. Some(args) = native mapping; None = unsupported,
    // so the orchestrator emits a no-op + warn. Default impls return None,
    // so each harness overrides only what it truly supports. (Backlog for MVP.)
    fn model_args(&self, _model: &str) -> Option<Vec<String>> { None }
    fn escalate_args(&self) -> Option<Vec<String>> { None }
    fn system_prompt_args(&self, _text: &str) -> Option<Vec<String>> { None }
    fn agent_args(&self, _name: &str) -> Option<Vec<String>> { None }

    /// Pull the final assistant text out of the collected JSON-lines events.
    fn extract_result(&self, events: &[serde_json::Value]) -> Option<String>;
}
```

Design notes:

- **`Option<Vec<String>>` returning `None` *is* the no-op + warn mechanism.**
  Honesty about unsupported (flag, harness) cells falls out of the trait for
  free; no separate capability matrix needed.
- **`--timeout` and `--cwd` are NOT trait methods.** Timeout is the supervisor's
  job (wall-clock kill of the process group). cwd is set on the spawned child.
- The `claudecode` identity resolves to executable `claude`, mirroring the
  existing pattern in `crates/cast/src/dev/claudecode/mod.rs:15-32`
  (`name() -> "claudecode"`, `base_command() -> "claude"`).

## Command shape

```
cast-agent run --harness <opencode|claudecode|pi> [flags] [prompt]
```

- `run` is a subcommand (not a bare `cast-agent <prompt>`), reserving the
  namespace for later verbs (e.g. `list-harnesses`, future observe/session
  verbs). Mirrors cast's own `run`/`shell`/`exec` structure. `cast-agent` is a
  separate binary, so `cast-agent run` does not collide with `cast run`.
- `--harness` is **required**, no default — the one flag that cannot be
  no-op'd; it selects the adapter (binary, JSON dialect, extractor, mappings).
  Values: `opencode | claudecode | pi` (`claudecode` spelling matches cast's
  `Agent::name`).
- Prompt input: `--file <path>` (canonical, avoids shell-escaping per the
  cue-plugins temp-file pattern), stdin, or positional for short prompts. All
  fed to the child via stdin uniformly.

## Git worktrees (priority 2)

Built-in lifecycle support — the *create → run → conditional cleanup* lifecycle
is what earns being built in (a bare `git worktree add` hook would not give
auto-cleanup or ephemeral naming).

Two distinct lifecycles:

- **Ephemeral / throwaway** — auto-generated name, **fully discarded on return
  (worktree dir *and* the throwaway branch)**. Purpose: give the agent a private
  **writable scratch space** to test hypotheses; keep only its final output
  message. Use cases: code research, code review, "try things out."
- **Named / persistent** — `--worktree <name>` + `--branch <name>`; the branch
  *is* the artifact you want to land. Cleanup deferred (see open questions).

Proposed surface (orchestrator flags on `run`):

- `--worktree <name>` → create `./worktrees/<name>` (relative to
  `git rev-parse --show-toplevel`), run the child there.
- `--worktree` (no value) / `--ephemeral` → auto-generated name, discarded on
  return.
- `--branch <name>` → branch to create/checkout (default: the worktree name).
- `--base <ref>` → ref to branch from (default: current HEAD).

### cwd vs worktree — lifecycle ownership

`--cwd` and `--worktree` answer different questions and are **mutually
exclusive** (`--worktree` wins / error if both given):

- `--cwd <path>`: *where* it runs. No lifecycle; cast-agent sets the child's
  working directory and walks away.
- `--worktree ...`: cast-agent *owns a lifecycle* (create → run → maybe clean).

They converge only when a worktree is already checked out and you just want to
run in it — then `--cwd` into it is equivalent (no lifecycle needed). Because
both answer "where it runs," they ship **together in priority 2**, not split
across phases.

### Known limitation

An ephemeral worktree branches off **committed** HEAD, so it does **not**
contain the user's uncommitted working-tree changes. An agent asked to "review
my current changes" in an ephemeral worktree would see a clean tree. Documented,
accepted.

"Read-only/exploratory" describes **protecting the user's real tree**, not
making the worktree immutable — the worktree is fully writable scratch that is
discarded.

## Multiplexer (priority 4 — future enhancement, NOT designed yet)

Captured for the full picture, deliberately undesigned. Motivating use cases:

- primary agent delegates a task to a subagent without waiting for it to return
- primary agent at context saturation hands off to a fresh primary agent
  (headless or interactive) to continue with clean context

Shape (to be designed later):

- **`Multiplexer` trait** symmetric with `Harness` (default `Tmux`, room for
  `Zellij`): identity + "spawn this command in a detached/named session" + "how
  to reference it for later attach."
- **Detachment changes the return contract**: a foreground headless run returns
  the result text; a detached run returns a **handle** (session name / run-log
  path) — the caller attaches or reads the log later. "Spawn and don't wait" is
  a different return type, not just a flag.
- **Supervision partly dissolves under detach**: cast-agent no longer holds the
  process group; a timeout would need a self-reaping wrapper inside the session
  or be dropped. `--detach` + `--timeout` is a hard combination.

## Roadmap (priorities)

1. **MVP** — `cast-agent run --harness <h> --file <prompt>`; headless +
   foreground + supervised; runs in cast-agent's own cwd; consumes the
   JSON-lines stream; returns the final assistant message as plain text.
   `Harness` trait carries `name` / `base_command` / `headless_args` /
   `extract_result`; flag-mapper methods exist as `None`-returning stubs.
   Start with the opencode adapter; add claudecode and pi.
2. **Git worktrees** — the "where it runs" surface: `--cwd` + `--worktree`
   (ephemeral and named lifecycles).
3. **Practical MVP enhancements** — the deferred flag shortlist (`--model`,
   permission escalation, `--system-prompt`, `--agent`, `--timeout`, ...), each
   filled in as an `Option<Vec<String>>` trait mapper with per-harness native
   mappings from the three headless research docs.
4. **Multiplexer** — future enhancement per above.

## Open questions (deferred)

- Worktree cleanup on failure/crash: keep for debugging vs `--keep-worktree`
  override. Deferred (too detailed for this stage).
- Structured `--output json` shape; session/resume semantics; parallel spawn +
  process-tree observability — all backlog (carried from `plan/index.md`).
