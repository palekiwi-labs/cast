# rmcp v1.6.0 — HTTP Client Reference

Reference material for implementing `cast-mcp-client` HTTP transport with multi-server support,
custom headers, and the full tool list/call lifecycle.

---

## Cargo.toml Features

Current features in `crates/cast-mcp-client/Cargo.toml`:

```toml
rmcp = { version = "1.6.0", features = [
    "client",
    "transport-streamable-http-client",
    "transport-streamable-http-client-reqwest",
] }
```

No new dependencies are required for the planned implementation.
The `http` crate is a transitive dependency via rmcp and will be available as `http::HeaderName` / `http::HeaderValue`.

---

## Key use Statements

```rust
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::service::RunningService;
use rmcp::{Peer, RoleClient, ClientHandler};
use rmcp::model::{Tool, CallToolRequestParams, CallToolResult, ListToolsResult, ClientInfo, ClientCapabilities, Implementation};
use http::{HeaderName, HeaderValue};
use std::collections::HashMap;
```

---

## Transport: StreamableHttpClientTransportConfig

Struct definition (from `src/transport/streamable_http_client.rs`):

```rust
pub struct StreamableHttpClientTransportConfig {
    pub uri: Arc<str>,
    pub retry_config: Arc<dyn SseRetryPolicy>,
    pub channel_buffer_capacity: usize,
    pub allow_stateless: bool,
    pub auth_header: Option<String>,                     // Bearer token (without "Bearer " prefix)
    pub custom_headers: HashMap<HeaderName, HeaderValue>, // uses http crate types
    pub reinit_on_expired_session: bool,
}
```

Builder methods:

```rust
impl StreamableHttpClientTransportConfig {
    pub fn with_uri(uri: impl Into<Arc<str>>) -> Self;

    /// Sets Authorization: Bearer <value>
    pub fn auth_header<T: Into<String>>(mut self, value: T) -> Self;

    /// Accepts HashMap<http::HeaderName, http::HeaderValue> — NOT HashMap<String, String>
    pub fn custom_headers(mut self, custom_headers: HashMap<HeaderName, HeaderValue>) -> Self;

    pub fn reinit_on_expired_session(mut self, enable: bool) -> Self;
}
```

### Reserved headers (rejected by rmcp, do not include in custom_headers)
- `accept`
- `mcp-session-id`
- `last-event-id`

### Building the transport

```rust
let transport = StreamableHttpClientTransport::from_config(config);
```

---

## Passing Arbitrary Headers (String → http types)

There is no built-in `HashMap<String, String>` shortcut. Convert manually:

```rust
use http::{HeaderName, HeaderValue};
use std::str::FromStr;

let mut headers: HashMap<HeaderName, HeaderValue> = HashMap::new();
for (k, v) in string_headers {
    let name = HeaderName::from_str(&k)?;      // returns Err if invalid
    let value = HeaderValue::from_str(&v)?;    // returns Err if invalid (non-ASCII)
    headers.insert(name, value);
}
```

For the Authorization header specifically, prefer `.auth_header(token)` over inserting into
`custom_headers` — it handles the `Bearer ` prefix automatically.

---

## McpClientHandler — Minimal Implementation

```rust
#[derive(Clone, Debug, Default)]
pub struct McpClientHandler;

impl ClientHandler for McpClientHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("cast-cli-client", env!("CARGO_PKG_VERSION")),
        )
    }
    // All other methods (on_progress, on_cancelled, etc.) have default no-op impls
}
```

---

## Connecting: serve() and RunningService

```rust
let handler = McpClientHandler;
// Returns Result<RunningService<RoleClient, McpClientHandler>, ClientInitializeError>
let service = handler.serve(transport).await?;
let peer: Peer<RoleClient> = service.peer().clone();
```

`RunningService` must be kept alive for the duration of use; it owns the background task.
Call `service.cancel().await` for graceful shutdown (sends session-delete to server).

---

## Peer<RoleClient> — Tool Operations

