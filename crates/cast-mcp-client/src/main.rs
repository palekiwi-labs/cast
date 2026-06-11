use cast_mcp_client::config;
use std::collections::HashMap;
use cast_mcp_client::{
    build_server_map, call_tool_cmd, describe_tool_cmd, generate_scripts_cmd, list_tools_cmd,
    print_json_error, status_cmd,
};
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
    /// List tools exposed by configured MCP servers.
    /// Optionally filter to one or more servers by name.
    List {
        /// Cast MCP server URL (overrides CAST_MCP_URL env and config)
        #[arg(long)]
        cast_mcp_url: Option<String>,

        /// Optional server names to filter output (positional, repeatable)
        #[arg(value_name = "SERVER")]
        servers: Vec<String>,
    },
    /// Show the input schema for a specific MCP tool
    Describe {
        /// Name of the server hosting the tool
        server_name: String,

        /// Name of the tool to inspect
        tool_name: String,

        /// Cast MCP server URL (overrides CAST_MCP_URL env and config)
        #[arg(long)]
        cast_mcp_url: Option<String>,
    },
    /// Call a tool on the MCP server with JSON arguments
    Call {
        /// Name of the server hosting the tool
        server_name: String,

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
    /// Check the health of all configured MCP servers
    Status {
        /// Cast MCP server URL (overrides CAST_MCP_URL env and config)
        #[arg(long)]
        cast_mcp_url: Option<String>,
    },
    /// Generate bash script wrappers for every tool on configured MCP servers
    Generate {
        /// Output directory for generated scripts (created if absent)
        #[arg(long)]
        dir: std::path::PathBuf,

        /// Cast MCP server URL (overrides CAST_MCP_URL env and config)
        #[arg(long)]
        cast_mcp_url: Option<String>,

        /// Optional server names to restrict generation (positional, repeatable)
        #[arg(value_name = "SERVER")]
        servers: Vec<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Capture the process environment once at the binary boundary.
    // Library code receives an immutable snapshot — no env reads inside.
    let env_snapshot: HashMap<String, String> = std::env::vars().collect();
    // Read CAST_MCP_URL at the binary boundary and pass it as a pure value.
    let env_url = env_snapshot.get("CAST_MCP_URL").cloned();
    // Load config from disk (global + project-local); env substitution applied inside.
    let cfg = config::load(&env_snapshot);

    let result = match cli.command {
        Commands::List {
            cast_mcp_url,
            servers,
        } => {
            // Only flag or env var counts as an override; config-sourced URL is
            // handled directly inside build_server_map (preserves headers).
            let cast_override = cast_mcp_url.or(env_url);
            let server_map = build_server_map(cast_override, &cfg);
            list_tools_cmd(server_map, servers).await
        }
        Commands::Describe {
            server_name,
            tool_name,
            cast_mcp_url,
        } => {
            let cast_override = cast_mcp_url.or(env_url);
            let server_map = build_server_map(cast_override, &cfg);
            describe_tool_cmd(server_name, tool_name, server_map).await
        }
        Commands::Call {
            server_name,
            tool_name,
            params,
            cast_mcp_url,
        } => {
            let cast_override = cast_mcp_url.or(env_url);
            let server_map = build_server_map(cast_override, &cfg);
            call_tool_cmd(server_name, tool_name, params, server_map).await
        }
        Commands::Status { cast_mcp_url } => {
            let cast_override = cast_mcp_url.or(env_url);
            let server_map = build_server_map(cast_override, &cfg);
            status_cmd(server_map).await
        }
        Commands::Generate {
            dir,
            cast_mcp_url,
            servers,
        } => {
            let cast_override = cast_mcp_url.or(env_url);
            let server_map = build_server_map(cast_override, &cfg);
            generate_scripts_cmd(servers, &dir, server_map).await
        }
    };

    if let Err(err) = result {
        print_json_error("COMMAND_ERROR", &err.to_string());
        std::process::exit(1);
    }
}
