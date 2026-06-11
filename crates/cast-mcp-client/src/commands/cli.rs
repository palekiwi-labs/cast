use crate::config;
use crate::server_map::build_server_map;
use clap::{Parser, Subcommand};
use std::collections::HashMap;

use super::call::call_tool_cmd;
use super::describe::describe_tool_cmd;
use super::generate::generate_scripts_cmd;
use super::list::list_tools_cmd;
use super::status::status_cmd;

#[derive(Parser)]
#[command(name = "cast-mcp-client")]
#[command(about = "Lightweight MCP client for cast", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    // Capture the process environment once at the binary boundary.
    // Library code receives an immutable snapshot — no env reads inside.
    let env_snapshot: HashMap<String, String> = std::env::vars().collect();
    // Read CAST_MCP_URL at the binary boundary and pass it as a pure value.
    let env_url = env_snapshot.get("CAST_MCP_URL").cloned();
    // Load config from disk (global + project-local); env substitution applied
    // inside.
    let cfg = config::load(&env_snapshot);

    match cli.command {
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
    }
}

pub fn print_json_error(code: &str, message: &str) {
    let payload = serde_json::json!({
        "error": {
            "code": code,
            "message": message
        }
    });
    eprintln!(
        "{}",
        serde_json::to_string_pretty(&payload)
            .expect("static JSON payload should always serialize")
    );
}
