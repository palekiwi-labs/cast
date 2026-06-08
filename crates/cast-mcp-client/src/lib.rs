use http::{HeaderName, HeaderValue};
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, Tool};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{ClientHandler, Peer, RoleClient, ServiceExt};
use std::collections::HashMap;
use std::str::FromStr;

pub mod config;

/// Resolve the MCP server URL based on the following priority:
/// 1. Explicitly provided `url` flag.
/// 2. `CAST_MCP_URL` environment variable (typically injected by `cast run`).
/// 3. Default `http://127.0.0.1:8080/mcp`.
pub fn resolve_server_url(explicit_url: Option<String>) -> String {
    // 1. Explicit override
    if let Some(url) = explicit_url {
        return url;
    }

    // 2. Environment variable
    if let Ok(url) = std::env::var("CAST_MCP_URL") {
        return url;
    }

    // 3. Default
    "http://127.0.0.1:8080/mcp".to_string()
}

/// Build the active server map from config, optionally injecting a cast URL from flag/env.
///
/// - All config entries with `enabled: true` are included (except `"cast"`, which is special).
/// - If `cast_url` is `Some(url)` (sourced from CLI flag or env var), a bare `"cast"` entry is
///   injected with that URL and no headers, overriding any `"cast"` entry in the config.
/// - If `cast_url` is `None`, the config's `"cast"` entry is included as-is (with its headers).
pub fn build_server_map(
    cast_url: Option<String>,
    config: &config::ClientConfig,
) -> HashMap<String, config::RemoteServerConfig> {
    let mut map = HashMap::new();

    // Include all enabled non-cast entries from config
    for (name, server) in &config.mcp {
        if name == "cast" {
            continue; // handled separately below
        }
        if server.enabled {
            map.insert(name.clone(), server.clone());
        }
    }

    // Resolve the "cast" entry
    match cast_url {
        Some(url) => {
            // URL came from flag/env — inject bare entry, no headers
            map.insert(
                "cast".to_string(),
                config::RemoteServerConfig {
                    url,
                    headers: HashMap::new(),
                    enabled: true,
                },
            );
        }
        None => {
            // URL from config (if present and enabled) — use full entry including headers
            if let Some(server) = config.mcp.get("cast")
                && server.enabled
            {
                map.insert("cast".to_string(), server.clone());
            }
        }
    }

    map
}

/// Resolve the cast server URL for the multi-server client, with priority:
/// 1. Explicit `--cast-mcp-url` CLI flag value.
/// 2. `CAST_MCP_CLIENT_URL` environment variable (caller responsibility to read and pass in).
/// 3. `mcp.cast.url` from the loaded config file.
///
/// Returns `None` if no source provides a URL (cast server is simply absent).
///
/// This function is pure — it has no side effects and does not read the environment directly.
/// The caller reads `std::env::var("CAST_MCP_CLIENT_URL").ok()` and passes it as `env_url`.
pub fn resolve_cast_mcp_url(
    explicit: Option<String>,
    env_url: Option<String>,
    config: &config::ClientConfig,
) -> Option<String> {
    explicit
        .or(env_url)
        .or_else(|| config.mcp.get("cast").map(|s| s.url.clone()))
}

/// A minimal handler to manage client-side callbacks (e.g., logging or sampling).
/// Required by the `rmcp` crate to serve as a client service.
#[derive(Clone, Debug, Default)]
pub struct McpClientHandler;

impl ClientHandler for McpClientHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("cast-cli-client", env!("CARGO_PKG_VERSION")),
        )
    }
}

/// A clean, stateless wrapper around an rmcp client.
pub struct McpClient {
    peer: Peer<RoleClient>,
    service: RunningService<RoleClient, McpClientHandler>,
}

impl McpClient {
    /// Connect to an MCP server and perform the initialization handshake.
    ///
    /// Custom headers defined in `server.headers` are forwarded on every HTTP request.
    /// Header values must already have `{env:VAR}` substitutions applied (see `config::parse_from_str`).
    pub async fn connect(server: &config::RemoteServerConfig) -> anyhow::Result<Self> {
        // Convert HashMap<String, String> → HashMap<HeaderName, HeaderValue> (required by rmcp)
        let mut http_headers: HashMap<HeaderName, HeaderValue> = HashMap::new();
        for (k, v) in &server.headers {
            let name = HeaderName::from_str(k)
                .map_err(|e| anyhow::anyhow!("invalid header name '{}': {}", k, e))?;
            let value = HeaderValue::from_str(v)
                .map_err(|e| anyhow::anyhow!("invalid header value for '{}': {}", k, e))?;
            http_headers.insert(name, value);
        }

        let config = StreamableHttpClientTransportConfig::with_uri(server.url.as_str())
            .custom_headers(http_headers)
            .reinit_on_expired_session(true);
        let transport = StreamableHttpClientTransport::from_config(config);

        // Serving the client handler automatically triggers the standard JSON-RPC 2.0 handshake under the hood:
        // 1. Sends InitializeRequest
        // 2. Expects InitializeResult
        // 3. Sends InitializedNotification
        let handler = McpClientHandler;
        let service = handler.serve(transport).await?;
        let peer = service.peer().clone();

        Ok(Self { peer, service })
    }

