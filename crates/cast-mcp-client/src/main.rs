use clap::{Parser, Subcommand};
use cast_mcp_client::{list_tools_cmd, describe_tool_cmd, call_tool_cmd};

#[derive(Parser)]
#[command(name = "cast-mcp-client")]
#[command(about = "Lightweight MCP client for cast", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List tools exposed by the MCP server
    List {
        /// MCP server URL (overrides CAST_MCP_URL env and default)
        #[arg(long)]
        url: Option<String>,
    },
    /// Show the input schema for a specific MCP tool
    Describe {
        /// Name of the tool to inspect
        tool_name: String,

        /// MCP server URL (overrides CAST_MCP_URL env and default)
        #[arg(long)]
        url: Option<String>,
    },
    /// Call a tool on the MCP server with JSON arguments
    Call {
        /// Name of the tool to call
        tool_name: String,

        /// JSON arguments as an inline string, or '-' to read from stdin.
        /// Defaults to '{}' if omitted and stdin is a terminal.
        #[arg(value_name = "JSON")]
        params: Option<String>,

        /// MCP server URL (overrides CAST_MCP_URL env and default)
        #[arg(long)]
        url: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List { url } => list_tools_cmd(url).await,
        Commands::Describe { tool_name, url } => describe_tool_cmd(tool_name, url).await,
        Commands::Call { tool_name, params, url } => call_tool_cmd(tool_name, params, url).await,
    }
}
