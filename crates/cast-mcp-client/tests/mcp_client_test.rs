use assert_cmd::Command;
use cast_mcp_client::McpClient;
use cast_mcp_client::config::RemoteServerConfig;
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
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

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
    let server_cfg = RemoteServerConfig {
        url: format!("http://{addr}/mcp"),
        headers: HashMap::new(),
        enabled: true,
    };
    let client = McpClient::connect(&server_cfg).await?;

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

    // 2. Invoke `cast-mcp-client list --cast-mcp-url <mock_url>` as a subprocess.
    // spawn_blocking prevents executor starvation: without it, the blocking
    // assert() would starve the Tokio reactor, preventing the mock server from
    // processing the client's delete_session cleanup request, causing a deadlock.
    let url = format!("http://{addr}/mcp");
    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["list", "--cast-mcp-url", &url]);

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        // output is a nested object keyed by server name
        assert!(json.is_object());
        assert_eq!(json["cast"][0]["name"], "dummy_tool");
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

    // 2. Invoke `cast-mcp-client describe cast dummy_tool --cast-mcp-url <mock_url>` as a subprocess.
    // spawn_blocking prevents executor starvation (same pattern as list test).
    let url = format!("http://{addr}/mcp");
    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["describe", "cast", "dummy_tool", "--cast-mcp-url", &url]);

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
    cmd.args([
        "describe",
        "cast",
        "nonexistent_tool",
        "--cast-mcp-url",
        &url,
    ]);

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().failure().get_output().stderr.clone();
        let s = std::str::from_utf8(&output)
            .expect("stderr should be UTF-8")
            .trim();
        let json: serde_json::Value = serde_json::from_str(s)
            .expect("stderr should be exactly one valid JSON object, nothing else");
        assert_eq!(json["error"]["code"], "COMMAND_ERROR");
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
        "cast",
        "dummy_tool",
        r#"{"message": "hello"}"#,
        "--cast-mcp-url",
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
    cmd.args(["call", "cast", "dummy_tool", "-", "--cast-mcp-url", &url])
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
    cmd.args([
        "call",
        "cast",
        "nonexistent_tool",
        "{}",
        "--cast-mcp-url",
        &url,
    ]);

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
    cmd.args(["call", "cast", "error_tool", "{}", "--cast-mcp-url", &url]);

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
    cmd.args(["list", "--cast-mcp-url", &url]);
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

#[tokio::test]
async fn test_headers_are_sent_to_server() -> anyhow::Result<()> {
    use axum::extract::Request;
    use axum::middleware::Next;
    use std::sync::Mutex;

    // Shared storage for the captured header value
    let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let captured_clone = captured.clone();

    // MCP service
    let ct = CancellationToken::new();
    let service = StreamableHttpService::new(
        || Ok(MockServerHandler),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default().with_cancellation_token(ct.child_token()),
    );

    // Middleware that captures the X-Test-Token header value
    let router =
        axum::Router::new()
            .nest_service("/mcp", service)
            .layer(axum::middleware::from_fn(
                move |req: Request, next: Next| {
                    let captured = captured_clone.clone();
                    async move {
                        if let Some(val) = req.headers().get("x-test-token") {
                            let mut lock = captured.lock().unwrap();
                            if lock.is_none() {
                                *lock = Some(val.to_str().unwrap_or("").to_string());
                            }
                        }
                        next.run(req).await
                    }
                },
            ));

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
    let mut headers = HashMap::new();
    headers.insert("X-Test-Token".to_string(), "test-secret".to_string());
    let server_cfg = RemoteServerConfig {
        url: format!("http://{addr}/mcp"),
        headers,
        enabled: true,
    };

    let client = McpClient::connect(&server_cfg).await?;
    let tools = client.list_tools().await?;
    assert_eq!(tools.len(), 1); // sanity-check: connection worked
    client.shutdown().await?;

    // Verify the header was received by the server
    let val = captured.lock().unwrap().clone();
    assert_eq!(val.as_deref(), Some("test-secret"));

    ct.cancel();
    Ok(())
}

