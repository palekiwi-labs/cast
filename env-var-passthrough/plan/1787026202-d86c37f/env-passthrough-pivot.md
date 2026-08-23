---
status: in-progress
refs:
- .cue/env-var-passthrough/plan/index.md
- .cue/master/task/env-var-passthrough.md
parent: plan/index.md
---
# Executive plan: env passthrough pivot (base + extra, agent forwarding removed)

## Foreword

Implements the 0.2.0 pivot recorded on the task card
(`.cue/master/task/env-var-passthrough.md`, Design + Status-and-handover,
2026-08-23). The closed master plan
(`.cue/env-var-passthrough/plan/index.md`) remains authoritative for
constraints, trust-boundary analysis, and rejected alternatives; this
plan inherits them by reference and does not repeat them.

In one sentence: add `extra_env_passthrough` beside the existing
`env_passthrough`, feed the run-time flags from their concatenation,
and delete the hardcoded per-agent env forwarding entirely, so that
config becomes the only passthrough channel and approval covers all of
it.

Lands as NEW commits appended to `feat/env-passthrough` (at `d86c37f`,
12 commits, tree green). No rework of existing commits. One phase =
one commit, each green and clippy-clean on its own. TDD throughout.

Facts established by the pre-plan survey, so a fresh agent need not
rediscover them:

- Schema field lives at `config/schema.rs:54` (`env_passthrough`), the
  `Default` arm at `schema.rs:172`. The new key slots in beside both.
- The flags call site is `dev/run.rs:436-439`
  (`build_env_passthrough_args(&config.env_passthrough, ...)`) inside
  `build_docker_run_flags`; the reserved-name console warning is at
  `run.rs:162-176` and uses `reserved_names_in(&config.env_passthrough)`
  at `:165`. Both must switch to the concatenated effective list.
- The env snapshot at `run.rs:151` exists to build `host_env_names`
  (`run.rs:156-160`, names only, set-but-empty filtered). After this
  pivot it feeds nothing else; keep the filter and its comment.
- The agent forwarding surface to delete, in full:
  - trait method `Agent::env_passthrough_args` — `dev/agent.rs:30-36`
    (its only reason to import `HashMap`);
  - impls `dev/opencode/mod.rs:56-58`, `dev/claudecode/mod.rs:61-63`,
    `dev/pi/mod.rs:48-50`, plus each `mod env;` declaration and the
    claudecode impl test at `claudecode/mod.rs:131`;
  - the three modules `dev/opencode/env.rs` (39 names),
    `dev/claudecode/env.rs` (9 names), `dev/pi/env.rs` (19 names) —
    each contains nothing but the const, the builder, and tests, so
    whole-file deletion;
  - the sole consumer: layer 3 of `build_universal_run_args`
    (`dev/universal/volumes.rs:84`), whose `launched_agent` (`:66`) and
    `env` (`:69`) parameters exist ONLY for that layer — both params
    and doc bullet 3 (`:58-63`, "four layers" becomes three) go with
    it, plus the test `env_passthrough_from_launched_agent_only`
    (`volumes.rs:371`);
  - wrapper `build_session_run_args` (`run.rs:126-133`) shrinks to
    `(config, run_opts)`; production caller `run.rs:189` and test
    caller `run.rs:857` follow; 6 test call sites in `volumes.rs`
    (`:235-387`) follow the shrunk signature;
  - doc comments referencing the mechanism: `run.rs:62` and
    `run.rs:124-125`; docs page `crates/cast/docs/agents.md:34` names
    `env_passthrough_args` and must drop it in the same commit.
- `mcp/exec.rs` `host_env` (`:70`, `:175`) is a separate channel
  (env_clear + resolve_env for MCP tool subprocesses). Untouched.
- Deletion removes 7 existing tests (2 opencode env, 2 claudecode env,
  1 pi env, 1 claudecode mod, 1 volumes). Expect the lib count to drop
  from 261 by 7, then rise with the new tests.
