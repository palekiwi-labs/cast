use crate::config::ApprovedConfig;
use anyhow::Context as _;
use axum::Router;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tracing::warn;

use super::handler::McpHandler;

pub async fn run_http_server(host: String, port: u16, approved: ApprovedConfig) -> anyhow::Result<()> {
    let mcp_config = approved.mcp.clone();
    let host_env: std::collections::HashMap<String, String> = std::env::vars().collect();

    let handler = McpHandler::new(mcp_config, host_env)
        .context("Failed to initialize MCP handler from cast.json")?;

    // SSE keep-alives are enabled by default (every 15 s) in StreamableHttpServerConfig,
    // preventing Docker's virtual network from silently dropping idle SSE connections.
    let config = StreamableHttpServerConfig::default().with_allowed_hosts([
        "localhost",
        "127.0.0.1",
        "::1",
        // Allows agents running in Docker containers to reach the host.
        // Requires `--add-host host.docker.internal:host-gateway` on Linux.
        "host.docker.internal",
    ]);

    let service = StreamableHttpService::new(
        move || Ok(handler.clone()),
        LocalSessionManager::default().into(),
        config,
    );

    let app = Router::new().nest_service("/mcp", service);
    let addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {addr}"))?;

    if host == "0.0.0.0" {
        warn!("MCP server is listening on 0.0.0.0. This exposes the server to your local network!");
    }

    tracing::info!(addr = %addr, "cast MCP server listening");
    eprintln!("cast MCP server listening on http://{addr}/mcp");
    eprintln!("Connect from container:   http://host.docker.internal:{port}/mcp");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("MCP server encountered a fatal error")?;

    tracing::info!("cast MCP server stopped");
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C signal handler");
    tracing::info!("Shutdown signal received, stopping MCP server");
}
