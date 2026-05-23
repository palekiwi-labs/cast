use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, Tool};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{ClientHandler, Peer, RoleClient, ServiceExt};

/// Resolve the MCP server URL based on the following priority:
/// 1. Explicitly provided `url` flag.
/// 2. `CAST_MCP_URL` environment variable (typically injected by `cast run`).
/// 3. Default `http://127.0.0.1:8080/mcp`.
pub fn resolve_server_url(explicit_url: Option<String>) -> String {
    // 1. Explicit override
    if let Some(url) = explicit_url {
        return url;
    }

    // 2. Environment variable
    if let Ok(url) = std::env::var("CAST_MCP_URL") {
        return url;
    }

    // 3. Default
    "http://127.0.0.1:8080/mcp".to_string()
}

/// A minimal handler to manage client-side callbacks (e.g., logging or sampling).
/// Required by the `rmcp` crate to serve as a client service.
#[derive(Clone, Debug, Default)]
pub struct McpClientHandler;

impl ClientHandler for McpClientHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("cast-cli-client", env!("CARGO_PKG_VERSION")),
        )
    }
}

/// A clean, stateless wrapper around an rmcp client.
pub struct McpClient {
    peer: Peer<RoleClient>,
    service: RunningService<RoleClient, McpClientHandler>,
}

impl McpClient {
    /// Connect to an MCP server and perform the initialization handshake.
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        // Configure standard SSE HttpClient transport with automatic recovery on expired sessions
        let config =
            StreamableHttpClientTransportConfig::with_uri(url).reinit_on_expired_session(true);
        let transport = StreamableHttpClientTransport::from_config(config);

        // Serving the client handler automatically triggers the standard JSON-RPC 2.0 handshake under the hood:
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
        // `list_all_tools` automatically manages cursors and paginated results under the hood
        let tools = self.peer.list_all_tools().await?;
        Ok(tools)
    }

    /// Gracefully shut down the client, awaiting the background service task.
    ///
    /// Must be called explicitly to avoid leaving the Tokio runtime blocked on
    /// the rmcp session-deletion cleanup (which has a 5-second internal timeout).
    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.service
            .cancel()
            .await
            .map_err(|e| anyhow::anyhow!("MCP client shutdown join error: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_resolve_explicit_url() {
        let url = Some("http://example.com/mcp".to_string());
        assert_eq!(resolve_server_url(url), "http://example.com/mcp");
    }

    #[test]
    fn test_resolve_env_url() {
        unsafe {
            env::set_var("CAST_MCP_URL", "http://env.com/mcp");
        }
        let result = resolve_server_url(None);
        unsafe {
            env::remove_var("CAST_MCP_URL");
        }
        assert_eq!(result, "http://env.com/mcp");
    }
}
