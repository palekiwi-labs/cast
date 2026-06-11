use crate::client::McpClient;
use crate::config::RemoteServerConfig;
use std::collections::HashMap;

pub async fn describe_tool_cmd(
    server_name: String,
    tool_name: String,
    server_map: HashMap<String, RemoteServerConfig>,
) -> anyhow::Result<()> {
    let server = server_map.get(server_name.as_str()).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown server '{}'. Check your cast-mcp-client.json or pass --cast-mcp-url.",
            server_name
        )
    })?;

    let mcp_client = McpClient::connect(server).await?;
    let tools = mcp_client.list_tools().await?;

    let tool = tools
        .into_iter()
        .find(|t| t.name == tool_name.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown tool '{}' on server '{}'. Run 'cast-mcp-client list' to see available tools.",
                tool_name,
                server_name
            )
        })?;

    println!("{}", serde_json::to_string_pretty(&tool)?);
    mcp_client.shutdown().await
}