- `CHANGELOG.md` has an `[Unreleased]` section at the top; the entry
  goes there. Fold-into-0.2.0 mechanics are owned by release-0.2.0.

Operational warnings carried from the handover:

- A concurrent writer mutated `run.rs` / `loader.rs` mid-session last
  round. Before each edit session, verify working tree against git
  (`git status`, `git diff`); if the writer reappears, build exact
  content in /tmp, verify against the parent commit, and commit from
  the index — never trust working-tree greps.
- Never include `.cue/` in git commits.
- Pre-existing `cargo fmt --check` diffs (cli.rs, build.rs, image.rs)
  stay untouched; the deferred repo-wide formatting pass is separate.

## Steps

### Phase 1 — second schema key + effective-set concat

- [x] `config/schema.rs`: add `#[serde(default)] pub
      extra_env_passthrough: Vec<String>` beside `env_passthrough`
      (`:54`) with a doc comment stating its role (per-project
      additions to the base list) and that values are never stored;
      `Vec::new()` in the `Default` impl beside `:172`.
- [x] Concat location decided: a small private helper co-located in
      `dev/run.rs` (e.g. `effective_env_passthrough(&Config) ->
      Vec<String>` returning `base ++ extra`), called by BOTH the
      reserved-name warning (`:165`) and the flags builder (`:437`),
      so the concat exists in exactly one place. Not a `Config`
      method: `schema.rs` stays pure data, and `loader.rs` does not
      combine `nix_extra_substituters` either — consumption-site
      combining is the existing precedent.
- [x] Loader tests (`config/loader.rs`, mirroring the `env_passthrough`
      block at `:162-201`): `extra_env_passthrough` defaults to empty;
      global extra applies when the project is silent; a project extra
      list REPLACES the global extra list (per-key replacement); a
      project that sets only `extra` leaves the global base list
      intact (keys replace independently).
- [x] Approval test (`config/approval.rs`, mirroring `:403`): adding
      an `extra_env_passthrough` name changes the hash and returns
      `ApprovalStatus::Changed`.
- [x] Wiring tests (`dev/run.rs` `build_docker_run_flags`): a base
      name and an extra name from one config BOTH emit valueless
      `-e NAME`; a name listed in BOTH keys emits a single `-e` pair
      (cross-key dedupe — same BTreeSet pass as within-key); a name
      listed only in `extra` and reserved is included in the reserved
      set fed to the warning (assert via the effective helper or
      `reserved_names_in` over the concat).
- [x] Suite + clippy green. Commit (schema+concat+tests in one atomic
      unit, matching the single-key precedent of 31a8e71).
      **Done: 1313986** (272 lib + 18 + 9; committed via /tmp build
      against an active concurrent writer — see log).

### Phase 2 — zero-trace deletion of agent forwarding

- [x] Delete `dev/opencode/env.rs`, `dev/claudecode/env.rs`,
      `dev/pi/env.rs`; drop their `mod env;` lines, the three trait
      impls, and the claudecode impl test (`claudecode/mod.rs:131`).
- [x] Remove `env_passthrough_args` from the `Agent` trait
      (`agent.rs:30-36`) and the now-unused `HashMap` import.
- [x] Shrink `build_universal_run_args` to
      `(included_agents, config, opts)`: params `launched_agent` and
      `env` die with layer 3; doc comment layers go four to three;
      update the 6 test call sites (`volumes.rs:235-387`); delete
      `env_passthrough_from_launched_agent_only` (`volumes.rs:371`).
- [x] Shrink `build_session_run_args` to `(config, run_opts)`
      (`run.rs:126-133`); update the production call at `:189` (note:
      `run_in_container`'s own `agent` param may have other uses —
      check before touching it) and the test call at `:857`.
