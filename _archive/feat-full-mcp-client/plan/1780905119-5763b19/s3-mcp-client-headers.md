---
status: complete
---

# S3 Executive Plan: McpClient with Custom Headers

## Foreword

This plan implements Slice 3 of the cast-mcp-client generalization (see `plan/index.md`).

**Goal:** Update `McpClient::connect` to accept a `&RemoteServerConfig` instead of a bare
`url: &str`, forwarding its custom headers to the rmcp transport layer. A new integration
test verifies that headers are actually received by a real HTTP server.

**Prerequisites:**
- S1 (config module) and S2 (server map logic) are committed and all tests green.
- `config::RemoteServerConfig` is the canonical struct for server configuration
  (`url: String`, `headers: HashMap<String, String>`, `enabled: bool`).
- The rmcp reference doc (`doc/1780761606-73dff8d/rmcp-client-reference.md`) confirms the
  exact API: `StreamableHttpClientTransportConfig::custom_headers()` takes
  `HashMap<http::HeaderName, http::HeaderValue>`, manual conversion required.
- `http` crate is a transitive dep of rmcp — no new Cargo entries needed.
- `axum` is already a dev-dependency (used by existing integration tests).

**Files touched:**
- `crates/cast-mcp-client/src/lib.rs` — signature change + header conversion
- `crates/cast-mcp-client/tests/mcp_client_test.rs` — new integration test

**Files NOT touched in this slice:**
- `src/main.rs` — still uses old `McpClient::connect(&url)` until S4
- `src/config.rs` — no changes needed

---

## Context: What Exists Today

`McpClient::connect(url: &str)` builds a plain `StreamableHttpClientTransportConfig::with_uri(url)`
with no headers. The existing tests all call it via `McpClient::connect(&server_url)`.

After S3, the signature becomes `McpClient::connect(server: &RemoteServerConfig)`. Existing
callers in the test file use `McpClient::connect(&server_url)` — these must be updated to pass
a `RemoteServerConfig` constructed from the URL.

However, the **CLI integration tests** (those using `Command::cargo_bin`) pass `--url` through
`main.rs` → `list_tools_cmd(url)` → old connect path. Those tests keep working unchanged because
`list_tools_cmd` / `describe_tool_cmd` / `call_tool_cmd` still build a `RemoteServerConfig`
internally from the URL they receive. We update those helper functions to construct a
`RemoteServerConfig { url, headers: HashMap::new(), enabled: true }` and call `connect(server)`.

---

## Steps

### RED phase — write the failing test first

- [x] **Step 1: Add `test_headers_are_sent_to_server`** in
  `tests/mcp_client_test.rs`.

  The test must:
  1. Spin up an axum server with an `axum::middleware::from_fn` layer that
     captures the value of a custom header (e.g. `X-Test-Token`) into a
     `Arc<Mutex<Option<String>>>` before delegating to the MCP service.
  2. Build a `RemoteServerConfig { url, headers: { "X-Test-Token" => "test-secret" }, enabled: true }`.
  3. Call `McpClient::connect(&server_cfg).await?` (new signature — compile error expected).
  4. Call `client.list_tools().await?` to trigger a real HTTP round-trip.
  5. Assert the captured header value equals `"test-secret"`.
  6. Graceful shutdown.

  Required imports to add at the top of the test file:
  ```rust
  use axum::middleware::Next;
  use axum::extract::Request;
  use cast_mcp_client::config::RemoteServerConfig;
  use std::sync::{Arc, Mutex};
  use std::collections::HashMap;
  ```

  **Expected result:** Does not compile (connect still takes `&str`). This is the RED state.

---

### GREEN phase — implement the signature change

- [x] **Step 2: Update `McpClient::connect` signature in `src/lib.rs`.**

  Change from:
  ```rust
  pub async fn connect(url: &str) -> anyhow::Result<Self>
  ```
  To:
  ```rust
  pub async fn connect(server: &config::RemoteServerConfig) -> anyhow::Result<Self>
  ```

  Implementation body:
  ```rust
  pub async fn connect(server: &config::RemoteServerConfig) -> anyhow::Result<Self> {
      let mut http_headers: HashMap<HeaderName, HeaderValue> = HashMap::new();
      for (k, v) in &server.headers {
          let name = HeaderName::from_str(k)
              .map_err(|e| anyhow::anyhow!("invalid header name '{}': {}", k, e))?;
          let value = HeaderValue::from_str(v)
              .map_err(|e| anyhow::anyhow!("invalid header value for '{}': {}", k, e))?;
          http_headers.insert(name, value);
      }

      let config = StreamableHttpClientTransportConfig::with_uri(server.url.as_str())
          .custom_headers(http_headers)
          .reinit_on_expired_session(true);

      let transport = StreamableHttpClientTransport::from_config(config);
      let handler = McpClientHandler;
      let service = handler.serve(transport).await?;
      let peer = service.peer().clone();

      Ok(Self { peer, service })
  }
  ```

  Add required imports at top of `lib.rs`:
  ```rust
  use http::{HeaderName, HeaderValue};
  use std::collections::HashMap;
  use std::str::FromStr;
  ```

  Note: `HashMap` is already used in the file via `std::collections::HashMap` inline — change to a
  top-level `use` at this step to avoid duplication.

