# Project Log

## [c6794d5] Research complete: dead code scan post-nix-native-harnesses

Conducted a thorough scan of the codebase for dead code post-nix-native-harnesses pivot.

Key findings:
- `ureq` dependency in `crates/cast/Cargo.toml` is completely unused.
- `Agent::extra_run_args` references in `run.rs` documentation are outdated.
- `cast build <agent>` CLI surface is redundant as all agents share the same image and build logic.
- Outdated comment in `nix_daemon/daemon.rs` regarding image tag being based on assets hash.
- Legacy fields `agent_versions` and `universal_container` are explicitly ignored in tests, which is fine but worth noting.
- Doc comment in `Agent` trait mentions old behavior.

Full list of findings prepared for the report.

- **Found:** ureq dependency is dead in crates/cast/Cargo.toml
- **Found:** Agent::extra_run_args documentation reference is dead in crates/cast/src/dev/run.rs
- **Found:** build_agent takes unused _agent parameter in crates/cast/src/dev/build.rs
- **Found:** Redundant per-agent build CLI surface in crates/cast/src/commands/cli.rs
- **Found:** Outdated comment in crates/cast/src/nix_daemon/daemon.rs regarding assets hash tagging

