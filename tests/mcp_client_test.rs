#![cfg(feature = "mcp")]

use assert_cmd::Command;
use predicates::prelude::*;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    model::{
        Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo,
        Tool,
    },
    service::RequestContext,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

// Import our client from the cast crate
use cast::commands::mcp::client::McpClient;

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
        let test_tool = Tool::new_with_raw(
            "dummy_tool".to_string(),
            Some("A mock tool for integration testing".into()),
            serde_json::Map::new(),
        );
        Ok(ListToolsResult {
            tools: vec![test_tool],
            next_cursor: None,
            meta: Default::default(),
        })
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
    let mut cmd = Command::cargo_bin("cast")?;
    cmd.args(["mcp", "list", "--url", &url])
        .env("CAST_LOG_DIR", std::env::temp_dir().join("cast-test-logs"));

    tokio::task::spawn_blocking(move || {
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("dummy_tool"))
            .stdout(predicate::str::contains(
                "A mock tool for integration testing",
            ));
    })
    .await?;

    ct.cancel();
    Ok(())
}
