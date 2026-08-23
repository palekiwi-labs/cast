---
status: closed
refs:
  - .cue/env-var-passthrough/plan/index.md
  - .cue/master/task/env-var-passthrough.md
---

# Executive plan: env_passthrough allowlist

> **Superseded 2026-08-23 — closed.** Every step below was implemented
> except the manual-QA block. QA ran partially against this single-key
> build (inheritance, leak, precedence, and authority checks ticked on
> a real host; the approval-gate and replace-semantics sections did
> not run), and the base + extra pivot redefines exactly the
> replace semantics that section 7 was written to check. The pivot
> lands as appended commits per the task card; a new executive plan
> will be drafted for it.

## Foreword

Implements all four phases of `plan/index.md` for task
`env-var-passthrough`. Read that master plan first — it records why the
original `CAST_ENV__` design was rejected and why names, not values, are
placed under approval.

In one sentence: add `env_passthrough: Vec<String>` to `Config`, and for
each listed name that is set in the host environment, emit
`docker run -e NAME` (valueless form) so the value is inherited by the
docker child rather than passed in argv.

Key facts established during analysis, so a fresh agent need not
rediscover them:

- `DockerClient` spawns `Command::new("docker")` with no env
  customisation (`docker/client.rs:135, 160, 205`). The docker child
  **inherits cast's environment**, so the valueless `-e NAME` form works
  with no plumbing at the spawn site. Phase 3 of the master plan
  collapses into the `build_docker_run_flags` change alone.
- `build_docker_run_flags` is at `dev/run.rs:358`; the `--env-file` block
  is at line 397 and cast's infra `-e USER=...` at line 403. New args go
  between them: after the env files (so they beat `cast.env`), before the
  infra vars (so cast stays authoritative for its own names).
- Existing helper and test style to follow: `dev/env_file.rs`
  (`build_env_file_args`, pure, `Vec<String>`, tempdir-based tests).
- Tests must not read the real process environment — inject a host env
  map (`&BTreeMap<String, String>`) into the pure helper. This is also
  required for nix sandbox reproducibility.

TDD throughout: write the failing test, then the code.

## Steps

### Phase 1 — schema and merge

- [x] In `config/schema.rs`, add `#[serde(default)] pub env_passthrough:
    Vec<String>` to `Config`, and `Vec::new()` to the `Default` impl.
      **Shape decided by the user:** a list of names, not a map of name to
      bool, because a map reads as "set `GH_TOKEN` to `true`" and because
      an auditable single-file allowlist with fail-closed precedence beats
      a union of two files.
- [x] Test in `config/loader.rs`: global and project `cast.json` both
      contribute keys. **Resolved by adding
      `load_config_with_global(base_dir, Option<&Path>)`.**
      `figment::Jail` was tried first and rejected: it mutates
      process-global state (`set_var`, `set_current_dir`) and its lock
      only serialises jail-against-jail, not against the other ~245 tests
      running concurrently in the same binary, several of which read
      `$HOME`. That is the `getenv`/`setenv` race that made
      `std::env::set_var` unsafe in edition 2024. (AC 7)
- [x] Test: a project list replaces the global one, and the global list
      applies when the project config is silent. (AC 6)
- [x] Test in `config/approval.rs`: two configs differing only by an
      added `env_passthrough` key produce different hashes, and
      `get_approval_status_with` returns `ApprovalStatus::Changed`. (AC 8)
- [x] Test: serialized config contains the name and not the host
      value. (AC 9)
- [x] Test: `env_passthrough` defaults to empty and accepts multiple
      names. (AC 7)
- [x] **Dropped:** the `CAST_ENV_PASSTHROUGH__GH_TOKEN` nesting route.
      Not wanted, and advertising it created a self-inflicted problem:
      figment lowercases key paths derived from variable names, so the
      key arrived as `gh_token`. The single-variable form
      `CAST_ENV_PASSTHROUGH={GH_TOKEN=true}` works instead and preserves
      case, because figment's value parser handles compound literals
      (`value/parse.rs:169-183`) and only the name-derived key path is
      lowercased. No uppercase normalisation is needed in phase 2.

### Phase 2 — pure helper

