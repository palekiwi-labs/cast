# Execution Roadmap: Built-in MCP Server

## Slice 1: Configuration Schema & Dependencies
- [x] Update `Cargo.toml` with `mcp` feature (default) and optional dependencies (`rmcp`, `tokio`, `axum`, `jsonschema`).
- [x] **RED**: Write tests for deserializing `McpConfig`, `McpToolConfig`, and the heterogeneous `ArgTemplate` array (handling both literal strings and conditional objects).
- [x] **GREEN**: Implement `src/config/schema.rs` additions to make tests pass.
- [x] **REFACTOR**: Ensure the struct layout is clean and idiomatic.
- [x] Commit: `feat(config): add mcp tool configuration schema`

## Slice 2: Secure Execution Engine - Argument Mapper
- [x] **RED**: Write unit tests for mapping `Vec<ArgTemplate>` to `Vec<String>`. Test literal substitutions (`{var}`), spread operators (`{...array}`), and conditional evaluation (`if_present`, `if_true`) based on a sample JSON input.
- [x] **GREEN**: Implement the argument mapper logic in `src/commands/mcp/exec.rs`.
- [x] **REFACTOR**: Extract evaluation logic into testable helper functions if needed.
- [x] Commit: `feat(mcp): implement array-aware parameter mapper`
- [x] **FIX**: Apply logical AND to conditional evaluation and add `deny_unknown_fields` to schema.
- [x] Commit: `fix(mcp): ensure conditional arguments use logical AND and deny unknown fields`

## Slice 3: Secure Execution Engine - Sandbox
- [x] **RED**: Write pure unit tests for `resolve_env` (verifying PATH injection, inheritance, and overrides) and `build_exec_command`.
- [x] **GREEN**: Implement `resolve_env` and `build_exec_command` in `src/commands/mcp/exec.rs`.
- [x] **GREEN**: Implement the thin `run_command` executor using `tokio::process::Command` with `.env_clear()`.
- [x] **REFACTOR**: Ensure error handling wraps process failures clearly.
- [x] Commit: `feat(mcp): implement secure subprocess execution sandbox`
- [x] **FIX**: Apply refinements (TMPDIR retention, error handling, ref refactor, working_dir).
- [x] Commit: `fix(mcp): apply code review refinements for subprocess sandbox`
- [x] Commit: `feat(mcp): add working_dir support for tool isolation`

## Slice 4: Dynamic MCP Handler
- [x] **RED**: Write tests for converting `McpToolConfig` into `rmcp::Tool` definitions and testing `jsonschema` validation of mock `request.arguments`.
- [x] **GREEN**: Implement manual `ServerHandler` (`list_tools` and `call_tool`) in `src/commands/mcp/handler.rs`.
- [x] **REFACTOR**: Clean up json parsing and error mapping to MCP standard errors.
- [x] Commit: `feat: implement dynamic MCP handler (Slice 4)`

## Slice 4.5: Refinements & Fixes
- [x] Pre-compile `jsonschema` validators in `McpHandler` to avoid re-compilation on every request.
- [x] Correct "Tool Not Found" error code from `MethodNotFound` to `InvalidParams`.
- [x] Add direct unit tests for `handler.call_tool` exercising the full validation-to-execution pipeline.
- [x] Improve observability with `tracing::error!` logging for internal execution failures.

## Slice 5: Server Infrastructure & CLI Wire-up
- [x] Implement `tokio` runtime and `axum` server setup configuring `host.docker.internal` in `src/commands/mcp/server.rs`.
- [x] Register `Mcp` subcommand in `src/commands/cli.rs`.
- [x] Implement `ApprovedConfig` verification gate in `src/commands/mod.rs` for `mcp start`.
- [x] Commit: `feat(cli): add mcp start subcommand with approved config gate`
