# Project Log

## [660ec83] Commit: host module with hostname resolution

Added src/host/mod.rs implementing get_hostname() via libc::gethostname.
The pure normalize_hostname core handles empty/whitespace -> "unknown"
sentinel, trimming, and FQDN passthrough. The syscall wrapper uses libc
(already a dep) rather than the hostname binary, which is absent from the
Nix build sandbox stdenv. Smoke test asserts only on the non-empty
invariant, not ambient system state.

- **Found:** The cast Nix package has empty nativeCheckInputs; id works only because coreutils ships in stdenv; hostname is a separate package not in stdenv
- **Found:** POSIX does not guarantee NUL-termination when gethostname fills the buffer; capped NUL search at buf.len()
- **Found:** HOST_NAME_MAX is 64 on Linux; 256-byte buffer is a safe over-allocation
- **Decided:** Use libc::gethostname over the hostname binary (absent from Nix stdenv)
- **Decided:** normalize_hostname pure core maps empty/whitespace to 'unknown' sentinel
- **Decided:** Smoke test asserts non-empty invariant only, not specific hostname value

## [64310bf] Commit: inject CAST_HOST_NAME into containers

Threaded host_name through RunOpts and injected CAST_HOST_NAME in
build_docker_run_flags. resolve_run_opts soft-fails to 'unknown' + warn!
since hostname is metadata, not load-bearing. Updated all 14 literal
RunOpts constructions. Added tests for resolve_run_opts host_name
population and CAST_HOST_NAME injection in both interactive and headless
modes.

- **Found:** All 14 RunOpts literal constructions compile-checked via the struct field ripple
- **Found:** build_docker_run_flags has 20+ existing deterministic unit tests; adding host_name as data preserves determinism
- **Decided:** host_name resolved upstream in resolve_run_opts, not in build_docker_run_flags, to preserve the pure function
- **Decided:** Soft-fail to 'unknown' + tracing::warn! rather than hard bail (hostname is metadata)
- **Decided:** CAST_HOST_NAME injected adjacent to CAST_MCP_URL, after --env-file so it wins over cast.env

## [7f9e439] Feature complete: CAST_HOST_NAME injection

Completed all six phases of the CAST_HOST_NAME feature. Three commits on
feat/cast-host-name: (1) host module with normalize_hostname pure core +
libc::gethostname wrapper, (2) RunOpts threading + soft-fail resolution +
flag injection in build_docker_run_flags, (3) concepts.md documentation.
All 246 unit tests + integration tests pass; clippy clean. Pre-existing
cargo fmt debt confirmed on master and left out of scope.

- **Found:** cargo fmt --check shows pre-existing formatting debt on master across 8 files (import ordering, assertion wrapping) - unrelated to this feature, left out of scope
- **Decided:** All design decisions from the Opus consultation held: libc::gethostname, always-on, soft-fail to 'unknown', CAST_HOST_NAME naming

## [7f9e439] Task complete: link-containers-with-host

Master task `link-containers-with-host.md` advanced to `status: complete` with
`branch:` cleared. All five acceptance criteria evidence cells filled:
1. CAST_HOST_NAME=pale confirmed live in-session + user attested
2. Interactive + headless injection unit tests pass
3. resolve_run_opts soft-fail non-empty invariant test passes
4. 246 unit tests + integration tests pass, clippy clean, Nix-compatible design
5. concepts.md "Host Identity" section added

Consultant-opus review verdict: APPROVE WITH NITS (two non-blocking issues:
--env-file comment precision, optional pure helper for byte-parsing test
coverage). User directed to ship.

- **Decided:** Ship the feature as-is; review nits are non-blocking and can be addressed post-merge

