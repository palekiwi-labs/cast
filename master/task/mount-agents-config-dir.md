---
title: Mount ~/.agents config directory by default in cast sandboxes
status: complete
priority: normal
refs:
  - ~/code/palekiwi/palekiwi/.cue/master/doc/agent-harness-config.md
branch:
  - feat/mount-agents-config-dir
---
# Mount ~/.agents Config Directory by Default

Mount host `~/.agents` into container `/home/<user>/.agents` by default in `cast` sandboxes, providing cross-harness global skill and configuration sharing across agent sessions.

## Context & Rationale

Cross-harness research (`.cue/master/doc/agent-harness-config.md`) identified `.agents/` as the open vendor-neutral standard (`agentskills.io`) natively supported by OpenCode and Pi for global skills (`~/.agents/skills/`).

Currently, `cast` bind-mounts harness-specific global config directories for each agent:
- OpenCode: `~/.config/opencode` -> `/home/<user>/.config/opencode`
- Pi: `~/.pi` -> `/home/<user>/.pi`
- Claude Code: `~/.claude` -> `/home/<user>/.claude` and `~/.claude.json`

Because `~/.agents` is omitted, global skills stored in `~/.agents/skills` on the host are invisible inside `cast` sandboxes. Adding `~/.agents` to default volume mounts ensures seamless cross-harness skill availability across all agent sessions.

## Scope & Implementation Details

1. **Host Directory Preparation (`crates/cast/src/dev/`)**:
   - Call `ensure_config_dir(&home.join(".agents"))` during host preparation prior to starting containers so missing host directories are created with correct user ownership instead of Docker creating root-owned paths.

2. **Default Volume Mounting**:
   - Add bind mount `-v /home/<user>/.agents:/home/<user>/.agents:rw` in data volume construction (`crates/cast/src/dev/universal/volumes.rs` and relevant single-agent run modules).

3. **Dockerfile & Permissions (`assets/Dockerfile.dev`)**:
   - Ensure `/home/${USERNAME}/.agents` is included in pre-created user directories and permission fixes alongside `.claude`, `.pi`, and `.config`.

4. **Tests**:
   - Add unit tests verifying `~/.agents` is included in config mount arguments and universal volume mounts.

## Acceptance Criteria

1. **`~/.agents` is prepared on the host and mounted into container `/home/<user>/.agents:rw` by default.**
   - Verify by: `cargo test` in `crates/cast`
   - Evidence: `cargo test -p cast --lib` -> 235 passed; 0 failed. Mount verified
     by `universal::volumes::tests::universal_run_args_includes_cross_harness_agents_mount`;
     host prep by `universal::config_dir::tests::ensure_config_dir_creates_directory`.

2. **Permissions for `/home/<user>/.agents` are properly initialized in `Dockerfile.dev`.**
   - Verify by: `cargo test` / inspection of Dockerfile
   - Evidence: `image::tests::dev_dockerfile_creates_and_chowns_cross_harness_agents_dir`
     asserts `.agents` appears in both the mkdir and chown lists; Dockerfile.dev
     updated accordingly.
