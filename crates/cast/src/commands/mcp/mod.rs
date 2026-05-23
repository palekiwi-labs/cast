#[cfg(feature = "mcp")]
pub mod exec;

#[cfg(feature = "mcp")]
pub mod handler;

#[cfg(feature = "mcp")]
mod server;

#[cfg(feature = "mcp")]
pub async fn list_tools(url: Option<String>) -> anyhow::Result<()> {
    cast_mcp_client::list_tools_cmd(url).await
}

#[cfg(feature = "mcp")]
pub async fn describe_tool(tool_name: String, url: Option<String>) -> anyhow::Result<()> {
    cast_mcp_client::describe_tool_cmd(tool_name, url).await
}

#[cfg(feature = "mcp")]
pub async fn call_tool_cmd(
    tool_name: String,
    params: Option<String>,
    url: Option<String>,
) -> anyhow::Result<()> {
    cast_mcp_client::call_tool_cmd(tool_name, params, url).await
}

#[cfg(feature = "mcp")]
pub async fn run(
    command: crate::commands::cli::McpCommands,
    approved: crate::config::ApprovedConfig,
) -> anyhow::Result<()> {
    use crate::commands::cli::McpCommands;
    match command {
        McpCommands::Start { port, host } => {
            let host = host.unwrap_or_else(|| approved.mcp.hostname.clone());
            let port = port.unwrap_or(approved.mcp.port);
            server::run_http_server(host, port, approved).await
        }
        McpCommands::List { url } => list_tools(url).await,
        McpCommands::Describe { tool_name, url } => describe_tool(tool_name, url).await,
        McpCommands::Call { tool_name, params, url } => call_tool_cmd(tool_name, params, url).await,
    }
}