// ---------------------------------------------------------------------------
// S5 — multi-server nested list
// ---------------------------------------------------------------------------

/// No servers configured → stdout is exactly `{}`.
#[tokio::test]
async fn test_list_empty_config_returns_empty_object() -> anyhow::Result<()> {
    let tmpdir = tempfile::tempdir()?;
    // Write an empty config (no servers)
    std::fs::write(tmpdir.path().join("cast-mcp-client.json"), r#"{"mcp":{}}"#)?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.arg("list")
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let s = std::str::from_utf8(&output).unwrap().trim().to_string();
        assert_eq!(s, "{}");
    })
    .await?;

    Ok(())
}

/// Single server "cast" with dummy_tool → output is `{"cast": [{"name": "dummy_tool", ...}]}`.
#[tokio::test]
async fn test_list_nested_single_server() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"cast":{{"url":"{}"}}}}}}"#, url),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.arg("list")
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        assert!(json.is_object());
        assert_eq!(json["cast"].as_array().unwrap().len(), 1);
        assert_eq!(json["cast"][0]["name"], "dummy_tool");
    })
    .await?;

    ct.cancel();
    Ok(())
}

/// Two servers configured; `list sentry` (positional filter) returns only sentry tools.
#[tokio::test]
async fn test_list_filter_by_server() -> anyhow::Result<()> {
    let (cast_url, ct1) = spawn_mock_server().await?;
    let (sentry_url, ct2) = spawn_mock_server().await?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"cast":{{"url":"{cast_url}"}},"sentry":{{"url":"{sentry_url}"}}}}}}"#,),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["list", "sentry"])
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        assert!(json.is_object());
        let tools = json["sentry"]
            .as_array()
            .expect("sentry key should be an array");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "dummy_tool");
    })
    .await?;

    ct1.cancel();
    ct2.cancel();
    Ok(())
}

/// `list ghost` (unknown server positional arg) → non-zero exit + COMMAND_ERROR JSON on stderr.
#[tokio::test]
async fn test_list_unknown_server_fails() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"cast":{{"url":"{}"}}}}}}"#, url),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["list", "ghost"])
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let stderr = cmd.assert().failure().get_output().stderr.clone();
        let s = std::str::from_utf8(&stderr).unwrap().trim().to_string();
        let json: serde_json::Value =
            serde_json::from_str(&s).expect("stderr should be valid JSON");
        assert_eq!(json["error"]["code"], "COMMAND_ERROR");
        assert!(json["error"]["message"].as_str().unwrap().contains("ghost"));
    })
    .await?;

    ct.cancel();
    Ok(())
}

// ---------------------------------------------------------------------------
// S6 — describe/call: two positional args + routing
// ---------------------------------------------------------------------------

/// `describe cast dummy_tool` succeeds; stdout has bare name "dummy_tool".
#[tokio::test]
async fn test_describe_two_positional_args() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"cast":{{"url":"{}"}}}}}}"#, url),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["describe", "cast", "dummy_tool"])
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        assert!(json.is_object());
        // describe returns the raw tool object — name is bare (no prefix)
        assert_eq!(json["name"], "dummy_tool");
        assert_eq!(json["description"], "A mock tool for integration testing");
        assert!(json["inputSchema"]["properties"]["message"].is_object());
    })
    .await?;

    ct.cancel();
    Ok(())
}

/// `call cast dummy_tool '{"message":"hello"}'` succeeds and echoes correctly.
#[tokio::test]
async fn test_call_two_positional_args() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"cast":{{"url":"{}"}}}}}}"#, url),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["call", "cast", "dummy_tool", r#"{"message":"hello"}"#])
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

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

