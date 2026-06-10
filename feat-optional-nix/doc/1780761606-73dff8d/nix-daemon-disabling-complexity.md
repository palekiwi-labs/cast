# Research: Disabling Nix/nix-daemon Feature

This report analyzes the complexity of allowing users to disable the Nix integration in `cast` via a `"nix": false` configuration setting.

## Research Question
What would be the complexity of allowing users to turn the nix-daemon feature off with `"nix": false` in the config?

## Current State of Nix Integration

Nix is currently a mandatory component of the `cast` orchestration layer. It is used to provide a consistent development environment via Flakes and a shared Nix store.

### Key Integration Points

1.  **Daemon Lifecycle Management**
    *   **File**: `crates/cast/src/dev/run.rs`
    *   **Logic**: Every agent session start calls `nix_daemon::ensure_running` unconditionally.
    ```rust
    // crates/cast/src/dev/run.rs:63
    nix_daemon::ensure_running(&docker, config)?;
    ```

2.  **Container Volume Mounts**
    *   **File**: `crates/cast/src/dev/run.rs`
    *   **Logic**: The Nix store volume (`/nix`) is mounted into every agent container as read-only.
    ```rust
    // crates/cast/src/dev/run.rs:173-176
    run_args.extend([
        "-v".to_string(),
        format!("{}:/nix:ro", config.nix_volume_name),
    ]);
    ```

3.  **Command Execution Wrapping**
    *   **File**: `crates/cast/src/dev/build_command.rs`
    *   **Logic**: Commands are wrapped in `nix develop <flake> -c` if flakes are detected or enabled in configuration.

4.  **Agent Image Configuration**
    *   **File**: `crates/cast/assets/Dockerfile.dev.opencode`
    *   **Logic**: Dockerfiles for agent images bake in Nix-specific environment variables.
    ```dockerfile
    ENV PATH="/nix/var/nix/profiles/default/bin:${PATH}"
    ENV NIX_REMOTE=daemon
    ```

## Complexity Analysis

Allowing `"nix": false` involves several changes across the codebase.

### 1. Configuration Schema (Low Complexity)
The `Config` struct in `crates/cast/src/config/schema.rs` needs a new field:
```rust
pub struct Config {
    #[serde(default = "default_true")]
    pub nix: bool,
    // ...
}
```

### 2. Orchestration Logic (Low Complexity)
In `crates/cast/src/dev/run.rs`, the daemon startup and volume mounting logic must be gated:
*   Skip `nix_daemon::ensure_running` if `config.nix` is false.
*   Skip the `/nix` volume mount if `config.nix` is false.

### 3. Command Builder (Low Complexity)
In `crates/cast/src/dev/build_command.rs`, ensure that no `nix develop` wrapping occurs if `config.nix` is false, even if `use_flake` is true or a `flake.nix` exists.

### 4. Agent Container Environment (Medium Complexity)
The primary complication is that the agent Docker images are built with Nix-specific environment variables (`PATH`, `NIX_REMOTE`). 
*   **Issue**: If the `/nix` volume is missing, these `PATH` entries point to non-existent locations.
*   **Mitigation**: While not strictly "broken" (empty PATH entries are ignored), it's untidy. A cleaner approach would be to move these environment variables from the `Dockerfile` to the `docker run` command in `run.rs`, making them conditional on `config.nix`.

## Summary Table

| Component | Files | Complexity |
| :--- | :--- | :--- |
| **Config Schema** | `crates/cast/src/config/schema.rs` | Low |
| **Daemon Startup** | `crates/cast/src/dev/run.rs` | Low |
| **Volume Mounts** | `crates/cast/src/dev/run.rs` | Low |
| **Command Wrapping** | `crates/cast/src/dev/build_command.rs` | Low |
| **Env Cleanup** | `Dockerfile.dev.opencode`, `run.rs` | Medium |

**Overall Complexity: Low-Medium**
The implementation is straightforward "if-gating" of existing logic. The main effort is ensuring the transition is clean (handling environment variables) and that error messages are helpful if a user tries to use Nix features when disabled.