    /// Call a tool on the connected MCP server with a JSON arguments map.
    pub async fn call_tool(
        &self,
        name: String,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> anyhow::Result<rmcp::model::CallToolResult> {
        let request = rmcp::model::CallToolRequestParams::new(name).with_arguments(arguments);
        let result = self.peer.call_tool(request).await?;
        Ok(result)
    }

    /// Retrieve all tools from the connected MCP server (discovery).
    pub async fn list_tools(&self) -> anyhow::Result<Vec<Tool>> {
        // `list_all_tools` automatically manages cursors and paginated results under the hood
        let tools = self.peer.list_all_tools().await?;
        Ok(tools)
    }

    /// Gracefully shut down the client, awaiting the background service task.
    ///
    /// Must be called explicitly to avoid leaving the Tokio runtime blocked on
    /// the rmcp session-deletion cleanup (which has a 5-second internal timeout).
    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.service
            .cancel()
            .await
            .map_err(|e| anyhow::anyhow!("MCP client shutdown join error: {e}"))?;
        Ok(())
    }
}

/// Resolve the first available server from the server map.
///
/// Prefers "cast" if present, otherwise takes the first entry.
/// Returns an error if the map is empty.
fn pick_server(
    server_map: &HashMap<String, config::RemoteServerConfig>,
) -> anyhow::Result<config::RemoteServerConfig> {
    if let Some(s) = server_map.get("cast") {
        return Ok(s.clone());
    }
    server_map
        .values()
        .next()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No servers configured. Pass --cast-mcp-url or add a server to cast-mcp-client.json."))
}

pub async fn list_tools_cmd(
    server_map: HashMap<String, config::RemoteServerConfig>,
    server_filter: Option<String>,
) -> anyhow::Result<()> {
    // If a --server filter was given, validate it exists in the map first.
    if let Some(ref name) = server_filter
        && !server_map.contains_key(name.as_str()) {
            anyhow::bail!(
                "Unknown server '{}'. Check your cast-mcp-client.json or run without --server to list all.",
                name
            );
        }

    // Build the set of servers to query (all, or just the filtered one).
    let targets: Vec<(String, config::RemoteServerConfig)> = match server_filter {
        Some(ref name) => {
            let server = server_map[name.as_str()].clone();
            vec![(name.clone(), server)]
        }
        None => server_map.into_iter().collect(),
    };

    if targets.is_empty() {
        println!("[]");
        return Ok(());
    }

    // Query all target servers concurrently.
    let futures: Vec<_> = targets
        .into_iter()
        .map(|(name, server)| async move {
            let client = McpClient::connect(&server).await?;
            let tools = client.list_tools().await?;
            client.shutdown().await?;
            // Prefix each tool name with "server_name/"
            let prefixed: Vec<Tool> = tools
                .into_iter()
                .map(|mut t| {
                    t.name = format!("{}/{}", name, t.name).into();
                    t
                })
                .collect();
            Ok::<Vec<Tool>, anyhow::Error>(prefixed)
        })
        .collect();

    let results = futures::future::join_all(futures).await;

    let mut all_tools: Vec<Tool> = Vec::new();
    for result in results {
        match result {
            Ok(tools) => all_tools.extend(tools),
            Err(e) => eprintln!("Warning: {}", e),
        }
    }

    println!("{}", serde_json::to_string_pretty(&all_tools)?);
    Ok(())
}

pub async fn describe_tool_cmd(
    tool_name: String,
    server_map: HashMap<String, config::RemoteServerConfig>,
) -> anyhow::Result<()> {
    let server = pick_server(&server_map)?;
    let mcp_client = McpClient::connect(&server).await?;
    let tools = mcp_client.list_tools().await?;

    let tool = tools
        .into_iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown tool '{}'. Run 'cast-mcp-client list' to see available tools.",
                tool_name
            )
        })?;

    println!("{}", serde_json::to_string_pretty(&tool)?);
    mcp_client.shutdown().await
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

pub async fn call_tool_cmd(
    tool_name: String,
    params: Option<String>,
    server_map: HashMap<String, config::RemoteServerConfig>,
) -> anyhow::Result<()> {
    let arguments = read_params(params)?;

    let server = pick_server(&server_map)?;
    let mcp_client = McpClient::connect(&server).await?;
    let result = mcp_client.call_tool(tool_name.clone(), arguments).await?;

    println!("{}", serde_json::to_string_pretty(&result)?);

    mcp_client.shutdown().await?;

    Ok(())
}

