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
pub async fn describe_tool(tool_name: String, url: Option<String>) -> anyhow::Result<()> {
    let url = client::resolve_server_url(url);
    let mcp_client = client::McpClient::connect(&url).await?;
    let tools = mcp_client.list_tools().await?;

    let tool = tools
        .into_iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown tool '{}'. Run 'cast mcp list' to see available tools.",
                tool_name
            )
        })?;

    print_tool_schema(&tool.name, tool.description.as_deref(), &tool.input_schema);
    mcp_client.shutdown().await
}

/// Read JSON parameters from an inline string, explicit stdin (`-`), piped stdin, or default to `{}`.
#[cfg(feature = "mcp")]
pub fn read_params(params: Option<String>) -> anyhow::Result<serde_json::Map<String, serde_json::Value>> {
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

    value
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Parameters must be a JSON object, e.g. '{{\"key\": \"val\"}}'"))
}

#[cfg(feature = "mcp")]
pub async fn call_tool_cmd(
    tool_name: String,
    params: Option<String>,
    url: Option<String>,
) -> anyhow::Result<()> {
    let arguments = read_params(params)?;

    let url = client::resolve_server_url(url);
    let mcp_client = client::McpClient::connect(&url).await?;
    let result = mcp_client.call_tool(tool_name.clone(), arguments).await?;

    for item in &result.content {
        use rmcp::model::RawContent;
        match &item.raw {
            RawContent::Text(t) => println!("{}", t.text),
            other => println!("{}", serde_json::to_string_pretty(other)?),
        }
    }

    mcp_client.shutdown().await?;

    if result.is_error.unwrap_or(false) {
        anyhow::bail!("Tool '{}' reported an error.", tool_name);
    }

    Ok(())
}

/// Pretty-print an MCP tool's input schema to stdout.
#[cfg(feature = "mcp")]
pub fn print_tool_schema(
    name: &str,
    description: Option<&str>,
    schema: &serde_json::Map<String, serde_json::Value>,
) {
    use std::collections::HashSet;

    println!("Tool: {}", name);
    if let Some(desc) = description {
        println!("Description: {}", desc);
    }
    println!();

    let properties = schema
        .get("properties")
        .and_then(|v| v.as_object());

    let required: HashSet<&str> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    match properties {
        None => {
            println!("Parameters: none");
        }
        Some(props) if props.is_empty() => {
            println!("Parameters: none");
        }
        Some(props) => {
            println!("Parameters:");
            for (prop_name, prop_val) in props {
                let prop_type = prop_val
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("any");
                let prop_desc = prop_val
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("");
                let required_label = if required.contains(prop_name.as_str()) {
                    "required"
                } else {
                    "optional"
                };

                println!(
                    "  {} ({}, {}): {}",
                    prop_name, prop_type, required_label, prop_desc
                );
            }
        }
    }

    println!();
    println!(
        "Example: cast mcp call {} '{{\"key\": \"value\"}}'",
        name
    );
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
