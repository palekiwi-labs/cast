# Comprehensive Guide: Building and Testing Streamable HTTP (SSE) MCP Clients

This guide focuses exclusively on implementing and testing MCP clients that communicate with a remote server using **HTTP Streaming (Server-Sent Events)**.

---

## 1. Architecture Overview
The Streamable HTTP client in the Rust SDK follows a bi-directional pattern:
1.  **Requests (Client -> Server)**: Sent via standard HTTP POST requests.
2.  **Responses & Notifications (Server -> Client)**: Delivered via a persistent Server-Sent Events (SSE) stream.

The SDK manages this complexity (multiplexing, session IDs, and reconnection) through the `StreamableHttpClientTransport`.

---

## 2. Implementing the Client

### Step 1: Define the `ClientHandler`
Even for HTTP clients, you must implement the `ClientHandler` trait to define how your client reacts to server-initiated messages, such as sampling requests or progress notifications.

```rust
use rmcp::handler::client::ClientHandler;
use rmcp::model::{CreateMessageRequestParams, CreateMessageResult, ErrorData};

struct MyHttpClientHandler;

impl ClientHandler for MyHttpClientHandler {
    // Required if the server uses the 'sampling' capability
    async fn create_message(
        &self,
        params: CreateMessageRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, ErrorData> {
        Ok(CreateMessageResult::new(
            SamplingMessage::assistant_text("Response from HTTP client".into()),
            "my-llm-model".into(),
        ))
    }

    // Handle notifications from the server
    async fn on_notification(&self, method: String, params: Option<serde_json::Value>) {
        println!("Received notification: {} with {:?}", method, params);
    }
}
```

### Step 2: Initialize the Transport
The `StreamableHttpClientTransport` requires a URI and can be configured with authentication headers.

```rust
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::common::reqwest::StreamableHttpClientTransportConfig;

// Configure the transport
let config = StreamableHttpClientTransportConfig {
    uri: "https://mcp.example.com/api/mcp".into(),
    auth_header: Some("Bearer your-api-token".to_string()),
    // Optional: Auto-reconnect and session recovery settings
    reinit_on_expired_session: true, 
    ..Default::default()
};

let transport = StreamableHttpClientTransport::from_config(config);
```

### Step 3: Start the Service
Connect your handler to the transport to get a `Peer` handle.

```rust
let handler = MyHttpClientHandler;
let running_service = handler.serve(transport).await?;

// The 'peer' is used to send requests to the server
let client = running_service.peer();

// Example: Calling a tool over HTTP
let result = client.call_tool(CallToolRequestParams::new("my_remote_tool")).await?;
```

---

## 3. Advanced Features

### Transparent Re-initialization
HTTP sessions can expire (e.g., if the server is restarted). The SDK handles this by:
1.  Detecting a `404 Not Found` on a POST request (indicating a stale session).
2.  Automatically performing a new `initialize` handshake.
3.  Retrying the original request with the new `Mcp-Session-Id`.

### SSE Multi-plexing
Since multiple requests might be pending, the SDK automatically correlates the asynchronous events coming over the SSE stream back to the correct JSON-RPC request IDs.

---

## 4. Testing the HTTP Client
Testing HTTP clients requires simulating the HTTP/SSE lifecycle. The recommended approach is an integration test using a local `axum` server.

### Integration Test Pattern
**Source**: `crates/rmcp/tests/test_streamable_http_stale_session.rs`

```rust
#[tokio::test]
async fn test_http_client_flow() -> anyhow::Result<()> {
    // 1. Setup a Mock Server in your test using Axum
    let service = StreamableHttpService::new(
        || Ok(MyMockServerHandler::new()), // Your server logic
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );
    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    // Run the mock server in the background
    tokio::spawn(async move { axum::serve(listener, router).await });

    // 2. Connect the Client to the local address
    let transport = StreamableHttpClientTransport::from_uri(format!("http://{addr}/mcp"));
    let client = MyHttpClientHandler.serve(transport).await?;

    // 3. Perform assertions
    let tools = client.peer().list_tools(None).await?;
    assert!(!tools.tools.is_empty());

    Ok(())
}
```

---

## 5. Summary Table: HTTP Streaming Specifics

| Category | Detail |
| :--- | :--- |
| **Protocol** | JSON-RPC 2.0 over HTTP POST (Requests) and SSE (Events). |
| **Session Tracking** | Handled via the `Mcp-Session-Id` header. |
| **Resilience** | Built-in auto-reconnect and `reinit_on_expired_session`. |
| **Transport Type** | `StreamableHttpClientTransport` (uses `reqwest` by default). |
| **Testing** | Requires an HTTP mock (e.g., `axum` + `StreamableHttpService`). |
