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

pub async fn list_tools_cmd(url: Option<String>) -> anyhow::Result<()> {
    let url = resolve_server_url(url);
    let mcp_client = McpClient::connect(&url).await?;
    let tools = mcp_client.list_tools().await?;
    println!("{}", serde_json::to_string_pretty(&tools)?);
    mcp_client.shutdown().await
}

pub async fn describe_tool_cmd(tool_name: String, url: Option<String>) -> anyhow::Result<()> {
    let url = resolve_server_url(url);
    let mcp_client = McpClient::connect(&url).await?;
    let tools = mcp_client.list_tools().await?;

    let tool = tools
        .into_iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| {
            print_json_error(
                "TOOL_NOT_FOUND",
                &format!(
                    "Unknown tool '{}'. Run 'cast-mcp-client list' to see available tools.",
                    tool_name
                ),
            );
            anyhow::anyhow!("tool not found")
        })?;

    println!("{}", serde_json::to_string_pretty(&tool)?);
    mcp_client.shutdown().await
}

fn print_json_error(code: &str, message: &str) {
    let payload = serde_json::json!({
        "error": {
            "code": code,
            "message": message
        }
    });
    eprintln!(
        "{}",
        serde_json::to_string_pretty(&payload).unwrap_or_default()
    );
}

pub async fn call_tool_cmd(
    tool_name: String,
    params: Option<String>,
    url: Option<String>,
) -> anyhow::Result<()> {
    let arguments = read_params(params)?;

    let url = resolve_server_url(url);
    let mcp_client = McpClient::connect(&url).await?;
    let result = mcp_client.call_tool(tool_name.clone(), arguments).await?;

    println!("{}", serde_json::to_string_pretty(&result)?);

    mcp_client.shutdown().await?;

    Ok(())
}

/// Read JSON parameters from an inline string, explicit stdin (`-`), piped stdin, or default to `{}`.
pub fn read_params(
    params: Option<String>,
) -> anyhow::Result<serde_json::Map<String, serde_json::Value>> {
    use std::io::Read;

    let raw = match params.as_deref() {
        // Explicit stdin flag
        Some("-") => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        }
        // Inline JSON string provided directly
        Some(s) => s.to_string(),
        // Nothing provided: read from stdin if it's a pipe, otherwise use empty object
        None => {
            use std::io::IsTerminal;
            if !std::io::stdin().is_terminal() {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf
            } else {
                "{}".to_string()
            }
        }
    };

    let trimmed = raw.trim();
    let value: serde_json::Value = if trimmed.is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_str(trimmed)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON parameters: {}", e))?
    };

    value.as_object().cloned().ok_or_else(|| {
        anyhow::anyhow!("Parameters must be a JSON object, e.g. '{{\"key\": \"val\"}}'")
    })
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
