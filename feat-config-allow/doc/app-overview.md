# High-Level Overview: Cast

## Summary
`cast` (**Coding Agent Sandbox Tool**) is a Rust-based CLI utility designed to manage and run AI coding agents (specifically `OpenCode` and `Pi`) within secure, reproducible Docker containers. It leverages Nix to provide consistent development environments (`nix develop`) inside these containers.

## Core Purpose
The application automates the setup and execution of coding agents, ensuring they have access to the correct tools and dependencies by wrapping their commands in Nix shells. It manages the lifecycle of these environments using Docker.

## Key Components

### 1. Agent Management
The application uses an `Agent` trait to abstract the differences between various coding agents. This allows for a consistent interface to resolve versions, generate Dockerfiles, and define execution arguments.

- **Source**: `/home/pl/code/palekiwi-labs/cast/src/dev/agent.rs`
- **Snippet**:
```rust
pub trait Agent {
    fn name(&self) -> &'static str;
    fn dockerfile(&self) -> &'static str;
    fn resolve_version(&self, config: &Config) -> Result<String>;
    fn extra_run_args(
        &self,
        config: &Config,
        opts: &RunOpts,
        env: &HashMap<String, String>,
    ) -> Result<Vec<String>>;
    fn base_command(&self) -> &'static str;
}
```

### 2. Environment Wrapping (Nix)
A primary feature of `cast` is its ability to wrap agent execution commands inside one or more `nix develop` layers. This ensures that the agent runs in a shell with all required dependencies as defined in Nix flakes.

- **Source**: `/home/pl/code/palekiwi-labs/cast/src/dev/build_command.rs`
- **Snippet**:
```rust
pub fn build_command(...) -> Vec<String> {
    // ... logic to check for global and project flakes ...
    if opts.user_flake_present {
        cmd.extend(["nix".to_string(), "develop".to_string(), global_flake, "-c".to_string()]);
    }
    if let Some(flake_ref) = project_flake {
        cmd.extend(["nix".to_string(), "develop".to_string(), flake_ref.to_string(), "-c".to_string()]);
    }
    cmd.push(base_command.to_string());
    cmd.extend(extra_args);
    cmd
}
```

### 3. CLI Interface
The app uses `clap` to provide a robust CLI for interacting with agents and managing the sandbox.

- **Source**: `/home/pl/code/palekiwi-labs/cast/src/commands/cli.rs`
- **Key Subcommands**:
  - `run`: Executes an agent.
  - `build`: Prepares the sandbox environment.
  - `shell`: Opens a shell inside the sandbox.
  - `nix-daemon`: Manages the shared Nix daemon.

## Infrastructure & Lifecycle
- **Docker**: Used for process isolation and environment reproducibility.
- **Nix Daemon**: A specialized container manages a shared `/nix/store` to optimize builds and environment setup across agent runs (`src/nix_daemon/`).
- **Persistence**: Managed through Docker volumes and bind-mounts for caches and configuration.