```rust
impl Peer<RoleClient> {
    // Paginated list (pass None for first page)
    pub async fn list_tools(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListToolsResult, ServiceError>;

    // Convenience: fetches all pages, returns full Vec<Tool>
    pub async fn list_all_tools(&self) -> Result<Vec<Tool>, ServiceError>;

    // Call a tool
    pub async fn call_tool(
        &self,
        params: CallToolRequestParams,
    ) -> Result<CallToolResult, ServiceError>;
}
```

`list_all_tools` implementation paginates via `next_cursor` automatically — use this for `list` command.

---

## Model Types

### Tool

```rust
pub struct Tool {
    pub name: Cow<'static, str>,
    pub title: Option<String>,
    pub description: Option<Cow<'static, str>>,
    pub input_schema: Arc<JsonObject>,
    pub output_schema: Option<Arc<JsonObject>>,
    // ... additional metadata/execution hints
}
```

### CallToolRequestParams

```rust
pub struct CallToolRequestParams {
    pub name: Cow<'static, str>,
    pub arguments: Option<JsonObject>,  // JsonObject = serde_json::Map<String, Value>
    pub meta: Option<Meta>,
    // ...
}

impl CallToolRequestParams {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self;
    pub fn with_arguments(mut self, arguments: JsonObject) -> Self;
    pub fn with_task(mut self, task: JsonObject) -> Self;
}
```

### CallToolResult

```rust
pub struct CallToolResult {
    pub content: Vec<Content>,
    pub is_error: Option<bool>,
    pub structured_content: Option<Value>,
    // ...
}
```

### ListToolsResult

```rust
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
    pub next_cursor: Option<String>,
}
```

---

## Error Types

| Type | When used |
|---|---|
| `ServiceError` | Errors during active peer operations (transport closed, timeout, MCP protocol errors) |
| `ClientInitializeError` | Handshake failures during `serve()` |
| `ErrorData` | Standard MCP JSON-RPC error payload (code, message, data) |
| `RmcpError` | Top-level unified error (wraps the above) |

All implement `std::error::Error` and `Display`. Map to `anyhow::Error` via `?`.

---

## Full Connection Pattern (with custom headers)

```rust
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{ClientHandler, Peer, RoleClient};
use http::{HeaderName, HeaderValue};
use std::collections::HashMap;
use std::str::FromStr;

pub async fn connect(
    url: &str,
    headers: &HashMap<String, String>,
) -> anyhow::Result<McpClient> {
    let mut http_headers: HashMap<HeaderName, HeaderValue> = HashMap::new();
    for (k, v) in headers {
        let name = HeaderName::from_str(k)
            .map_err(|e| anyhow::anyhow!("invalid header name '{}': {}", k, e))?;
        let value = HeaderValue::from_str(v)
            .map_err(|e| anyhow::anyhow!("invalid header value for '{}': {}", k, e))?;
        http_headers.insert(name, value);
    }

    let config = StreamableHttpClientTransportConfig::with_uri(url)
        .custom_headers(http_headers)
        .reinit_on_expired_session(true);

    let transport = StreamableHttpClientTransport::from_config(config);
    let service = McpClientHandler.serve(transport).await?;
    let peer = service.peer().clone();

    Ok(McpClient { peer, service })
}
```

---

## Concurrency Pattern for Multi-Server List

Use `tokio::join_all` / `futures::future::join_all` — each server connection is independent:

```rust
use futures::future::join_all;

let futures = servers.iter().map(|(name, cfg)| async move {
    let result = McpClient::connect(&cfg.url, &cfg.headers).await
        .and_then(|client| client.list_all_tools().await);
    (name, result)
});

let results: Vec<_> = join_all(futures).await;
```

`futures` is already available as a transitive dependency via rmcp. Alternatively use
`tokio::spawn` + `JoinSet` for structured concurrency.

---

## Notes

- `auth_header()` builder is for **Bearer tokens only** (adds `Authorization: Bearer <token>`).
  For other auth schemes (API keys, custom headers), use `custom_headers` with explicit header names.
- `{env:VAR}` substitution in config values must be resolved **before** passing to this API.
- `service.cancel().await` sends the MCP session-delete request cleanly; always call on shutdown.
