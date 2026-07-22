---
status: complete
refs: .cue/master/task/1782643633-993e391/link-containers-with-host.md
---
# Master Plan: Inject host name into containers

## Problem

The host machine's name is not currently available inside `cast` sandbox
containers. Diagnostics collected in sandboxes cannot be grouped by host
because the container has no knowledge of which host launched it.

Implements: task/1782643633-993e391/link-containers-with-host.md

## Approach

Inject the host's kernel hostname as the environment variable
`CAST_HOST_NAME` into every agent sandbox container (`cast run` and
`cast exec`), following the same pattern as the existing `USER` and
`CAST_MCP_URL` injections.

## Design decisions (locked)

1. **Hostname source: `libc::gethostname`.** `libc` is already a dependency
   (zero `Cargo.lock` churn). The `hostname` binary is rejected because it is
   NOT part of coreutils/stdenv and is absent in the Nix build sandbox, which
   would break reproducible test runs. The `unsafe` FFI is contained in one
   auditable safe wrapper.

2. **Always-on, no config field.** `CAST_HOST_NAME` is injected
   unconditionally for every sandbox, matching the tier of `USER` and
   `CAST_MCP_URL`. Diagnostics grouping requires uniform presence. A config
   override can be revisited later via cast.env precedence.

3. **Soft-fail to `"unknown"`.** Hostname resolution is diagnostic metadata,
   not load-bearing for the sandbox. If `gethostname` fails or returns an
   empty string, the session proceeds with `CAST_HOST_NAME=unknown` and a
   `tracing::warn!`. This deliberately diverges from `get_user()`'s hard-fail
   because a missing user breaks the sandbox; a missing hostname does not.

4. **Env var name: `CAST_HOST_NAME`.** Uses the `_NAME` split rather than the
   single token `CAST_HOSTNAME` because the codebase already overloads
   "hostname" to mean the MCP bind address (`mcp.hostname`,
   `CAST_MCP__HOSTNAME`). `CAST_HOST` is rejected as dangerously ambiguous
   (collides with the host:port convention).

## Architecture

The codebase uses a functional-core / imperative-shell split:
`build_docker_run_flags(config, opts)` is a **pure** function (no I/O, 20+
deterministic unit tests). Host facts are resolved upstream and passed down
as data via the `RunOpts` struct. This design preserves that purity:

- New module `src/host/mod.rs` mirrors `src/user/mod.rs`: a `get_hostname()`
  imperative-shell function plus a **pure** `normalize_hostname(&str)`
  policy core that is fully unit-testable.
- `RunOpts` gains a `host_name: String` field, resolved in
  `resolve_run_opts` and read by `build_docker_run_flags`.

### Pitfalls handled

- **NUL-termination**: POSIX does not guarantee a NUL if the name fills the
  buffer; the wrapper caps the NUL search at `buf.len()` and uses a 256-byte
  buffer (`HOST_NAME_MAX` is 64).
- **Empty hostname**: a host can have `""`; `normalize_hostname` maps it to
  the `"unknown"` sentinel.
- **Short vs FQDN**: `gethostname` returns whatever the kernel has set
  (usually short). We accept it as-is and document that it may be short or
  FQDN. No DNS resolution attempted (non-reproducible).

## Phases

1. Pure `normalize_hostname` core + tests (deterministic).
2. `gethostname_impl` / `get_hostname` safe wrapper + smoke test.
3. Add `host_name: String` to `RunOpts`; mechanically update all ~14
   literal constructions across run.rs, exec.rs, build_command.rs, and the
   three agent modules.
4. Wire `resolve_run_opts` with soft-fail fallback to `"unknown"` + `warn!`.
5. Inject `-e CAST_HOST_NAME` in `build_docker_run_flags` + flag tests.
6. Docs: document `CAST_HOST_NAME` in concepts.md.

## Scope

Agent sandbox containers only (`run` + `exec`). The Nix daemon helper
container is out of scope (it injects no such env vars and is not a
diagnostics target).