/// Read JSON parameters from an inline string, explicit stdin (`-`), piped stdin, or default to `{}`.
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

    value.as_object().cloned().ok_or_else(|| {
        anyhow::anyhow!("Parameters must be a JSON object, e.g. '{{\"key\": \"val\"}}'")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_resolve_explicit_url() {
        let url = Some("http://example.com/mcp".to_string());
        assert_eq!(resolve_server_url(url), "http://example.com/mcp");
    }

    #[test]
    fn test_resolve_env_url() {
        unsafe {
            env::set_var("CAST_MCP_URL", "http://env.com/mcp");
        }
        let result = resolve_server_url(None);
        unsafe {
            env::remove_var("CAST_MCP_URL");
        }
        assert_eq!(result, "http://env.com/mcp");
    }

    // --- resolve_cast_mcp_url ---

    #[test]
    fn test_resolve_cast_mcp_url_flag_wins_over_env_and_config() {
        let config = config::parse_from_str(r#"{"mcp":{"cast":{"url":"http://config.com/mcp"}}}"#);
        let result = resolve_cast_mcp_url(
            Some("http://flag.com/mcp".to_string()),
            Some("http://env.com/mcp".to_string()),
            &config,
        );
        assert_eq!(result, Some("http://flag.com/mcp".to_string()));
    }

    #[test]
    fn test_resolve_cast_mcp_url_env_wins_over_config() {
        let config = config::parse_from_str(r#"{"mcp":{"cast":{"url":"http://config.com/mcp"}}}"#);
        let result = resolve_cast_mcp_url(None, Some("http://env.com/mcp".to_string()), &config);
        assert_eq!(result, Some("http://env.com/mcp".to_string()));
    }

    #[test]
    fn test_resolve_cast_mcp_url_config_is_fallback() {
        let config = config::parse_from_str(r#"{"mcp":{"cast":{"url":"http://config.com/mcp"}}}"#);
        let result = resolve_cast_mcp_url(None, None, &config);
        assert_eq!(result, Some("http://config.com/mcp".to_string()));
    }

    #[test]
    fn test_resolve_cast_mcp_url_returns_none_when_no_source() {
        let config = config::ClientConfig::default();
        let result = resolve_cast_mcp_url(None, None, &config);
        assert_eq!(result, None);
    }

    // --- build_server_map ---

    #[test]
    fn test_build_server_map_includes_enabled_servers() {
        let config = config::parse_from_str(
            r#"{"mcp":{"sentry":{"url":"http://sentry.com/mcp"},"ctx7":{"url":"http://ctx7.com/mcp"}}}"#,
        );
        let map = build_server_map(None, &config);
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("sentry"));
        assert!(map.contains_key("ctx7"));
    }

    #[test]
    fn test_build_server_map_excludes_disabled_servers() {
        let config = config::parse_from_str(
            r#"{"mcp":{"sentry":{"url":"http://sentry.com/mcp"},"ctx7":{"url":"http://ctx7.com/mcp","enabled":false}}}"#,
        );
        let map = build_server_map(None, &config);
        assert_eq!(map.len(), 1);
        assert!(map.contains_key("sentry"));
        assert!(!map.contains_key("ctx7"));
    }

    #[test]
    fn test_build_server_map_injects_bare_cast_entry_when_url_from_flag_or_env() {
        // Config has a "cast" entry with a header — flag/env URL must override it (no headers)
        let config = config::parse_from_str(
            r#"{"mcp":{"cast":{"url":"http://config.com/mcp","headers":{"X-Token":"secret"}}}}"#,
        );
        let map = build_server_map(Some("http://flag.com/mcp".to_string()), &config);
        let cast = map.get("cast").expect("cast entry should be present");
        assert_eq!(cast.url, "http://flag.com/mcp");
        assert!(
            cast.headers.is_empty(),
            "headers must be stripped when URL comes from flag/env"
        );
    }

    #[test]
    fn test_build_server_map_preserves_full_cast_entry_when_url_from_config() {
        // No explicit URL provided — config entry (including headers) should be used as-is
        let config = config::parse_from_str(
            r#"{"mcp":{"cast":{"url":"http://config.com/mcp","headers":{"X-Token":"secret"}}}}"#,
        );
        let map = build_server_map(None, &config);
        let cast = map.get("cast").expect("cast entry should be present");
        assert_eq!(cast.url, "http://config.com/mcp");
        assert_eq!(
            cast.headers.get("X-Token").map(String::as_str),
            Some("secret")
        );
    }

    #[test]
    fn test_build_server_map_empty_when_no_servers() {
        let config = config::ClientConfig::default();
        let map = build_server_map(None, &config);
        assert!(map.is_empty());
    }
}
