---
refs: task/update-dependencies-0.2.0/log.md
status: open
---
# Manual QA: Rust 1.98.0 toolchain pin (b985368)

Verify the toolchain pin on branch `chore/pin-rust-toolchain` (b985368),
checked out at the repository root. All commands run from the repository
root unless noted.

The canonical toolchain on this machine is Nix-managed (fenix via
`fromToolchainFile` in `flake.nix`). Do NOT run rustup or bare rustup-shimmed
cargo/rustc in this repo: `rust-toolchain.toml` would make rustup download a
full duplicate 1.98.0 toolchain that is never used. Run all cargo/rustc
commands inside the nix shell (`nix develop -c <cmd>`). The toolchain file
still benefits rustup-only contributors on other machines automatically.

## Toolchain resolution (Nix)

- [x] `nix develop -c rustc --version` reports 1.98.0
- [x] `nix develop -c cargo --version` reports 1.98.0
- [x] `nix develop -c cargo clippy --version` and
      `nix develop -c rustfmt --version` resolve (components present in
      the fenix derivation)

## Cargo workflows

Run inside the nix shell; prefix each with `nix develop -c ` or enter
`nix develop` once.

- [x] `cargo build` succeeds at root
- [x] `cargo test` passes (expect 328 unit/integration tests)
- [x] `cargo clippy --all-targets` passes without warnings
- [x] `cargo fmt --check` is clean
- [x] `cargo build` succeeds inside `worktrees/feat-cast-agent-mvp` and
      `worktrees/spike-herdr-service-pivot` (shared toolchain pin follows)

## Nix workflows

- [x] `nix build` succeeds for both package outputs (cast, cast-mcp-client)
- [x] `nix build` result binaries run: `./result/bin/cast --help` and the
      cast-mcp-client binary respond correctly
- [x] `nix flake check` passes (or note if not part of the usual flow)

## Functional smoke tests (installed binaries)

- [x] `cast --help` renders usage
- [x] `cast --version` (or equivalent) runs without error
- [x] A basic sandbox/shell operation from the docs quickstart still works
- [x] `cast-mcp-client` help and a trivial subcommand run without error

## MSRV metadata

- [ ] `nix develop -c cargo metadata` confirms `rust-version = "1.98"` in
      both crate manifests does not block resolution

## Outcome

- Record pass/fail per section and note any environment-specific issues.
  On full pass: the branch `chore/pin-rust-toolchain` (b985368) is ready to
  push, and this todo can be closed.