/// `describe ghost dummy_tool` (unknown server) → failure with COMMAND_ERROR mentioning "ghost".
#[tokio::test]
async fn test_routing_unknown_server_fails() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"cast":{{"url":"{}"}}}}}}"#, url),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["describe", "ghost", "dummy_tool"])
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let stderr = cmd.assert().failure().get_output().stderr.clone();
        let s = std::str::from_utf8(&stderr).unwrap().trim().to_string();
        let json: serde_json::Value =
            serde_json::from_str(&s).expect("stderr should be valid JSON");
        assert_eq!(json["error"]["code"], "COMMAND_ERROR");
        assert!(
            json["error"]["message"]
                .as_str()
                .expect("error message should be a string")
                .contains("ghost")
        );
    })
    .await?;

    ct.cancel();
    Ok(())
}

// ---------------------------------------------------------------------------
// S7 — list: handle unreachable servers gracefully
// ---------------------------------------------------------------------------

/// One reachable + one unreachable server:
/// - stdout is nested object with only the reachable server's key
/// - stderr contains a warning mentioning the unreachable server name
/// - exit code is 0
#[tokio::test]
async fn test_list_ignores_unreachable_server() -> anyhow::Result<()> {
    let (good_url, ct) = spawn_mock_server().await?;

    // Use a port that is not bound — guaranteed to be unreachable
    let bad_url = "http://127.0.0.1:1/mcp";

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"good":{{"url":"{good_url}"}},"bad":{{"url":"{bad_url}"}}}}}}"#,),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.arg("list")
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().clone();

        // stdout: nested object with only the good server's key
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
        assert!(json.is_object());
        let obj = json.as_object().unwrap();
        assert_eq!(obj.len(), 1, "only the reachable server should appear");
        let tools = json["good"]
            .as_array()
            .expect("good key should be an array");
        assert_eq!(tools[0]["name"], "dummy_tool");

        // stderr: a warning mentioning the bad server
        let stderr = std::str::from_utf8(&output.stderr).expect("stderr should be UTF-8");
        assert!(
            stderr.contains("bad"),
            "stderr should warn about the unreachable server 'bad', got: {stderr}"
        );
    })
    .await?;

    ct.cancel();
    Ok(())
}

/// When no --cast-mcp-url flag is passed, `list` should read the project-local
/// cast-mcp-client.json and connect to the server configured there.
#[tokio::test]
async fn test_list_reads_project_config() -> anyhow::Result<()> {
    // 1. Spawn a mock server
    let (url, ct) = spawn_mock_server().await?;

    // 2. Write a project-local config pointing at the mock server
    let tmpdir = tempfile::tempdir()?;
    let config_path = tmpdir.path().join("cast-mcp-client.json");
    std::fs::write(
        &config_path,
        format!(r#"{{"mcp":{{"cast":{{"url":"{}"}}}}}}"#, url),
    )?;

    // 3. Run `list` without --cast-mcp-url; config should supply the URL.
    // Unset CAST_MCP_URL so the ambient cast-injected env var does not override the config.
    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.arg("list")
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        assert!(json.is_object());
        assert_eq!(json["cast"][0]["name"], "dummy_tool");
    })
    .await?;

    ct.cancel();
    Ok(())
}

// ---------------------------------------------------------------------------
// S8 — status command
// ---------------------------------------------------------------------------

/// `status` with one reachable and one unreachable server:
/// - stdout is a JSON array with one entry per configured server
/// - reachable entry:   { "name": "good", "url": "...", "status": "ok" }
/// - unreachable entry: { "name": "bad",  "url": "...", "status": "error", "error": "..." }
/// - exit code is 0
#[tokio::test]
async fn test_status_command_output() -> anyhow::Result<()> {
    let (good_url, ct) = spawn_mock_server().await?;
    let bad_url = "http://127.0.0.1:1/mcp";

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"good":{{"url":"{good_url}"}},"bad":{{"url":"{bad_url}"}}}}}}"#,),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.arg("status")
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");

        assert!(json.is_array(), "output should be a JSON array");
        let entries = json.as_array().unwrap();
        assert_eq!(entries.len(), 2, "one entry per server");

        // Find entries by name (order is not guaranteed due to concurrent execution)
        let good = entries
            .iter()
            .find(|e| e["name"] == "good")
            .expect("should have a 'good' entry");
        let bad = entries
            .iter()
            .find(|e| e["name"] == "bad")
            .expect("should have a 'bad' entry");

        assert_eq!(good["status"], "ok");
        assert!(
            good["url"].as_str().is_some(),
            "good entry should have a url"
        );
        assert!(
            good.get("error").is_none() || good["error"].is_null(),
            "ok entry should not have an error field"
        );

        assert_eq!(bad["status"], "error");
        assert!(bad["url"].as_str().is_some(), "bad entry should have a url");
        assert!(
            bad["error"]
                .as_str()
                .map(|s| !s.is_empty())
                .unwrap_or(false),
            "error entry should have a non-empty error message"
        );
    })
    .await?;

    ct.cancel();
    Ok(())
}

