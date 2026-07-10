# Research Report: Cast Run Options

## Overview
This report documents the current implementation of `cast run` and outlines the changes required to support headless mode, JSON output, and custom container management as per the `run-opts.md` specification.

## Findings

### 1. CLI Architecture
- **Location**: `crates/cast/src/commands/cli.rs`
- **Pattern**: `cast run` uses the `RunAgent` enum for agent-specific subcommands.
- **Current State**: Each variant (e.g., `RunAgent::Opencode`) only accepts trailing arguments via `trailing_var_arg = true`.
- **Snippet**:
```rust
pub enum RunAgent {
    /// Start an interactive OpenCode session
    #[command(alias = "o", disable_help_flag = true)]
    Opencode {
        /// Extra arguments to pass to the opencode command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
        extra_args: Vec<String>,
    },
    // ...
}
```

### 2. Container Execution Logic
- **Location**: `crates/cast/src/dev/run.rs`
- **Pattern**: `run_agent` orchestrates session parameters, resolves port/name, and calls `docker.interactive_command`.
- **Docker Flags**: `build_run_opts` hardcodes `-it` and `-p` (conditional on `config.publish_port`).
- **Snippet**:
```rust
pub fn build_run_opts(config: &Config, opts: &RunOpts) -> Vec<String> {
    let mut run_args: Vec<String> = vec![
        "--rm".to_string(),
        "-it".to_string(),
        // ...
    ];
    if config.publish_port {
        run_args.push("-p".to_string());
        run_args.push(format!("{}:80", opts.port));
    }
    // ...
}
```

### 3. Docker Client Capabilities
- **Location**: `crates/cast/src/docker/client.rs`
- **Current State**: `DockerClient` only supports interactive execution via `interactive_command`, which inherits TTY and masks signals.
- **Missing**: A non-interactive execution method that correctly pipes `stdout` for JSON consumption.

### 4. Agent Command Construction
- **Location**: `crates/cast/src/dev/build_command.rs`
- **Pattern**: Wraps the base command (e.g., `opencode`) in `nix develop` shells.
- **Requirement**: Needs to pass `--headless` and `--json` (or equivalent flags like `--format json` for `opencode`) to the inner command.

## Proposed Questions for Consultant

1. **Shared Flags Pattern**: Should we use `clap`'s `flatten` attribute to share flags like `--headless`, `--json`, `--name`, and `--no-publish` across all agent subcommands?
2. **`run` vs `exec` Strategy**: If a user runs a headless command while an interactive session is already active, should we default to `docker run` (independence) or support `docker exec` (shared context)?
3. **Container Name Conflicts**: If using `docker run` for headless sessions, how should we resolve name conflicts with existing interactive sessions? Should we append a `-headless` or `-<id>` suffix?
4. **TTY and JSON Output**: For `--json` mode, we must omit `-it` to avoid TTY-specific control characters in the output. Does `DockerClient` need a specialized `query_json` method that ensures clean stdout?
5. **Port Mapping defaults**: Should `--headless` automatically disable port publishing (`-p`) to prevent conflicts with the primary interactive session?

## Sourced Files
- `crates/cast/src/commands/cli.rs`
- `crates/cast/src/dev/run.rs`
- `crates/cast/src/dev/agent.rs`
- `crates/cast/src/dev/build_command.rs`
- `crates/cast/src/docker/client.rs`
- `crates/cast/src/docker/args.rs`
