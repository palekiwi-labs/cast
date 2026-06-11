use crate::client::McpClient;
use crate::config::RemoteServerConfig;
use std::collections::HashMap;

/// Perform a concurrent health check of all servers in `server_map`.
///
/// Outputs a JSON array to stdout — one entry per server — sorted by name for
/// deterministic output. Each entry has the shape:
///
/// ```json
/// { "name": "cast", "url": "http://...", "status": "ok" }
/// { "name": "bad",  "url": "http://...", "status": "error", "error": "..." }
/// ```
///
/// Exit code is always 0; individual server failures are reported inline.
pub async fn status_cmd(
    server_map: HashMap<String, RemoteServerConfig>,
) -> anyhow::Result<()> {
    let futures: Vec<_> = server_map
        .into_iter()
        .map(|(name, server)| async move {
            let url = server.url.clone();
            let result: anyhow::Result<()> = async {
                let client = McpClient::connect(&server).await?;
                client.shutdown().await?;
                Ok(())
            }
            .await;
            (name, url, result)
        })
        .collect();

    let mut results = futures::future::join_all(futures).await;

    // Sort by server name for deterministic output
    results.sort_by(|a, b| a.0.cmp(&b.0));

    let entries: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(name, url, result)| match result {
            Ok(()) => serde_json::json!({
                "name": name,
                "url": url,
                "status": "ok",
            }),
            Err(e) => serde_json::json!({
                "name": name,
                "url": url,
                "status": "error",
                "error": e.to_string(),
            }),
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}
