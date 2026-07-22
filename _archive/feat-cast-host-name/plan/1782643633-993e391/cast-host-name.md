---
status: complete
refs:
- .cue/feat-cast-host-name/plan/index.md
- .cue/master/task/1782643633-993e391/link-containers-with-host.md
---
# Executive Plan: Inject CAST_HOST_NAME

## Foreword

This plan implements the full feature described in the master plan
`plan/index.md`. It covers all six phases: the pure normalization core,
the syscall wrapper, the `RunOpts` threading, the soft-fail wiring, the
flag injection, and docs. Each step is a discrete TDD red-green-commit
cycle. Prerequisites: the design decisions are locked (see master plan);
`libc` is already a dependency.

## Steps

### Phase 1 — Pure normalize_hostname core

- [x] 1.1 RED: write `normalize_hostname` tests in `src/host/mod.rs`
      (empty -> "unknown", whitespace -> "unknown", "myhost" passthrough,
      FQDN passthrough, trim)
- [x] 1.2 GREEN: implement `normalize_hostname` to pass; commit

### Phase 2 — gethostname syscall wrapper

- [x] 2.1 Implement `gethostname_impl` (unsafe libc) + `get_hostname()`
      public fn using `normalize_hostname`
- [x] 2.2 RED/GREEN: lenient smoke test (asserts non-empty after
      normalization, never a specific value); register module in lib.rs;
      commit

### Phase 3 — RunOpts threading

- [x] 3.1 Add `host_name: String` field to `RunOpts` in run.rs
- [x] 3.2 Update `resolve_run_opts` to set the field (hardcoded test value
      for now; real resolution in phase 4)
- [x] 3.3 Mechanically update all ~14 literal `RunOpts { ... }`
      constructions with `host_name: "test-host".to_string()`; compile +
      existing tests green; commit

### Phase 4 — Soft-fail resolution

- [x] 4.1 RED: write test asserting `resolve_run_opts` yields a non-empty
      `host_name` (the real hostname or "unknown")
- [x] 4.2 GREEN: call `get_hostname()` in `resolve_run_opts` with
      `unwrap_or_else` -> "unknown" + `warn!`; commit

### Phase 5 — Flag injection

- [x] 5.1 RED: write tests asserting `CAST_HOST_NAME=<value>` present in
      interactive and headless output of `build_docker_run_flags`
- [x] 5.2 GREEN: inject `-e CAST_HOST_NAME={opts.host_name}` adjacent to
      `CAST_MCP_URL`; commit

### Phase 6 — Docs

- [x] 6.1 Document `CAST_HOST_NAME` in concepts.md "Persistence and Host
      Access" section; commit
