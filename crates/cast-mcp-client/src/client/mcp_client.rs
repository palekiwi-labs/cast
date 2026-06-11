use http::{HeaderName, HeaderValue};
use rmcp::model::Tool;
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{Peer, RoleClient, ServiceExt};
use std::collections::HashMap;
use std::str::FromStr;

use crate::config::RemoteServerConfig;
use super::handler::McpClientHandler;


/// A clean, stateless wrapper around an rmcp client.
pub struct McpClient {
    peer: Peer<RoleClient>,
    service: RunningService<RoleClient, McpClientHandler>,
}

impl McpClient {
    /// Connect to an MCP server and perform the initialization handshake.
    ///
    /// Custom headers defined in `server.headers` are forwarded on every HTTP
    /// request. Header values must already have `{env:VAR}` substitutions
    /// applied (see `config::parse_from_str`).
    pub async fn connect(server: &RemoteServerConfig) -> anyhow::Result<Self> {
        // Convert HashMap<String, String> → HashMap<HeaderName, HeaderValue>
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

        // Serving the client handler automatically triggers the standard
        // JSON-RPC 2.0 handshake under the hood:
        // 1. Sends InitializeRequest
        // 2. Expects InitializeResult
        // 3. Sends InitializedNotification
        let handler = McpClientHandler;
        let service = handler.serve(transport).await?;
        let peer = service.peer().clone();

        Ok(Self { peer, service })
    }

    /// Call a tool on the connected MCP server with a JSON arguments map.
    pub async fn call_tool(
        &self,
        name: String,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> anyhow::Result<rmcp::model::CallToolResult> {
        let request = rmcp::model::CallToolRequestParams::new(name).with_arguments(arguments);
        let result = self.peer.call_tool(request).await?;
        Ok(result)
    }

    /// Retrieve all tools from the connected MCP server (discovery).
    pub async fn list_tools(&self) -> anyhow::Result<Vec<Tool>> {
        // `list_all_tools` automatically manages cursors and paginated results
        let tools = self.peer.list_all_tools().await?;
        Ok(tools)
    }

    /// Gracefully shut down the client, awaiting the background service task.
    ///
    /// Must be called explicitly to avoid leaving the Tokio runtime blocked on
    /// the rmcp session-deletion cleanup (5-second internal timeout).
    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.service
            .cancel()
            .await
            .map_err(|e| anyhow::anyhow!("MCP client shutdown join error: {e}"))?;
        Ok(())
    }
}
