---
status: complete
refs: .cue/master/task/cast-local-json-overrides.md
---

# Plan: cast.local.json Override Layer

Task: `.cue/master/task/cast-local-json-overrides.md`
Branch: `build-cast-local-json` (off `master` @ d580f04)
Worktree: `worktrees/build-cast-local-json`

## Objective

Support an optional `cast.local.json` in the project root that merges on top
of the git-tracked `cast.json` with higher precedence, giving users a
file-based override layer (no direnv/env vars needed) for local-machine
settings such as `extra_data_volumes`.

Resulting chain (low to high):

    defaults < global (~/.config/cast/cast.json) < ./cast.json
            < ./cast.local.json < ./cast-mcp.json < CAST_* env vars

## Research Findings (verified)

- `load_config_with_global` in `crates/cast/src/config/loader.rs` is the
  single place the project `cast.json` is read. All other `cast.json`
  references in the tree are comments or global-config scaffolding paths.
- figment 0.10.19 `Json::file` semantics (verified in registry source):
  - absolute path + missing file -> empty provider, clean no-op
  - malformed file -> error surfaces at `extract()` (same as `cast.json`)
  - dict keys deep-merge per key; arrays replace wholesale
    (existing `env_passthrough` tests document array replace)
- Approval system (`crates/cast/src/config/approval.rs`) hashes the
  serialized effective `Config` (sha256 of canonical workspace + config
  JSON). Local overrides therefore change the hash and flip status to
  `Changed`, requiring `cast config allow` re-approval. This is the
  desired security property: local overrides can change security-relevant
  fields (env_passthrough, volumes) and must be re-approved.
- Master and `build-repo-defined-shells` have identical loader merge-chain
  structure; the feature is independent, so base branch is `master`.

## Design Decisions

- Insert one figment layer in `load_config_with_global`:

      .merge(Json::file(base_dir.join("cast.json")))
      .merge(Json::file(base_dir.join("cast.local.json")))   // NEW
      .merge(Serialized::defaults(mcp_json).key("mcp"))
      .merge(Env::prefixed("CAST_").split("__"))

- No schema changes: any `Config` field is overridable via the existing
  typed merge.
- Missing/malformed `cast.local.json` is treated like `cast.json` today
  (missing = no-op; malformed = error).
- Do not git-track `cast.local.json`: add it to the repo `.gitignore` and
  recommend the same for downstream projects in docs.

## TDD Execution Slices

One test per cycle, red -> green -> commit. After slice 1 the mechanism is
complete; later slices are behavioral confirmations (expected green on
first run — run them to verify, not to drive new code).

- [x] Slice 1 — tracer bullet (loader unit test):
      `cast.local.json` overrides a `cast.json` scalar (`memory`).
      GREEN = add the single `.merge(...)` line.
- [x] Slice 2 — fallthrough (loader unit test):
      keys absent from `cast.local.json` keep `cast.json` values
      (`cpus` from cast.json, `memory` from local).
- [x] Slice 3 — absent file no-op (loader unit test):
      workspace with only `cast.json` loads identically with the new
      layer in the chain.
- [x] Slice 4 — mcp precedence (loader unit test):
      `cast-mcp.json` `port` still wins over `cast.local.json` `mcp.port`.
- [x] Slice 5 — array replace (loader unit test):
      `env_passthrough` in `cast.local.json` replaces the `cast.json`
      list (does not concat).
- [x] Slice 6 — integration (CLI, `crates/cast/tests/config_test.rs`):
      `cast config show` in a workspace with `cast.json` +
      `cast.local.json` reports overridden values; also assert a `CAST_*`
      env var still beats `cast.local.json` in the same subprocess.
- [x] Slice 7 — approval interaction (integration):
      approve a `cast.json`-only config, then add `cast.local.json`;
      `cast config show` stderr hints `cast config diff` (status Changed).
- [x] Slice 8 — polish: extend the `Changed` error note in
      `approval.rs` (`verify`) to mention `cast.local.json` alongside
      env vars; red-green the message assertion.

## Docs and Repo Hygiene

- [x] Update precedence doc comment on `load_config` / `load_config_with_global`
      in `loader.rs` (numbered chain including `cast.local.json`).
- [x] Update `crates/cast/docs/config/overview.md`: Configuration Files
      list + Loading Precedence list; note re-approval requirement and
      gitignore recommendation for downstream projects.
- [x] Add `cast.local.json` to root `.gitignore`.
- [x] Check `crates/cast/docs/config/reference.md` for file-source mentions
      that need the new layer documented.

## Verification

- [x] `cargo test -p cast --lib config::loader` after each slice.
- [x] `cargo test -p cast --test config_test` for integration slices.
- [x] Full gate before final: changed-file `rustfmt --check`,
      `cargo clippy -p cast --all-targets`, `cargo build -p cast`, and
      `cargo test -p cast` (all tests green).
- [x] Tests use tempdirs only (nix sandbox compatible) — no network, no
      `$HOME` assumptions.

## Commit Strategy

- One commit per green slice where it drove code (slices 1, 6-8);
  confirmation-only slices (2-5) may be batched into a single `test:`
  commit if all pass immediately.
- Docs + .gitignore in a separate `docs:` commit.
- Conventional commits, imperative, <= 50-char summary.
