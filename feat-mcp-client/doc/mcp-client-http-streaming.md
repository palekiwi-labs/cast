# MCP Client Research Report: CLI and Scripting with HTTP Streaming

## Research Questions
1. Is the MCP client (Rust SDK) the only way to interact with an HTTP streaming MCP server from the command line?
2. Can interaction be scripted without the SDK (e.g., using `curl`)?
3. How does the HTTP streaming protocol work under the hood?

## Findings

### 1. Scripting without the SDK
The MCP HTTP streaming protocol is based on standard JSON-RPC 2.0 delivered over HTTP POST and Server-Sent Events (SSE). It is highly scriptable using common CLI tools like `curl`.

#### Manual Interaction Workflow (Conceptual)
1. **Initialize Session**: Send a POST request with the `initialize` method.
   - **Endpoint**: The server's MCP URL (e.g., `http://localhost:8080/mcp`).
   - **Header**: `Accept: text/event-stream, application/json`.
   - **Body**: Standard JSON-RPC `initialize` request.
   - **Response**: The server provides an `Mcp-Session-Id` header and a body (often containing the server's capabilities).

2. **Establish Listen Stream**: Open a persistent GET request to receive responses and notifications.
   - **Header**: `Mcp-Session-Id: <ID_FROM_STEP_1>`, `Accept: text/event-stream`.
   - **Note**: This stream must remain open to receive the results of subsequent POST requests in many server implementations.

3. **Call Tools**: Send POST requests with the `tools/call` method.
   - **Header**: `Mcp-Session-Id: <ID_FROM_STEP_1>`.
   - **Body**: JSON-RPC `tools/call` request.

### 2. Protocol Details (Sourced from `rust-sdk`)

#### Handshake and Session Management
The protocol uses a stateful session identified by the `Mcp-Session-Id` header.
- **Source**: `crates/rmcp/src/transport/streamable_http_client.rs`
- **Logic**: The client manages a session ID and re-initializes transparently if it receives a `404 Not Found` (session expired).

#### Request/Response Pattern
In the default SSE mode, the client sends a request via POST and receives the JSON-RPC response asynchronously over the SSE GET stream.
- **Source**: `crates/rmcp/src/transport/streamable_http_client.rs`
- **Snippet (Headers)**:
  ```rust
  const SESSION_ID_HEADER: &str = "Mcp-Session-Id";
  const PROTOCOL_VERSION_HEADER: &str = "MCP-Protocol-Version";
  ```

### 3. Using the MCP Client (Rust SDK)
The Rust SDK provides a robust `StreamableHttpClientTransport` that handles:
- **Session Lifecycle**: Automatic `initialize` and `initialized` notification flow.
- **Transparent Re-initialization**: If a session expires (404), the SDK automatically re-authenticates and retries the pending request.
- **Response Matching**: Correlating asynchronous SSE events back to the original POST requests using JSON-RPC IDs.

#### Implementation Entry Point
To use the SDK for a CLI tool, you would typically use the `serve` method on a `ClientInfo`.
- **Source**: `crates/rmcp/src/service.rs`
- **Snippet**:
  ```rust
  fn serve<T, E, A>(self, transport: T) -> impl Future<Output = Result<RunningService<R, Self>, R::InitializeError>>
  ```

## Summary Recommendation
- **For quick scripting**: `curl` is sufficient if you only need to trigger a single tool and can parse the SSE stream manually.
- **For robust CLI tools**: Using the `rust-sdk` (MCP client) is highly recommended because it handles session expiry, transparent re-initialization, and complex message multiplexing (SSE) which are difficult to implement reliably in a simple script.
- **Examples**: The SDK includes a `streamable_http` example at `examples/clients/src/streamable_http.rs` which can be adapted into a CLI utility.
