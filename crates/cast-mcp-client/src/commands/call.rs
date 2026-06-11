use crate::client::McpClient;
use crate::config::RemoteServerConfig;
use std::collections::HashMap;

pub async fn call_tool_cmd(
    server_name: String,
    tool_name: String,
    params: Option<String>,
    server_map: HashMap<String, RemoteServerConfig>,
) -> anyhow::Result<()> {
    let server = server_map.get(server_name.as_str()).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown server '{}'. Check your cast-mcp-client.json or pass --cast-mcp-url.",
            server_name
        )
    })?;

    let arguments = read_params(params)?;
    let mcp_client = McpClient::connect(server).await?;
    let result = mcp_client.call_tool(tool_name, arguments).await?;

    println!("{}", serde_json::to_string_pretty(&result)?);

    mcp_client.shutdown().await?;

    Ok(())
}

/// Read JSON parameters from an inline string, explicit stdin (`-`), piped
/// stdin, or default to `{}`.
pub fn read_params(
    params: Option<String>,
) -> anyhow::Result<serde_json::Map<String, serde_json::Value>> {
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
        // Nothing provided: read from stdin if it's a pipe, otherwise use
        // empty object
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

    value.as_object().cloned().ok_or_else(|| {
        anyhow::anyhow!("Parameters must be a JSON object, e.g. '{{\"key\": \"val\"}}'")
    })
}
