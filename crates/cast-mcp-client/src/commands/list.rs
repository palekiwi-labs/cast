use crate::client::McpClient;
use crate::config::RemoteServerConfig;
use rmcp::model::Tool;
use std::collections::HashMap;

pub async fn list_tools_cmd(
    server_map: HashMap<String, RemoteServerConfig>,
    servers: Vec<String>,
) -> anyhow::Result<()> {
    // If server name filters were given, validate each one exists in the map first.
    for name in &servers {
        if !server_map.contains_key(name.as_str()) {
            anyhow::bail!(
                "Unknown server '{}'. Check your cast-mcp-client.json or run without a server filter to list all.",
                name
            );
        }
    }

    // Build the set of servers to query (all, or the requested subset).
    let targets: Vec<(String, RemoteServerConfig)> = if servers.is_empty() {
        server_map.into_iter().collect()
    } else {
        servers
            .into_iter()
            .map(|name| {
                let server = server_map[name.as_str()].clone();
                (name, server)
            })
            .collect()
    };

    if targets.is_empty() {
        println!("{{}}");
        return Ok(());
    }

    // Query all target servers concurrently.
    // Each future resolves to (server_name, Result<Vec<Tool>>) so errors can
    // be attributed to the specific server that failed.
    let futures: Vec<_> = targets
        .into_iter()
        .map(|(name, server)| async move {
            let result: anyhow::Result<Vec<Tool>> = async {
                let client = McpClient::connect(&server).await?;
                let tools = client.list_tools().await?;
                client.shutdown().await?;
                Ok(tools)
            }
            .await;
            (name, result)
        })
        .collect();

    let results = futures::future::join_all(futures).await;

    // Build nested output object: { "server_name": [tools...] }
    // Unreachable servers are warned on stderr and omitted from output.
    let mut output: HashMap<String, Vec<Tool>> = HashMap::new();
    for (server_name, result) in results {
        match result {
            Ok(tools) => {
                output.insert(server_name, tools);
            }
            Err(e) => eprintln!("Warning: server '{}' is unreachable: {}", server_name, e),
        }
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
