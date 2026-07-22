# Project Log

## [73dff8d] Research complete: Complexity of disabling nix-daemon feature

- **Found:** Nix is currently a hard dependency in `crates/cast/src/dev/run.rs`, starting the daemon and mounting the volume unconditionally.
- **Found:** Agent Docker images bake in `PATH` and `NIX_REMOTE` environment variables assuming Nix is present.
- **Found:** Allowing `"nix": false` would require gating logic in `run.rs` and `build_command.rs`, and moving Nix-specific env vars from Dockerfiles to runtime configuration.

