# Research Report: Rust CLI Patterns in `cast`

## Overview
This report documents the architectural and implementation patterns discovered in the `cast` codebase. These patterns prioritize security, testability, and a clean separation of concerns.

## 1. Project Structure
The project is a Rust workspace located in the `crates/` directory.
- **`crates/cast/`**: Primary crate for sandbox orchestration.
- **`crates/cast-mcp-client/`**: Lightweight MCP client.
- **`src/commands/`**: Contains thin CLI command handlers.
- **Domain Logic**: Logic is extracted into separate modules (e.g., `src/dev/`, `src/mcp/`) or crates.

## 2. Command Handler Pattern
Command handlers in `src/commands/` are "thin" and primarily responsible for argument parsing/validation and delegating to domain logic.

**Example: `crates/cast/src/commands/mcp.rs`**
```rust
pub async fn run(
    command: crate::commands::cli::McpCommands,
    approved: crate::config::ApprovedConfig,
) -> anyhow::Result<()> {
    match command {
        McpCommands::Start { port, host } => {
            let host = host.unwrap_or_else(|| approved.mcp.hostname.clone());
            let port = port.unwrap_or(approved.mcp.port);
            crate::mcp::server::run_http_server(host, port, approved).await
        }
    }
}
```

## 3. CLI Definition (`clap`)
The CLI is defined using `clap`'s derive API in `src/commands/cli.rs`.
- **Nested Enums**: Used for subcommands.
- **Trailing Arguments**: `trailing_var_arg = true` is used for passing flags to underlying agents.

```rust
#[derive(Parser)]
#[command(name = "cast", about, version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}
```

## 4. Error Handling and Logging
- **`anyhow`**: Used for error propagation. `.context("...")` is mandatory for providing user-facing explanations.
- **`tracing`**: Used for internal file-based logging (`init_file_logger`).
- **User Feedback**: `println!` for requested data (stdout); `eprintln!` for status/progress (stderr) to keep stdout clean for piping.

## 5. Configuration and Security (`ApprovedConfig`)
`cast` uses a "capability-based" security pattern for configuration.
- **Tiered Loading**: Handled by `figment` (Env > Files > Defaults).
- **`ApprovedConfig`**: A Newtype wrapper around `Config`. It can only be created by verifying a configuration hash against an approval store (`~/.local/share/cast/approved_configs.json`).
- **Enforcement**: Sensitive domain functions (like starting an agent) require an `&ApprovedConfig`, forcing the user to have run `cast config allow` first.

## 6. Testing Strategy (TDD)
### Unit Tests
Located within the same file as the code. Focus on testing "pure" functions.
- **Pure Logic**: Functions that transform data (e.g., building a list of shell arguments) are extracted from side-effectful code to avoid mocking.
  - **Example**: `crates/cast/src/dev/run.rs`: `build_run_opts` is tested by checking the returned `Vec<String>`.

### Integration Tests
Located in the `tests/` directory.
- **`assert_cmd`**: Used to execute the binary.
- **Isolation**: Environment variables (e.g., `CAST_LOG_DIR`, `CAST_DATA_DIR`) are overridden to `std::env::temp_dir()` to ensure tests don't interfere with the user's home directory and remain compatible with `nix build` sandboxes.

## 7. Documentation
Documentation follows a "Progressive Discovery" pattern:
- **Top-level**: `docs/README.md` provides an overview.
- **Per-crate**: `crates/<crate>/docs/README.md` provides crate-specific technical details.
- Each README acts as a Table of Contents for its directory.
