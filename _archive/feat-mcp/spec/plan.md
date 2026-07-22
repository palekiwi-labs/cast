# Implementation Plan: Built-in MCP Server (`cast mcp start`)

This plan translates the MCP server specification into a technical roadmap based on `rmcp` research.

## Architectural Approach

We will build a dynamic, JSON-Schema backed MCP server ("Semantic Tools") rather than a raw shell-command interceptor. This aligns directly with MCP standards, provides superior LLM UX, and heavily mitigates shell injection risks.

### Phase 1: Configuration & Dependencies
1. **Cargo Updates**
   - Add `mcp` to the `default = []` feature list in `Cargo.toml`.
   - Add `rmcp`, `tokio`, `axum`, and `jsonschema` as optional dependencies gated by the `mcp` feature.
2. **Config Schema (`src/config/schema.rs`)**
   - Introduce `McpConfig` -> `McpServerConfig` -> `McpToolConfig`.
   - `McpToolConfig` structure:
     - `description` (String)
     - `command` (String) - the base binary
     - `args` (Vec<ArgTemplate>) - using an untagged Enum with `serde` to support literal strings with placeholders AND conditional blocks (`if_present`, `if_true`).
     - `env` (Optional struct) - with `inherit` (`Vec<String>`) and `set` (`BTreeMap<String, String>`).
     - `parameters` (`serde_json::Value`) - the raw JSON schema.

### Phase 2: CLI Integration
1. **Subcommand Registration (`src/commands/cli.rs`)**
   - Add `Mcp { #[command(subcommand)] command: McpCommands }` variant.
   - Define `McpCommands::Start` with optional override flags like `--port`.
2. **Command Dispatch (`src/commands/mod.rs` & `src/commands/mcp/mod.rs`)**
   - Route to `mcp::run`.
   - **Security Gate**: Call `verify_config` (which checks `ApprovedConfig`) *before* initializing the server to ensure tools are trusted.

### Phase 3: Dynamic MCP Handler (`src/commands/mcp/handler.rs`)
1. **Manual `ServerHandler`**
   - Bypass `#[tool_router]` and manually implement the `ServerHandler` trait.
2. **`list_tools`**
   - Iterate over `config.mcp.tools` and use `Tool::new_with_raw` to construct the tool list dynamically using the user's schemas.
3. **`call_tool`**
   - Lookup the tool in config.
   - Compile the tool's JSON schema with the `jsonschema` crate.
   - Validate incoming `request.arguments` against the schema. If invalid, return an MCP error.

### Phase 4: Secure Execution Engine (`src/commands/mcp/exec.rs`)
1. **Array-Aware Placeholder Mapper**
   - Iterate through `ArgTemplate` array.
   - Evaluate `if_present` and `if_true` conditional objects based on JSON arguments.
   - Expand literal strings containing `"{var}"` or spread placeholders `"{...array}"` into standard `Vec<String>`.
2. **Environment Sandbox (Nix-Safe Pattern)**
   - To maintain Nix compatibility and thread safety, decouple command construction from execution.
   - **`resolve_env(config, host_env)`**: Pure function to resolve the environment map.
     - Enforces Default-Deny.
     - Always maps `PATH` from host.
     - Maps explicitly whitelisted variables from `env.inherit` and `env.set`.
   - **`build_exec_command(tool, mapped_args)`**: Pure function to return `(executable, args)` tuple.
3. **Execution**
   - Use `tokio::process::Command` for execution.
   - Wire the pure builders into a thin executor: `.env_clear().envs(resolved_env)`.
   - Use `tokio::task::spawn_blocking` only if needed for blocking I/O, otherwise prefer `tokio::process`.
   - Execute, capture stdout/stderr, and format the output into an MCP `CallToolResult`.

### Phase 5: Networking (`src/commands/mcp/server.rs`)
1. **Server Setup**
   - Initialize a multi-threaded `tokio` runtime (only for the `mcp` command).
   - Configure `rmcp`'s `StreamableHttpServerConfig`.
   - **Crucial**: Explicitly configure `allowed_hosts` to include `host.docker.internal` (plus localhost/127.0.0.1) so sandboxed agents can connect.
   - Bind `axum` TCP listener and run the server.