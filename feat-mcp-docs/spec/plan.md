# Implementation Plan: MCP Documentation Resources

This plan outlines the technical approach for serving `cast` documentation via the MCP Resources pattern.

## 1. Architectural Approach
- **Pattern**: MCP Resources (idiomatic for read-only data).
- **URI Scheme**: `cast://docs/<path>` (e.g., `cast://docs/mcp/configuration`).
- **Embedding**: Use `include_str!` to bake markdown files into the binary.
- **Registry**: A static registry of documentation entries will be maintained in the MCP handler.

## 2. Component Design

### Documentation Registry
A static array of `EmbeddedDoc` structs will be defined in `src/commands/mcp/handler.rs`.

```rust
struct EmbeddedDoc {
    uri: &'static str,
    name: &'static str,
    description: &'static str,
    content: &'static str,
}
```

### Handler Extension
The `McpHandler` (manual `ServerHandler` implementation) will be extended:

1. **Capabilities**: Enable `resources` in `get_info`.
2. **`list_resources`**: Map the `DOCS` registry to `RawResource` metadata.
3. **`read_resource`**: Route requests by URI to the embedded content.

## 3. Implementation Steps

### Phase 1: Content Preparation
- Finalize `docs/mcp/configuration.md`.

### Phase 2: Core Implementation
- Modify `src/commands/mcp/handler.rs` to add the `EmbeddedDoc` struct and `DOCS` registry.
- Update `McpHandler::get_info` to announce resource support.
- Implement `McpHandler::list_resources`.
- Implement `McpHandler::read_resource`.

### Phase 3: Validation
- Add unit tests for resource listing and reading.
- Manual test using an MCP client or internal CLI verification.

## 4. Verification
- `cargo test` to ensure no regressions in MCP tool handling.
- Verify `cast mcp start` correctly serves the new resources.