- [x] In `dev/env_file.rs`, add:
      `pub fn build_env_passthrough_args(allowlist: &[String], host_env:
    &BTreeMap<String, String>) -> Vec<String>`. Returns
      `["-e", "NAME", ...]`. Signature emits names only — this is what
      satisfies AC 10 structurally.
- [x] Duplicate handling: dedupe via `BTreeSet`, which also gives the
      sort for free. A name listed twice emits one `-e` pair.

- [x] Test: enabled name present in host env yields `["-e",
    "GH_TOKEN"]`. (AC 1)
- [x] Test: no emitted arg contains the secret value. (AC 2)
- [x] Test: name absent from host env yields nothing. (AC 4)
- [x] Test: empty name, and names failing
      `[A-Za-z_][A-Za-z0-9_]*`, yield nothing. (AC 4)
- [x] Test: multiple names emit sorted by name, independent of the order
      they appear in `cast.json`. (AC 5) Sort is by byte, so `_PRIVATE`
      follows `VAR2` — pinned by test.
- [x] Added a `debug!` logging only the emitted names. Never the values.

### Phase 3 — wiring

- [x] **Superseded: no `RunOpts` field.** Threaded
      `host_env_names: &BTreeSet<String>` through
      `build_docker_run_flags` as a parameter instead. `RunOpts` reaches
      ~20 functions across 7 files and is the only struct in `dev/`
      without `#[derive(Debug)]`; a field there would make one added
      `derive` plus one `debug!` into a host-environment dump. Follows
      the existing precedent at `run.rs:151`, where `run_in_container`
      already builds a transient env map for
      `Agent::env_passthrough_args` and keeps it out of `RunOpts`.
- [x] Narrowed the helper to names only (`be7585a`), severing the value
      channel at the earliest point.
- [x] Built the name set from the existing snapshot, filtered to
      non-empty values. That filter can only live at the boundary that
      holds values: `-e NAME` beats `--env-file`, so a set-but-empty host
      var would silently shadow a real `cast.env` value.
- [x] Called `build_env_passthrough_args` in `build_docker_run_flags`
      before the infra vars. **Ordering contract corrected:** docker
      collects all `--env-file` entries then appends all `--env`, last
      wins, so passthrough beats `cast.env` regardless of argv position.
      Only preceding cast's own `-e USER=` is load-bearing.
- [x] Test: allowlisted name emits valueless `-e NAME`, is omitted when
      unset on the host, and precedes `USER=alice`. (AC 3) These are the
      first positional assertions in the file; every pre-existing
      `build_docker_run_flags` test uses `contains`.
- [x] Documented the child-inherits-env invariant on both spawn sites in
      `docker/client.rs`: a future `env_clear()` would silently break
      passthrough, including the provider API keys that already rely on
      it.

### Phase 4 — docs and close-out

- [x] Updated `crates/cast/docs/config/env-overrides.md` (`ae565f8`): the
      `env_passthrough` list, that values come from the host shell and are
      never stored, that a project list replaces rather than extends the
      global one, and that adding a name requires `cast config allow`.
      Also documented the corrected precedence rule (docker appends all
      `--env` after all `--env-file`, last wins) and the skipping of
      set-but-empty names. (AC 12)
- [x] Added the trust-boundary note: approval gates which names cross the
      boundary, not what the shell put in them, with the reasoning that an
      attacker who can overwrite the value already has host code
      execution.
- [x] `crates/cast/docs/README.md` needs no new entry: it links
      `config/overview.md` as the section root and does not enumerate
      individual config pages. Added a cross-reference from
      `config/reference.md` instead, next to `forbidden_paths`.
- [x] `cargo test -p cast` (256 lib + 18 + 9 green) and `cargo clippy
    --all-targets` clean. (AC 11)
- [x] Filled Evidence fields in the task card and corrected two stale
      Design claims there: `Command::env` at the spawn site (superseded —
      the child inherits) and "emitted after `--env-file` so they
      override" (true, but not for the positional reason given).
- [ ] **Blocked on manual QA.** The task stays `in-progress` until the
      user works through
      `.cue/env-var-passthrough/todo/1786901418-ae565f8/manual-qa.md` and
      signs off. Added as AC 13. AC 10 is met structurally by the
      `&BTreeSet<String>` signature rather than by attestation, so that
      one needs no sign-off.
