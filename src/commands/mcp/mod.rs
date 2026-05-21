#[cfg(feature = "mcp")]
pub mod client;

#[cfg(feature = "mcp")]
pub mod exec;

#[cfg(feature = "mcp")]
pub mod handler;

#[cfg(feature = "mcp")]
mod server;

#[cfg(feature = "mcp")]
pub async fn list_tools(url: Option<String>) -> anyhow::Result<()> {
    let url = client::resolve_server_url(url);
    let mcp_client = client::McpClient::connect(&url).await?;
    let tools = mcp_client.list_tools().await?;
    for tool in &tools {
        let description = tool.description.as_deref().unwrap_or("");
        println!("{:<30} {}", tool.name, description);
    }
    mcp_client.shutdown().await
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
    }
}
