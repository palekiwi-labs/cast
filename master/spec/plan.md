# Rust rewrite of ocx - Phased Implementation Plan

## Phase 1: Project Scaffolding & CLI Structure
- Initialize Project: Setup the Rust project, along with `flake.nix` for the Rust environment.
- Crate Integration: Add the crates specified in the doc (`clap`, `figment`, `serde`, `thiserror`, `anyhow`, `tracing`, `whoami`).
- Module Skeleton: Scaffold the src/ directory exactly as outlined (config, docker, security, workspace, utils).
- CLI Definition: Implement the clap parser to define:
  - `ocx opencode` (alias o)
  - `ocx run [command]`
  - `ocx nix {start, stop, prune}`

## Phase 2: Configuration & State Management
- Schema Definition: Define the OcxConfig struct representing ocx.json.
- Figment Integration: Implement the loading precedence: Hardcoded Defaults -> ~/.config/ocx/ocx.json -> ./ocx.json -> OCX_ Env Vars.
- Tracing/Logging: Hook up tracing-subscriber to enable verbose/debug output when running the CLI to help test the config loader.

## Phase 3: Core Utilities (Workspace & Security)
- Workspace Mapping: Implement the logic to map $CWD to the correct container path (mirroring $HOME or defaulting to /workspace).
- Deterministic Port Mapping: Write the hashing utility that converts the project path into a deterministic port range to avoid local conflicts.
- Security Mounts: Implement the capability dropping (--cap-drop ALL), user mapping (via the whoami crate), and Shadow Mounts (generating --tmpfs and /dev/null mounts for sensitive paths).

## Phase 4: Nix Daemon Orchestration
- Daemon State Detection: Use std::process::Command to run `docker ps` and check if the daemon is currently running.
- Lifecycle Commands:
  - Start: Build and execute the `docker run` command with the ocx-nix-store volume.
  - Stop: Execute `docker stop ocx-nix-daemon`.
  - Prune: Execute `docker exec ocx-nix-daemon nix-store --gc`.

## Phase 5: Dev Container Execution (opencode / run)
- Command Assembly: Build the massive `docker run` command by aggregating:
  - The resolved OcxConfig (image, CPUs, memory, ports).
  - Security boundaries and shadow mounts.
  - Volume mounts (Workspace + Nix Store via daemon socket).
- Interactive TTY: Configure STDIN/STDOUT inheritance correctly for the `ocx opencode` interactive shell (`nix develop`).
- Headless Execution: Handle argument passing for `ocx run [command]`.

## Phase 6: Parity Review & Polish
- Cross-reference: Review the original .nu files (like shadow_mounts.nu, opencode_env.nu) to capture any subtle edge cases.
- Error Handling: Polish user-facing errors using anyhow contexts so users get actionable Docker/Nix setup errors.
- Testing: Write unit tests for the configuration hierarchy, the port hasher, and the docker command string builder.