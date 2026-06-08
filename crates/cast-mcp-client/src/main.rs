use cast_mcp_client::{
    build_server_map, call_tool_cmd, describe_tool_cmd, list_tools_cmd, print_json_error,
    resolve_cast_mcp_url,
};
use cast_mcp_client::config;
use clap::{Parser, Subcommand};

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
        /// Cast MCP server URL (overrides CAST_MCP_URL env and config)
        #[arg(long)]
        cast_mcp_url: Option<String>,

        /// Filter tools to a specific server by name
        #[arg(long)]
        server: Option<String>,
    },
    /// Show the input schema for a specific MCP tool
    Describe {
        /// Name of the tool to inspect
        tool_name: String,

        /// Cast MCP server URL (overrides CAST_MCP_URL env and config)
        #[arg(long)]
        cast_mcp_url: Option<String>,
    },
    /// Call a tool on the MCP server with JSON arguments
    Call {
        /// Name of the tool to call
        tool_name: String,

        /// JSON arguments as an inline string, or '-' to read from stdin.
        /// Defaults to '{}' if omitted and stdin is a terminal.
        #[arg(value_name = "JSON")]
        params: Option<String>,

        /// Cast MCP server URL (overrides CAST_MCP_URL env and config)
        #[arg(long)]
        cast_mcp_url: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Load config from disk (global + project-local); env substitution applied inside.
    let cfg = config::load();
    // Read CAST_MCP_URL at the binary boundary and pass it as a pure value.
    let env_url = std::env::var("CAST_MCP_URL").ok();

    let result = match cli.command {
        Commands::List {
            cast_mcp_url,
            server,
        } => {
            let cast_url = resolve_cast_mcp_url(cast_mcp_url, env_url, &cfg);
            let server_map = build_server_map(cast_url, &cfg);
            list_tools_cmd(server_map, server).await
        }
        Commands::Describe {
            tool_name,
            cast_mcp_url,
        } => {
            let cast_url = resolve_cast_mcp_url(cast_mcp_url, env_url, &cfg);
            let server_map = build_server_map(cast_url, &cfg);
            describe_tool_cmd(tool_name, server_map).await
        }
        Commands::Call {
            tool_name,
            params,
            cast_mcp_url,
        } => {
            let cast_url = resolve_cast_mcp_url(cast_mcp_url, env_url, &cfg);
            let server_map = build_server_map(cast_url, &cfg);
            call_tool_cmd(tool_name, params, server_map).await
        }
    };

    if let Err(err) = result {
        print_json_error("COMMAND_ERROR", &err.to_string());
        std::process::exit(1);
    }
}