- [x] `run.rs:151` snapshot now feeds only `host_env_names`; keep it
      (or inline it into the `:156-160` construction — either is
      fine), preserving the set-but-empty filter and its comment.
- [x] Update stale doc comments: `run.rs:62` (drop
      `Agent::env_passthrough_args` mention), `run.rs:124-125`
      ("Environment passthrough comes from the launched agent only" —
      now config-only). One-line fix in `docs/agents.md:34` in the
      same commit so docs never reference a deleted API.
- [x] Verify zero remaining references:
      `grep -r 'PASSTHROUGH_VARS\|build_passthrough_env_args\|env_passthrough_args'
      crates/` returns nothing.
- [x] Suite (expect lib count down 7, plus any new) + clippy green.
      Commit as a breaking removal. **Done: 85e030a** (265 lib + 18 + 9).

### Phase 3 — docs + CHANGELOG

- [x] Rewrite the `env_passthrough` section of
      `crates/cast/docs/config/env-overrides.md` (`:22-`): two keys and
      their intended scopes (base = global `cast.json`, extra =
      per-project additions); per-key replacement stated for BOTH keys
      with the additive trap called out ("extra" does NOT merge global
      entries into every project — that union was rejected); effective
      set = base ++ extra, then one filter pass (validity, reserved,
      unset/empty, dedupe, sort).
- [x] Same section: provider API keys become the canonical example
      (task card example: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` in
      base; `GH_TOKEN` in extra) — N5 inverts into the required usage
      pattern. Reserved names stated ONCE and exactly (PATH, HOME,
      NIX_REMOTE — keep prose and `RESERVED_ENV_NAMES` in sync or
      reference the const; N6). `CAST_EXTRA_ENV_PASSTHROUGH` bracketed
      list literal documented in one line beside the existing
      `CAST_ENV_PASSTHROUGH` syntax note (same unbracketed-fails
      caveat). Audit via `cast config show`.
- [x] Trust-boundary note updated to the stronger post-pivot claim:
      config is the ONLY env forwarding channel, so approval gates all
      passthrough; the "attacker with `$(...)` already has host code
      execution" reasoning stays as-is.
- [x] `crates/cast/docs/config/reference.md:12`: add
      `extra_env_passthrough` beside `env_passthrough`.
- [x] `CHANGELOG.md` `[Unreleased]`: under **Removed** — breaking
      deletion of hardcoded per-agent env forwarding (provider API
      keys, agent flags), with the migration: add provider keys to
      global `cast.json` `env_passthrough` or agent auth breaks after
      upgrade. Under **Added** — the two config keys. Note that adding
      config fields changes every config's hash, so existing approvals
      trip `Changed` once on upgrade (N7).
- [x] Suite + clippy green (docs only, but verify nothing else broke).
      Commit. **Done: 8602518**.

### Phase 4 — QA checklist + close-out

- [ ] Derive a fresh manual-QA todo (point-in-time artifact under this
      task's `todo/`) for the two-key design. MUST include the
      never-run end-to-end approval-gate check on a real host: adding
      a name to project `cast.json` makes `cast run` hard-error;
      `cast config allow` then lets it forward. Also: per-key
      replacement live; cross-key dedupe; provider-key migration
      (allowlist `ANTHROPIC_API_KEY`, agent authenticates again);
      reserved-name console warning fires for names from either key;
      and re-verify the previously proven properties survive the
      deletion (valueless `-e` inheritance, no leak to argv/approval
      store/logs, empty treated as unset, passthrough beats
      `cast.env`, cast authoritative for `USER`/`CAST_MCP_URL`).
- [ ] `cargo test -p cast` and `cargo clippy --all-targets` green
      across the workspace.
- [ ] Task card: refresh Status-and-handover to record the pivot
      landed (commit list, test counts), fill Evidence for the new
      acceptance shape.
- [ ] Task stays `in-progress` until the user works through the QA
      checklist and signs off. Only then `complete`.