- [x] **Step 3: Update all callers of `McpClient::connect` in `src/lib.rs`.**

  The three command functions (`list_tools_cmd`, `describe_tool_cmd`, `call_tool_cmd`) call
  `McpClient::connect(&url)`. Each must be updated to construct a bare `RemoteServerConfig`:

  ```rust
  let server = config::RemoteServerConfig {
      url: url.clone(),
      headers: HashMap::new(),
      enabled: true,
  };
  McpClient::connect(&server).await?
  ```

  (These functions still take `url: Option<String>` — that is S4's domain. For now just
  adapt the call site.)

- [x] **Step 4: Update the existing integration test caller in `tests/mcp_client_test.rs`.**

  `test_mcp_client_handshake_and_discovery` calls `McpClient::connect(&server_url)` directly.
  Change it to:
  ```rust
  use cast_mcp_client::config::RemoteServerConfig;
  // ...
  let server_cfg = RemoteServerConfig {
      url: server_url,
      headers: std::collections::HashMap::new(),
      enabled: true,
  };
  let client = McpClient::connect(&server_cfg).await?;
  ```

- [x] **Step 5: Run all tests — confirm GREEN.**
  ```
  cargo test -p cast-mcp-client
  ```
  All existing tests must still pass. The new header test must now pass.

---

### REFACTOR phase

- [x] **Step 6: Review for duplication / clarity.**
  - Confirm `HashMap` import is not duplicated.
  - Confirm `std::str::FromStr` import is not already present under a different path.
  - No logic change, only tidy-up.

- [x] **Step 7: Run tests one final time to confirm no regression.**
  ```
  cargo test -p cast-mcp-client
  ```

---

## Test Design: `test_headers_are_sent_to_server`

```rust
#[tokio::test]
async fn test_headers_are_sent_to_server() -> anyhow::Result<()> {
    use axum::middleware::Next;
    use axum::extract::Request;
    use cast_mcp_client::config::RemoteServerConfig;

    // Shared storage for the captured header value
    let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();

    // MCP service (same MockServerHandler as other tests)
    let ct = CancellationToken::new();
    let service = StreamableHttpService::new(
        || Ok(MockServerHandler),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );

    // Middleware that captures the X-Test-Token header
    let router = axum::Router::new()
        .nest_service("/mcp", service)
        .layer(axum::middleware::from_fn(move |req: Request, next: Next| {
            let captured = captured_clone.clone();
            async move {
                if let Some(val) = req.headers().get("x-test-token") {
                    let mut lock = captured.lock().unwrap();
                    *lock = Some(val.to_str().unwrap_or("").to_string());
                }
                next.run(req).await
            }
        }));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    tokio::spawn({
        let ct = ct.clone();
        async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move { ct.cancelled_owned().await })
                .await;
        }
    });

    // Connect with a custom header
    let mut headers = std::collections::HashMap::new();
    headers.insert("X-Test-Token".to_string(), "test-secret".to_string());
    let server_cfg = RemoteServerConfig {
        url: format!("http://{addr}/mcp"),
        headers,
        enabled: true,
    };

    let client = McpClient::connect(&server_cfg).await?;
    let tools = client.list_tools().await?;
    assert_eq!(tools.len(), 1); // sanity check the connection worked
    client.shutdown().await?;

    // Verify the header was received
    let val = captured.lock().unwrap().clone();
    assert_eq!(val.as_deref(), Some("test-secret"));

    ct.cancel();
    Ok(())
}
```

**Key design note on middleware ordering:** `axum::middleware::from_fn` wraps the router, so
the middleware executes on every request including the MCP handshake. The header will be
captured on the very first request (`POST /mcp` initialize), which is sufficient for the
assertion.

---

## Risk Notes

- **`HashMap` import clash:** `lib.rs` currently uses `std::collections::HashMap` inline at
  several places. Adding `use std::collections::HashMap` at the top is safe (idiomatic Rust),
  but double-check there are no shadowing issues.
- **`http::HeaderName` / `http::HeaderValue`:** These come from the `http` crate, which is a
  transitive dep of rmcp. They should be resolvable without adding to `Cargo.toml`. If the
  compiler can't find them, add `http = "1"` to `[dependencies]` (it's already implicitly present).
- **Middleware capture timing:** The `list_tools` call triggers multiple HTTP requests
  (initialize + list_tools). The middleware captures on the first matching request. The
  `Arc<Mutex<Option<String>>>` pattern is race-safe within a single test because the client
  is sequential (no concurrent requests in the test body).
