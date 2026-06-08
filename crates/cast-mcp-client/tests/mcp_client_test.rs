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

    // 2. Invoke `cast mcp list --cast-mcp-url <mock_url>` as a subprocess.
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
        assert!(json.is_array());
        assert_eq!(json[0]["name"], "cast/dummy_tool");
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

    // 2. Invoke `cast mcp describe dummy_tool --cast-mcp-url <mock_url>` as a subprocess.
    // spawn_blocking prevents executor starvation (same pattern as list test).
    let url = format!("http://{addr}/mcp");
    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["describe", "cast/dummy_tool", "--cast-mcp-url", &url]);

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
    cmd.args(["describe", "cast/nonexistent_tool", "--cast-mcp-url", &url]);

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
        "cast/dummy_tool",
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
    cmd.args(["call", "cast/dummy_tool", "-", "--cast-mcp-url", &url])
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
    cmd.args(["call", "cast/nonexistent_tool", "{}", "--cast-mcp-url", &url]);

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
    cmd.args(["call", "cast/error_tool", "{}", "--cast-mcp-url", &url]);

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
// S5 — multi-server flat prefixed list
// ---------------------------------------------------------------------------

/// No servers configured → stdout is exactly `[]`.
#[tokio::test]
async fn test_list_empty_config_returns_empty_array() -> anyhow::Result<()> {
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
        assert_eq!(s, "[]");
    })
    .await?;

    Ok(())
}

/// Single server "cast" with dummy_tool → output name is "cast/dummy_tool".
#[tokio::test]
async fn test_list_prefixed_tools_single_server() -> anyhow::Result<()> {
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
        assert!(json.is_array());
        assert_eq!(json.as_array().unwrap().len(), 1);
        assert_eq!(json[0]["name"], "cast/dummy_tool");
    })
    .await?;

    ct.cancel();
    Ok(())
}

/// Two servers configured; `list --server sentry` returns only sentry tools.
#[tokio::test]
async fn test_list_filter_by_server() -> anyhow::Result<()> {
    let (cast_url, ct1) = spawn_mock_server().await?;
    let (sentry_url, ct2) = spawn_mock_server().await?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(
            r#"{{"mcp":{{"cast":{{"url":"{cast_url}"}},"sentry":{{"url":"{sentry_url}"}}}}}}"#,
        ),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["list", "--server", "sentry"])
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        assert!(json.is_array());
        let tools = json.as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "sentry/dummy_tool");
    })
    .await?;

    ct1.cancel();
    ct2.cancel();
    Ok(())
}

/// `list --server ghost` (unknown server) → non-zero exit + COMMAND_ERROR JSON on stderr.
#[tokio::test]
async fn test_list_unknown_server_fails() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"cast":{{"url":"{}"}}}}}}"#, url),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["list", "--server", "ghost"])
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let stderr = cmd.assert().failure().get_output().stderr.clone();
        let s = std::str::from_utf8(&stderr).unwrap().trim().to_string();
        let json: serde_json::Value =
            serde_json::from_str(&s).expect("stderr should be valid JSON");
        assert_eq!(json["error"]["code"], "COMMAND_ERROR");
        assert!(json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("ghost"));
    })
    .await?;

    ct.cancel();
    Ok(())
}

// ---------------------------------------------------------------------------
// S6 — describe/call: server/tool format + routing
// ---------------------------------------------------------------------------

/// `describe cast/dummy_tool` succeeds; stdout has bare name "dummy_tool" (no prefix in describe output).
#[tokio::test]
async fn test_describe_server_slash_tool_format() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"cast":{{"url":"{}"}}}}}}"#, url),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["describe", "cast/dummy_tool"])
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        assert!(json.is_object());
        // describe returns the raw tool object — name is bare (not prefixed)
        assert_eq!(json["name"], "dummy_tool");
        assert_eq!(json["description"], "A mock tool for integration testing");
        assert!(json["inputSchema"]["properties"]["message"].is_object());
    })
    .await?;

    ct.cancel();
    Ok(())
}

/// `call cast/dummy_tool '{"message":"hello"}'` succeeds and echoes correctly.
#[tokio::test]
async fn test_call_server_slash_tool_format() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"cast":{{"url":"{}"}}}}}}"#, url),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["call", "cast/dummy_tool", r#"{"message":"hello"}"#])
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let output = cmd.assert().success().get_output().stdout.clone();
        let json: serde_json::Value =
            serde_json::from_slice(&output).expect("stdout should be valid JSON");
        assert_eq!(json["content"][0]["type"], "text");
        assert!(json["content"][0]["text"]
            .as_str()
            .expect("content should be text")
            .contains("echo: hello"));
        assert!(json["isError"].is_null() || json["isError"] == false);
    })
    .await?;

    ct.cancel();
    Ok(())
}

/// `describe dummy_tool` (no slash) → failure with COMMAND_ERROR mentioning "server/tool".
#[tokio::test]
async fn test_routing_no_separator_fails() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"cast":{{"url":"{}"}}}}}}"#, url),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["describe", "dummy_tool"])
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let stderr = cmd.assert().failure().get_output().stderr.clone();
        let s = std::str::from_utf8(&stderr).unwrap().trim().to_string();
        let json: serde_json::Value =
            serde_json::from_str(&s).expect("stderr should be valid JSON");
        assert_eq!(json["error"]["code"], "COMMAND_ERROR");
        assert!(json["error"]["message"]
            .as_str()
            .expect("error message should be a string")
            .contains("server/tool"));
    })
    .await?;

    ct.cancel();
    Ok(())
}

/// `describe ghost/dummy_tool` (unknown server) → failure with COMMAND_ERROR mentioning "ghost".
#[tokio::test]
async fn test_routing_unknown_server_fails() -> anyhow::Result<()> {
    let (url, ct) = spawn_mock_server().await?;

    let tmpdir = tempfile::tempdir()?;
    std::fs::write(
        tmpdir.path().join("cast-mcp-client.json"),
        format!(r#"{{"mcp":{{"cast":{{"url":"{}"}}}}}}"#, url),
    )?;

    let mut cmd = Command::cargo_bin("cast-mcp-client")?;
    cmd.args(["describe", "ghost/dummy_tool"])
        .current_dir(tmpdir.path())
        .env_remove("CAST_MCP_URL");

    tokio::task::spawn_blocking(move || {
        let stderr = cmd.assert().failure().get_output().stderr.clone();
        let s = std::str::from_utf8(&stderr).unwrap().trim().to_string();
        let json: serde_json::Value =
            serde_json::from_str(&s).expect("stderr should be valid JSON");
        assert_eq!(json["error"]["code"], "COMMAND_ERROR");
        assert!(json["error"]["message"]
            .as_str()
            .expect("error message should be a string")
            .contains("ghost"));
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
        assert!(json.is_array());
        assert_eq!(json[0]["name"], "cast/dummy_tool");
    })
    .await?;

    ct.cancel();
    Ok(())
}
