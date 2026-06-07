# Research: MCP Server Timeout and Dangling Processes

This report documents the research into the lack of timeouts and the issue of dangling processes in the `cast` MCP server implementation.

## Research Questions Answered

1. **How is the MCP server implemented and how does it execute tools?**
   - The server uses the `rmcp` crate (v1.6.0) with an HTTP/SSE transport (`StreamableHttpService`) built on `axum`.
   - Tool execution is handled by `McpHandler` in `crates/cast/src/mcp/handler.rs`, which calls `run_command` in `crates/cast/src/mcp/exec.rs`.
   - `run_command` spawns host subprocesses using `tokio::process::Command`.

2. **Why are processes left dangling and how to fix it?**
   - **Current State**: `tokio::process::Command` is used without `kill_on_drop(true)`. When the future waiting for the process is dropped (e.g., if the server is stopped or a task is cancelled), the process handle is dropped but the OS process continues to run.
   - **Fix**: Add `cmd.kill_on_drop(true)` to the command builder in `crates/cast/src/mcp/exec.rs`.

3. **How does the server detect client disconnection?**
   - **Mechanism**: `rmcp` provides a `RequestContext<RoleServer>` to the `call_tool` method. This context contains a `CancellationToken` (`ct`).
   - **Discovery**: In `rmcp`, the `ct` token is cancelled when the underlying session or transport is closed.
   - **Integration**: The handler can use `tokio::select!` to listen for `context.ct.cancelled()`.

4. **How to implement a global timeout?**
   - **Configuration**: A `global_timeout` field (in seconds) should be added to `McpConfig` in `crates/cast/src/config/schema.rs`.
   - **Execution**: Wrap the `self.execute_tool(request)` call in `crates/cast/src/mcp/handler.rs` with `tokio::time::timeout`.

## Sourced Findings

### Current Indefinite Execution
- **File**: `crates/cast/src/mcp/exec.rs`
- **Snippet**:
```rust
40:     let child = match cmd.spawn() {
...
52:     let output = match child.wait_with_output().await {
```
The server waits indefinitely for `child.wait_with_output().await` to complete.

### RequestContext Cancellation Token
- **Source**: `rmcp` crate internals (resolved via dependency research)
- **File**: `rmcp/src/service.rs`
- **Symbol**: `RequestContext`
- **Snippet**:
```rust
pub struct RequestContext<R: ServiceRole> {
    pub ct: CancellationToken,
    pub id: RequestId,
    // ...
}
```

### Proposed Configuration Change
- **File**: `crates/cast/src/config/schema.rs`
- **Proposed Struct**:
```rust
pub struct McpConfig {
    pub port: u16,
    pub hostname: String,
    pub global_timeout: u64, // New field
    pub tools: BTreeMap<String, McpToolConfig>,
}
```

### Proposed Handler Integration
- **File**: `crates/cast/src/mcp/handler.rs`
- **Implementation Strategy**:
```rust
tokio::select! {
    res = tokio::time::timeout(timeout_duration, self.execute_tool(request)) => {
        // Handle result or timeout error
    }
    _ = context.ct.cancelled() => {
        // Handle client disconnection
    }
}
```

## Confidence Notes
- **High**: The use of `kill_on_drop(true)` is the standard `tokio` pattern for preventing dangling processes.
- **High**: `rmcp`'s `RequestContext` is explicitly designed to propagate cancellation.
- **Medium**: Ensuring the `global_timeout` is correctly propagated through all layers of the `McpHandler` and `Inner` struct will require careful auditing of the current initialization sequence.
