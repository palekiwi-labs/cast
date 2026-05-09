# Execution Roadmap: Built-in MCP Server

## Slice 1: Configuration Schema & Dependencies
- [ ] Update `Cargo.toml` with `mcp` feature (default) and optional dependencies (`rmcp`, `tokio`, `axum`, `jsonschema`).
- [ ] **RED**: Write tests for deserializing `McpConfig`, `McpToolConfig`, and the heterogeneous `ArgTemplate` array (handling both literal strings and conditional objects).
- [ ] **GREEN**: Implement `src/config/schema.rs` additions to make tests pass.
- [ ] **REFACTOR**: Ensure the struct layout is clean and idiomatic.
- [ ] Commit: `feat(config): add mcp tool configuration schema`

## Slice 2: Secure Execution Engine - Argument Mapper
- [ ] **RED**: Write unit tests for mapping `Vec<ArgTemplate>` to `Vec<String>`. Test literal substitutions (`{var}`), spread operators (`{...array}`), and conditional evaluation (`if_present`, `if_true`) based on a sample JSON input.
- [ ] **GREEN**: Implement the argument mapper logic in `src/commands/mcp/exec.rs`.
- [ ] **REFACTOR**: Extract evaluation logic into testable helper functions if needed.
- [ ] Commit: `feat(mcp): implement array-aware parameter mapper`

## Slice 3: Secure Execution Engine - Sandbox
- [ ] **RED**: Write tests verifying the `Command` builder logic: ensuring `.env_clear()` is applied, `PATH` is retained, and whitelisted/static variables (`inherit`, `set`) are mapped correctly. *(Ensure tests comply with Nix sandbox constraints)*.
- [ ] **GREEN**: Implement subprocess execution wrapped in `tokio::task::spawn_blocking` capturing stdout/stderr.
- [ ] **REFACTOR**: Ensure error handling wraps process failures clearly.
- [ ] Commit: `feat(mcp): implement secure subprocess execution sandbox`

## Slice 4: Dynamic MCP Handler
- [ ] **RED**: Write tests for converting `McpToolConfig` into `rmcp::Tool` definitions and testing `jsonschema` validation of mock `request.arguments`.
- [ ] **GREEN**: Implement manual `ServerHandler` (`list_tools` and `call_tool`) in `src/commands/mcp/handler.rs`.
- [ ] **REFACTOR**: Clean up json parsing and error mapping to MCP standard errors.
- [ ] Commit: `feat(mcp): implement dynamic tool router and schema validation`

## Slice 5: Server Infrastructure & CLI Wire-up
- [ ] Implement `tokio` runtime and `axum` server setup configuring `host.docker.internal` in `src/commands/mcp/server.rs`.
- [ ] Register `Mcp` subcommand in `src/commands/cli.rs`.
- [ ] Implement `ApprovedConfig` verification gate in `src/commands/mod.rs` for `mcp start`.
- [ ] Commit: `feat(cli): add mcp start subcommand with approved config gate`
