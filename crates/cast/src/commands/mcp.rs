#[cfg(feature = "mcp")]
pub async fn run(
    command: crate::commands::cli::McpCommands,
    approved: crate::config::ApprovedConfig,
) -> anyhow::Result<()> {
    use crate::commands::cli::McpCommands;
    match command {
        McpCommands::Start { port, host } => {
            let mcp = approved.effective_mcp();
            let host = host.unwrap_or(mcp.hostname);
            let port = port.unwrap_or(mcp.port);
            crate::mcp::server::run_http_server(host, port, approved).await
        }
    }
}
