---
refs: task/update-dependencies-0.2.0/log.md
status: open
---
# Manual QA: Rust 1.98.0 toolchain pin (b985368)

Verify the toolchain pin now living on master at the root checkout.
All commands run from the repository root unless noted.

## Toolchain resolution

- [ ] `rustup show` reports an active toolchain of 1.98.0 pulled from
      `rust-toolchain.toml` (not the system default)
- [ ] `rustc --version` and `cargo --version` both report 1.98.0
- [ ] `rustup component list --installed` includes rustc, cargo, clippy,
      rustfmt, and rust-src (matches `rust-toolchain.toml` components)
- [ ] From a clean shell with no rustup overrides: `cd` into the repo and
      confirm the toolchain file is honored automatically

## Cargo workflows

- [ ] `cargo build` succeeds at root
- [ ] `cargo test` passes (expect 328 unit/integration tests)
- [ ] `cargo clippy --all-targets` passes without warnings
- [ ] `cargo fmt --check` is clean
- [ ] `cargo build` succeeds inside `worktrees/feat-cast-agent-mvp` and
      `worktrees/spike-herdr-service-pivot` (shared toolchain pin follows)

## Nix workflows

- [ ] `nix build` succeeds for both package outputs (cast, cast-mcp-client)
- [ ] `nix build` result binaries run: `./result/bin/cast --help` and the
      cast-mcp-client binary respond correctly
- [ ] `nix develop` enters a shell where `rustc --version` reports 1.98.0
- [ ] `nix flake check` passes (or note if not part of the usual flow)

## Functional smoke tests (installed binaries)

- [ ] `cast --help` renders usage
- [ ] `cast --version` (or equivalent) runs without error
- [ ] A basic sandbox/shell operation from the docs quickstart still works
- [ ] `cast-mcp-client` help and a trivial subcommand run without error

## MSRV metadata

- [ ] `cargo metadata` or a build on a machine with only the pinned
      toolchain confirms `rust-version = "1.98"` in both crate manifests
      does not block resolution

## Outcome

- Record pass/fail per section and note any environment-specific issues.
  On full pass: master (b985368) is ready to push and this todo can be
  closed.
