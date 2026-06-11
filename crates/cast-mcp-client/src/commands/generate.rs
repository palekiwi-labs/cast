use crate::client::McpClient;
use crate::config::RemoteServerConfig;
use crate::generate;
use rmcp::model::Tool;
use std::collections::HashMap;

/// Generate bash script wrappers for every tool on the configured servers.
///
/// - Queries all (or filtered) servers concurrently to discover their tools.
/// - Writes one executable `.sh` script per tool into `output_dir` (created
///   if absent).
/// - Prints a JSON envelope to stdout listing every generated script.
///
/// Output schema:
/// ```json
/// { "output_dir": "/abs/path", "scripts": [{ "server", "tool", "path" }] }
/// ```
pub async fn generate_scripts_cmd(
    server_filter: Vec<String>,
    output_dir: &std::path::Path,
    server_map: HashMap<String, RemoteServerConfig>,
) -> anyhow::Result<()> {
    // Validate filter names upfront.
    for name in &server_filter {
        if !server_map.contains_key(name.as_str()) {
            anyhow::bail!(
                "Unknown server '{}'. Check your cast-mcp-client.json or pass --cast-mcp-url.",
                name
            );
        }
    }

    // Determine which servers to query.
    let targets: Vec<(String, RemoteServerConfig)> = if server_filter.is_empty() {
        server_map.into_iter().collect()
    } else {
        server_filter
            .into_iter()
            .map(|name| {
                let server = server_map[name.as_str()].clone();
                (name, server)
            })
            .collect()
    };

    // Build a name → url lookup for the manifest (before targets is consumed).
    let targets_with_url: HashMap<String, String> = targets
        .iter()
        .map(|(name, server)| (name.clone(), server.url.clone()))
        .collect();

    // Fetch tool lists from all target servers concurrently.
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

    // Ensure the output directory exists.
    std::fs::create_dir_all(output_dir)?;
    let abs_dir = output_dir.canonicalize()?;

    let mut script_entries: Vec<serde_json::Value> = Vec::new();
    // manifest: servers -> { url, tools: { tool_name -> filename } }
    let mut manifest_servers: serde_json::Map<String, serde_json::Value> =
        serde_json::Map::new();

    for (server_name, result) in results {
        let tools = match result {
            Ok(tools) => tools,
            Err(e) => {
                eprintln!("Warning: server '{}' is unreachable: {}", server_name, e);
                continue;
            }
        };

        // Retrieve the server URL for the manifest (empty string if absent).
        let server_url = targets_with_url
            .get(&server_name)
            .cloned()
            .unwrap_or_default();

        let mut manifest_tools: serde_json::Map<String, serde_json::Value> =
            serde_json::Map::new();

        for tool in &tools {
            let script_content = generate::generate_script(&server_name, tool);
            let filename = format!(
                "{}-{}.sh",
                server_name,
                generate::camel_to_kebab(tool.name.as_ref())
            );
            let path = abs_dir.join(&filename);
            std::fs::write(&path, &script_content)?;

            // Set executable bit (rwxr-xr-x = 0o755).
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;

            script_entries.push(serde_json::json!({
                "server": server_name,
                "tool": tool.name.as_ref(),
                "path": path.to_string_lossy(),
            }));

            manifest_tools.insert(
                tool.name.as_ref().to_string(),
                serde_json::Value::String(filename),
            );
        }

        manifest_servers.insert(
            server_name.clone(),
            serde_json::json!({
                "url": server_url,
                "tools": manifest_tools,
            }),
        );
    }

    // Write manifest.json into the output directory.
    let manifest = serde_json::json!({
        "generated_at": generate::now_unix(),
        "servers": manifest_servers,
    });
    let manifest_path = abs_dir.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    let output = serde_json::json!({
        "output_dir": abs_dir.to_string_lossy(),
        "scripts": script_entries,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
