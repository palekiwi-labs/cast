# Research: Serving `cast` Documentation via MCP

This report documents the research for implementing a documentation serving feature in the `cast` MCP server.

## Research Questions Answered

1. **How is the MCP server currently implemented?**
   - The server is implemented in `src/commands/mcp/`.
   - It uses a manual `ServerHandler` implementation in `src/commands/mcp/handler.rs` to support dynamic tools.
   - Entry point: `run_http_server` in `src/commands/mcp/server.rs`.

2. **Should we use Tools or Resources for documentation?**
   - **Resources** are the idiomatic MCP way to serve read-only data like documentation.
   - The `rmcp` crate supports resources via `list_resources` and `read_resource` methods in the `ServerHandler` trait.
   - However, the user specifically requested **tools** for listing and fetching. This might be because tools are more "active" and visible to some AI agents than resources.
   - Recommendation: Implement as tools to follow the user's design preference while acknowledging that resources are an alternative.

3. **How to register the new tools alongside dynamic ones?**
   - In `src/commands/mcp/handler.rs`, we can extend the `McpHandler` to include "built-in" tools.
   - `list_tools`: Append built-in tool definitions to the list of dynamic tools loaded from config.
   - `call_tool`: Match on the tool name. If it's `list_documentation_entries` or `fetch_documentation_entry`, handle it locally; otherwise, delegate to the dynamic tool execution logic.

4. **Where should documentation files be stored and how to access them?**
   - Pattern: Codebase uses `include_str!` for embedded assets (e.g., `src/nix_daemon/image.rs`).
   - For this feature, we can create a `docs/` directory in the repository root.
   - To serve them via MCP, we can use a registry (a map of IDs/Titles to content).

## Sourced Findings

### Current `ServerHandler` Implementation
- **File**: `/home/pl/code/palekiwi-labs/cast/src/commands/mcp/handler.rs`
- **Symbol**: `impl ServerHandler for McpHandler`
- **Snippet**:
```rust
#[async_trait]
impl ServerHandler for McpHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "cast-mcp".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: ServerCapabilities {
                tools: Some(true),
                ..Default::default()
            },
        }
    }
    // ... list_tools and call_tool implementation
}
```

### MCP Config Schema
- **File**: `/home/pl/code/palekiwi-labs/cast/src/config/schema.rs`
- **Symbol**: `pub struct McpConfig`
- **Snippet**:
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpConfig {
    #[serde(default = "default_mcp_port")]
    pub port: u16,
    #[serde(default = "default_mcp_hostname")]
    pub hostname: String,
    #[serde(default)]
    pub tools: BTreeMap<String, McpToolConfig>,
}
```

### rmcp Resource Model (for reference)
- **File**: `/home/pl/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rmcp-1.6.0/src/model/resource.rs`
- **Symbol**: `pub struct RawResource`
- **Snippet**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RawResource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}
```

## Implementation Strategy (Technical)

1. **Extend `McpHandler`**: Add a `built_in_tools` field or hardcode them in `list_tools`.
2. **Implement `list_documentation_entries`**:
   - Return a list of available doc titles/IDs.
3. **Implement `fetch_documentation_entry`**:
   - Takes `entry_id` as a parameter.
   - Returns the content of the corresponding documentation file.
4. **Docs Storage**:
   - Create `docs/mcp/configuration.md` as the first entry.
   - Use `include_str!` or runtime file reading to load contents.

## Confidence Notes
- **High**: Understanding of `rmcp` handler structure.
- **High**: Path to adding built-in tools to a dynamic handler.
- **Medium**: Whether to use a hardcoded map for docs or a directory-walking mechanism. Given the "only one entry for now" constraint, a simple map is preferred.