// ---------------------------------------------------------------------------
// P1 — generate: create bash script wrappers from tool schemas
// ---------------------------------------------------------------------------

/// `generate --cast-mcp-url <url> --dir <tmpdir>` creates correctly named,
/// executable scripts and emits a JSON result envelope.
#[tokio::test]
async fn test_generate_creates_scripts() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;
    let out_dir = tempfile::tempdir()?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args([
        "generate",
        "--cast-mcp-url",
        &url,
        "--dir",
        out_dir.path().to_str().unwrap(),
    ]);

    tokio::task::spawn_blocking({
        let out_path = out_dir.path().to_path_buf();
        move || {
            let output = cmd.assert().success().get_output().stdout.clone();
            let json: serde_json::Value =
                serde_json::from_slice(&output).expect("stdout should be valid JSON");

            // JSON schema: { output_dir, scripts: [{server, tool, path}] }
            assert!(json["output_dir"].is_string(), "output_dir in JSON");
            let scripts = json["scripts"]
                .as_array()
                .expect("scripts should be an array");
            assert_eq!(scripts.len(), 1, "one script for dummy_tool");

            let entry = &scripts[0];
            assert_eq!(entry["server"], "cast");
            assert_eq!(entry["tool"], "dummy_tool");
            let path_str = entry["path"].as_str().expect("path should be a string");

            // File should exist on disk
            let script_path = std::path::Path::new(path_str);
            assert!(script_path.exists(), "script file should exist on disk");

            // Filename convention: cast-dummy-tool.sh
            assert_eq!(
                script_path.file_name().and_then(|n| n.to_str()),
                Some("cast-dummy-tool.sh"),
                "filename follows <server>-<tool>.sh convention"
            );

            // File should be in the requested output dir
            assert_eq!(
                script_path.parent(),
                Some(out_path.as_path()),
                "script lives inside --dir"
            );

            // Script must be executable (Unix permission bit 0o111)
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(script_path).unwrap().permissions();
            assert!(
                perms.mode() & 0o111 != 0,
                "script must have execute permission"
            );

            // Script content sanity check
            let content = std::fs::read_to_string(script_path).unwrap();
            assert!(
                content.starts_with("#!/usr/bin/env bash"),
                "shebang present"
            );
            assert!(content.contains("--message"), "--message flag in script");
        }
    })
    .await?;

    ct.cancel();
    Ok(())
}

/// Running a generated script with the correct args calls the tool and prints the text result.
#[tokio::test]
async fn test_generate_script_runs_correctly() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;
    let out_dir = tempfile::tempdir()?;

    let bin_dir = std::path::Path::new(env!("CARGO_BIN_EXE_cast-mcp-client"))
        .parent()
        .unwrap()
        .to_path_buf();
    let path_env = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    // Both the generate step and the script execution run inside a single spawn_blocking.
    // spawn_blocking prevents executor starvation: the mock server runs on the same
    // single-threaded tokio reactor as the test. Any blocking call outside spawn_blocking
    // would starve the reactor, preventing the server from responding → deadlock.
    tokio::task::spawn_blocking({
        let url = url.clone();
        let out_path = out_dir.path().to_path_buf();
        move || {
            // Step 1: generate scripts (blocks on MCP list_tools round-trip).
            Command::cargo_bin("cast-mcp-client")
                .unwrap()
                .args([
                    "generate",
                    "--cast-mcp-url",
                    &url,
                    "--dir",
                    out_path.to_str().unwrap(),
                ])
                .assert()
                .success();

            let script_path = out_path.join("cast-dummy-tool.sh");
            assert!(script_path.exists(), "script must have been generated");

            // Step 2: run the generated script.
            //   PATH includes the cargo bin dir so the script can find cast-mcp-client.
            //   CAST_MCP_URL points to the mock server so the script can reach it.
            let output = std::process::Command::new(&script_path)
                .args(["--message", "hello from script"])
                .env("PATH", &path_env)
                .env("CAST_MCP_URL", &url)
                .output()
                .expect("failed to execute generated script");

            assert!(
                output.status.success(),
                "script should exit 0; stderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains("echo: hello from script"),
                "stdout should contain tool output; got: {stdout}"
            );
        }
    })
    .await?;

    ct.cancel();
    Ok(())
}

