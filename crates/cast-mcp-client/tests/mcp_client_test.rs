use assert_cmd::Command;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

// Import our client from the cast-mcp-client crate
use cast_mcp_client::McpClient;

/// Build a mock tool with a fully populated input schema for testing.
fn make_mock_tool() -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "message": {
                "type": "string",
                "description": "The message to send"
            },
            "count": {
                "type": "integer",
                "description": "Number of times to repeat"
            }
        },
        "required": ["message"]
    });

    Tool::new_with_raw(
        "dummy_tool".to_string(),
        Some("A mock tool for integration testing".into()),
        schema.as_object().cloned().unwrap_or_default(),
    )
}

/// A minimal dummy server handler to mock responses for our client.
struct MockServerHandler;

impl ServerHandler for MockServerHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("mock-server", "1.0.0"))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            tools: vec![make_mock_tool()],
            next_cursor: None,
            meta: Default::default(),
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        match request.name.as_ref() {
            "dummy_tool" => {
                let message = request
                    .arguments
                    .as_ref()
                    .and_then(|args| args.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no message)");
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "echo: {}",
                    message
                ))]))
            }
            "error_tool" => Ok(CallToolResult::error(vec![Content::text(
                "something went wrong",
            )])),
            other => Err(McpError::invalid_params(
                format!("Unknown tool: {}", other),
                None,
            )),
        }
    }
}

#[tokio::test]
async fn test_mcp_client_handshake_and_discovery() -> anyhow::Result<()> {
    // 1. Setup the cancellation token and the mock server
    let ct = CancellationToken::new();
    let service = StreamableHttpService::new(
        || Ok(MockServerHandler),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );

    // Nest the server service on the `/mcp` route
    let router = axum::Router::new().nest_service("/mcp", service);

    // Bind listener on a random free port (127.0.0.1:0)
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // Spawn the mock server in the background
    let server_handle = tokio::spawn({
        let ct = ct.clone();
        async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move { ct.cancelled_owned().await })
                .await;
        }
    });

    // 2. Connect the McpClient to the mock server (performing handshake under the hood)
    let server_url = format!("http://{addr}/mcp");
    let client = McpClient::connect(&server_url).await?;

    // 3. Verify discovery works (list_tools fetches tools from mock server)
    let tools = client.list_tools().await?;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "dummy_tool");
    assert_eq!(
        tools[0].description.as_deref(),
        Some("A mock tool for integration testing")
    );

    // 4. Gracefully shutdown the server
    ct.cancel();
    let _ = server_handle.await;

    Ok(())
}

#[tokio::test]
async fn test_mcp_list_subcommand_output() -> anyhow::Result<()> {
    // 1. Spawn a mock MCP server
    let ct = CancellationToken::new();
    let service = StreamableHttpService::new(
        || Ok(MockServerHandler),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );
    let router = axum::Router::new().nest_service("/mcp", service);
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

    // 2. Invoke `cast mcp list --url <mock_url>` as a subprocess.
    // spawn_blocking prevents executor starvation: without it, the blocking
    // assert() would starve the Tokio reactor, preventing the mock server from
    // processing the client's delete_session cleanup request, causing a deadlock.
    let url = format!("http://{addr}/mcp");
    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["list", "--url", &url]);

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        assert!(json.is_array());
        assert_eq!(json[0]["name"], "dummy_tool");
        assert_eq!(
            json[0]["description"],
            "A mock tool for integration testing"
        );
    })
    .await?;

    ct.cancel();
    Ok(())
}

#[tokio::test]
async fn test_mcp_describe_subcommand_output() -> anyhow::Result<()> {
    // 1. Spawn a mock MCP server
    let ct = CancellationToken::new();
    let service = StreamableHttpService::new(
        || Ok(MockServerHandler),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );
    let router = axum::Router::new().nest_service("/mcp", service);
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

    // 2. Invoke `cast mcp describe dummy_tool --url <mock_url>` as a subprocess.
    // spawn_blocking prevents executor starvation (same pattern as list test).
    let url = format!("http://{addr}/mcp");
    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["describe", "dummy_tool", "--url", &url]);

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        assert!(json.is_object());
        assert_eq!(json["name"], "dummy_tool");
        assert_eq!(json["description"], "A mock tool for integration testing");
        assert!(json["inputSchema"]["properties"]["message"].is_object());
    })
    .await?;

    ct.cancel();
    Ok(())
}

