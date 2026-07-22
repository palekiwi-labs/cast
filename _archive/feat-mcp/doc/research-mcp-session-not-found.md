# Research: MCP "Session not found" Error

This report investigates the cause of the `Streamable HTTP error: Error POSTing to endpoint: Not Found: Session not found` error reported by clients connecting to the `cast` MCP server.

## Research Questions Answered

1. **What is the root cause of the "Session not found" error?**
   - The error is a 404 response from the `cast` MCP server. It indicates that the `Mcp-Session-Id` header sent by the client (e.g., `opencode`) does not match any active session in the server's memory.
   - This happens because `cast` uses an in-memory session manager without persistence, and sessions are subject to timeouts.

2. **How are sessions managed on the server?**
   - `cast` uses the `rmcp` library for its MCP implementation.
   - The server instantiates a `LocalSessionManager` which maintains sessions in a `RwLock<HashMap<SessionId, LocalSessionHandle>>`.
   - By default, these sessions have a **5-minute inactivity timeout** (keep-alive) and are entirely volatile.

3. **Why does it happen when an agent from a different chat session wants to use the tool?**
   - Each "agent" or "chat session" typically performs its own MCP handshake (`initialize` request).
   - If an agent attempts to reuse a session ID that has expired or was created before a server restart, the 404 error occurs.
   - Reconnecting forces a new handshake, which generates a new valid `SessionId` on the server.

4. **Is state shared across sessions?**
   - The `McpHandler` in `cast` is stateless with respect to sessions. It shares global configuration and tool definitions but does not track per-client state.
   - Subprocesses spawned by tools are isolated but inherit the same environment from the host server.

## Sourced Findings

### Volatile Session Manager
- **File**: `src/commands/mcp/server.rs`
- **Symbol**: `run_http_server`
- **Snippet**:
```rust
37: let service = StreamableHttpService::new(
38:     move || Ok(handler.clone()),
39:     LocalSessionManager::default().into(),
40:     config,
41: );
```
*Note: `LocalSessionManager::default()` uses an in-memory store with no persistence.*

### Timeout Logic (via `rmcp` defaults)
- **Dependency**: `rmcp` crate
- **Logic**: `LocalSessionManager` implements a `keep_alive` timeout (default 300s / 5m). If no activity is detected within this window, the session worker terminates and the session is removed from the manager.

### 404 Response Logic (via `rmcp`)
- **Dependency**: `rmcp` crate (`src/transport/streamable_http_server/tower.rs`)
- **Logic**:
```rust
// If handle_post is called with an unknown session ID:
return Response::builder()
    .status(StatusCode::NOT_FOUND)
    .body(Body::from("Not Found: Session not found"))
    .unwrap();
```

### Handler Statelessness
- **File**: `src/commands/mcp/handler.rs`
- **Symbol**: `McpHandler`
- **Snippet**:
```rust
164: async fn list_tools(
165:     &self,
166:     _request: ListToolsRequestParams,
167:     _context: RequestContext<RoleServer>,
168: ) -> Result<ListToolsResult, McpError> {
```
*Note: The `_context` (which contains session info) is explicitly ignored, meaning the handler does not differentiate between sessions.*

## Confidence Notes
- **High**: The error message "Not Found: Session not found" is a verbatim string from the `rmcp` library's HTTP handler.
- **High**: The 5-minute timeout and volatile memory store perfectly explain why the error occurs after inactivity or restarts.
- **Medium**: The exact mechanism by which `opencode` persists its `session_id` between chat sessions was inferred from shared volume behavior, as the client source was not directly available.

## Conclusion
The "Session not found" error is a consequence of using a volatile, timeout-based session manager for a persistent MCP daemon. When an agent attempts to reuse an old or expired session ID, the server rejects it. A new handshake (reconnection) is required to establish a fresh session.