/// Running a generated script against an `error_tool` exits 1 and writes the error to stderr.
#[tokio::test]
async fn test_generate_script_tool_error() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;
    let out_dir = tempfile::tempdir()?;

    // Generate the error_tool script directly using the pub helper,
    // since MockServerHandler only lists dummy_tool.
    let schema = serde_json::json!({"type": "object", "properties": {}});
    let error_tool = rmcp::model::Tool::new_with_raw(
        "error_tool".to_string(),
        Some("A tool that always errors".into()),
        schema.as_object().cloned().unwrap_or_default(),
    );
    let script_content = cast_mcp_client::generate_script("cast", &error_tool);
    let script_path = out_dir.path().join("cast-error-tool.sh");
    std::fs::write(&script_path, &script_content)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))?;

    let bin_dir = std::path::Path::new(env!("CARGO_BIN_EXE_cast-mcp-client"))
        .parent()
        .unwrap()
        .to_path_buf();
    let path_env = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    tokio::task::spawn_blocking(move || {
        let output = std::process::Command::new(&script_path)
            .env("PATH", &path_env)
            .env("CAST_MCP_URL", &url)
            .output()
            .expect("failed to execute error script");

        assert!(
            !output.status.success(),
            "script should exit non-zero on MCP error"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("something went wrong"),
            "error message should be on stderr; got: {stderr}"
        );
    })
    .await?;

    ct.cancel();
    Ok(())
}

// P2 — generate: resilience against unreachable servers
// ---------------------------------------------------------------------------

/// Two servers configured: one reachable, one not.
/// `generate` should:
/// - write scripts for the reachable server
/// - emit a warning on stderr naming the unreachable server
/// - still emit valid JSON output containing the generated scripts
/// - exit 0
#[tokio::test]
async fn test_generate_skips_unreachable_server() -> anyhow::Result<()> {
    let (good_url, ct) = spawn_mock_server().await?;
    let bad_url = "http://127.0.0.1:1/mcp"; // guaranteed unreachable
    let out_dir = tempfile::tempdir()?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"good":{{"url":"{good_url}"}},"bad":{{"url":"{bad_url}"}}}}}}"#,),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["generate", "--dir", out_dir.path().to_str().unwrap()])
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking({
        let out_path = out_dir.path().to_path_buf();
        move || {
            let output = cmd.assert().success().get_output().clone();

            // stderr: warning mentioning the bad server
            let stderr = std::str::from_utf8(&output.stderr).expect("stderr should be UTF-8");
            assert!(
                stderr.contains("bad"),
                "stderr should warn about unreachable server 'bad', got: {stderr}"
            );

            // stdout: valid JSON with scripts for the good server only
            let json: serde_json::Value =
                serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
            let scripts = json["scripts"]
                .as_array()
                .expect("scripts should be an array");
            assert_eq!(
                scripts.len(),
                1,
                "only good server's script should be present"
            );
            assert_eq!(scripts[0]["server"], "good");

            // Script file exists on disk
            let script_path = out_path.join("good-dummy-tool.sh");
            assert!(
                script_path.exists(),
                "good server script file should exist on disk"
            );
        }
    })
    .await?;

    ct.cancel();
    Ok(())
}
