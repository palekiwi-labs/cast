#[cfg(feature = "mcp")]
pub mod exec;

#[cfg(feature = "mcp")]
pub mod handler;

#[cfg(feature = "mcp")]
mod server;

#[cfg(feature = "mcp")]
pub async fn run(
    command: crate::commands::cli::McpCommands,
    approved: crate::config::ApprovedConfig,
) -> anyhow::Result<()> {
    use crate::commands::cli::McpCommands;
    match command {
        McpCommands::Start { port, host } => server::run_http_server(host, port, approved).await,
    }
}