#[tokio::test]
async fn test_mcp_describe_unknown_tool_fails() -> anyhow::Result<()> {
    // 1. Spawn a mock MCP server
    let ct = CancellationToken::new();
    let service = StreamableHttpService::new(
        || Ok(MockServerHandler),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );
    let router = axum::Router::new().nest_service("/mcp", service);
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

    // 2. Ask for a tool that does not exist — expect a non-zero exit with a helpful message.
    let url = format!("http://{addr}/mcp");
    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["describe", "nonexistent_tool", "--url", &url]);

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().failure().get_output().stderr.clone();
        let s = std::str::from_utf8(&output).expect("stderr should be UTF-8");
        let mut stream = serde_json::Deserializer::from_str(s).into_iter::<serde_json::Value>();
        let json = stream
            .next()
            .expect("stderr should contain JSON")
            .expect("valid JSON expected");
        assert_eq!(json["error"]["code"], "TOOL_NOT_FOUND");
        assert!(
            json["error"]["message"]
                .as_str()
                .expect("error message should be a string")
                .contains("nonexistent_tool")
        );
    })
    .await?;

    ct.cancel();
    Ok(())
}

/// Spawn a fresh mock server and return its URL and cancellation token.
async fn spawn_mock_server() -> anyhow::Result<(String, CancellationToken)> {
    let ct = CancellationToken::new();
    let service = StreamableHttpService::new(
        || Ok(MockServerHandler),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );
    let router = axum::Router::new().nest_service("/mcp", service);
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
    Ok((format!("http://{addr}/mcp"), ct))
}

#[tokio::test]
async fn test_mcp_call_inline_json() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args([
        "call",
        "dummy_tool",
        r#"{"message": "hello"}"#,
        "--url",
        &url,
    ]);

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        assert_eq!(json["content"][0]["type"], "text");
        assert!(
            json["content"][0]["text"]
                .as_str()
                .expect("content should be text")
                .contains("echo: hello")
        );
        assert!(json["isError"].is_null() || json["isError"] == false);
    })
    .await?;

    ct.cancel();
    Ok(())
}

#[tokio::test]
async fn test_mcp_call_stdin_json() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["call", "dummy_tool", "-", "--url", &url])
        .write_stdin(r#"{"message": "from stdin"}"#);

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        assert_eq!(json["content"][0]["type"], "text");
        assert!(
            json["content"][0]["text"]
                .as_str()
                .expect("content should be text")
                .contains("echo: from stdin")
        );
        assert!(json["isError"].is_null() || json["isError"] == false);
    })
    .await?;

    ct.cancel();
    Ok(())
}

#[tokio::test]
async fn test_mcp_call_unknown_tool_fails() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["call", "nonexistent_tool", "{}", "--url", &url]);

    tokio::task::spawn_blocking(move || {
        cmd.assert().failure();
    })
    .await?;

    ct.cancel();
    Ok(())
}

#[tokio::test]
async fn test_mcp_call_tool_error_in_json() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["call", "error_tool", "{}", "--url", &url]);

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        assert_eq!(json["isError"], true);
        assert!(
            json["content"][0]["text"]
                .as_str()
                .expect("content should be text")
                .contains("something went wrong")
        );
    })
    .await?;

    ct.cancel();
    Ok(())
}

#[tokio::test]
async fn test_mcp_list_stdout_is_clean_json() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["list", "--url", &url]);
    cmd.env("RUST_LOG", "debug");

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let _json: serde_json::Value = serde_json::from_slice(&output)
            .expect("stdout should be valid JSON even with RUST_LOG=debug");
    })
    .await?;

    ct.cancel();
    Ok(())
}
