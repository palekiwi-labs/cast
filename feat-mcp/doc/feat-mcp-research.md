# Research: Built-in MCP Server for `cast`

This report documents the research conducted for implementing a built-in MCP (Model Context Protocol) server in `cast`.

## Research Questions Answered

1. **How to integrate a new `mcp` subcommand?**
   - Use `clap` derive in `src/commands/cli.rs`.
   - Register in `src/commands/mod.rs`.
   - Pattern: `cast mcp start --port <PORT>`.

2. **How to define commands dynamically?**
   - Instead of a "command interceptor" that parses shell strings, use **Semantic MCP Tools**.
   - Tools are defined in `cast.json` with structured JSON schemas.
   - Example config structure:
     ```json
     "mcp": {
       "tools": {
         "run_rspec": {
           "description": "Run RSpec tests",
           "host_cmd": ["docker", "compose", "exec", "test", "bundle", "exec", "rspec"],
           "parameters": {
             "type": "object",
             "properties": {
               "test_paths": { "type": "array", "items": { "type": "string", "pattern": "^spec/.*_spec\\.rb$" } }
             }
           }
         }
       }
     }
     ```

3. **How to implement dynamic tools with `rmcp`?**
   - Bypass `#[tool_router]` macro.
   - Manually implement `ServerHandler` trait in `src/commands/mcp/handler.rs`.
   - Use `Tool::new_with_raw` to instantiate tools from config schemas.
   - Use `CallToolRequestParams` to access raw `serde_json` arguments.

4. **What are the security implications?**
   - **Flag Injection**: Prevented by using structured parameters and hardcoded `--` separators in `host_cmd`.
   - **Config Tampering**: Prevented by requiring `ApprovedConfig` (`src/config/approval.rs`) before starting the server.
   - **Sandbox Escape**: Limited by whitelisting specific commands and using regex patterns for arguments.

## Sourced Findings

### Subcommand Registration
- **File**: `/home/pl/code/palekiwi-labs/cast/src/commands/cli.rs`
- **Symbol**: `enum Commands`
- **Snippet**:
```rust
#[derive(Subcommand)]
pub enum Commands {
    // ...
    /// Manage MCP server
    Mcp {
        #[command(subcommand)]
        command: mcp::McpCommands,
    },
}
```

### Config Loading
- **File**: `/home/pl/code/palekiwi-labs/cast/src/config/loader.rs`
- **Pattern**: Uses `figment` for merging sources. Adding fields to `Config` struct in `src/config/schema.rs` is sufficient.

### Sandbox Networking
- **File**: `/home/pl/code/palekiwi-labs/cast/src/dev/run.rs`
- **Snippet**:
```rust
208:     if config.add_host_docker_internal {
209:         run_args.push("--add-host".to_string());
210:         run_args.push("host.docker.internal:host-gateway".to_string());
211:     }
```

### Dynamic rmcp Handler
- **Reference**: `/home/pl/code/palekiwi-labs/dev-notes/cast/rmcp/DYNAMIC_TOOLS.md`
- **Trait Signature**:
```rust
async fn call_tool(
    &self,
    request: CallToolRequestParams,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError>
```

## Implementation Strategy

1. **Feature Flagging**: Add `mcp` feature to `Cargo.toml`. Gate `tokio`, `axum`, and `rmcp` dependencies.
2. **Schema Definition**: Expand `Config` to support `McpConfig` with dynamic JSON schema definitions.
3. **Dispatcher**: Implement a manual `ServerHandler` that maps MCP tool calls to host process execution using `std::process::Command`.
4. **Validation**: Use `jsonschema` crate to validate agent-provided arguments against the user-defined schemas.

## Confidence Notes
- **High**: Subcommand integration and config loading patterns are well-established in the codebase.
- **High**: The `rmcp` manual handler approach is well-documented in the reference material.
- **Medium**: Mapping `{flags}` and `{args}` placeholders in the `host_cmd` array will require careful implementation to handle vector splicing correctly.
