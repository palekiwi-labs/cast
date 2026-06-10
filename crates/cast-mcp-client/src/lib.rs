use http::{HeaderName, HeaderValue};
use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, Tool};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{ClientHandler, Peer, RoleClient, ServiceExt};
use std::collections::HashMap;
use std::str::FromStr;

pub mod config;

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

pub async fn list_tools_cmd(
    server_map: HashMap<String, config::RemoteServerConfig>,
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
    let targets: Vec<(String, config::RemoteServerConfig)> = if servers.is_empty() {
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
    // Each future resolves to (server_name, Result<Vec<Tool>>) so errors can be
    // attributed to the specific server that failed.
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

pub async fn describe_tool_cmd(
    server_name: String,
    tool_name: String,
    server_map: HashMap<String, config::RemoteServerConfig>,
) -> anyhow::Result<()> {
    let server = server_map.get(server_name.as_str()).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown server '{}'. Check your cast-mcp-client.json or pass --cast-mcp-url.",
            server_name
        )
    })?;

    let mcp_client = McpClient::connect(server).await?;
    let tools = mcp_client.list_tools().await?;

    let tool = tools
        .into_iter()
        .find(|t| t.name == tool_name.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown tool '{}' on server '{}'. Run 'cast-mcp-client list' to see available tools.",
                tool_name,
                server_name
            )
        })?;

    println!("{}", serde_json::to_string_pretty(&tool)?);
    mcp_client.shutdown().await
}

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
    server_map: HashMap<String, config::RemoteServerConfig>,
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

// ---------------------------------------------------------------------------
// generate command helpers
// ---------------------------------------------------------------------------

/// Return the current time as a Unix timestamp (seconds since epoch).
fn unix_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Metadata extracted from a single JSON Schema property.
struct ParamSpec {
    /// Original JSON property name (used as JSON key and jq variable name).
    name: String,
    /// Kebab-case version used for the bash `--flag` name.
    flag: String,
    /// ALL_CAPS bash variable name.
    var: String,
    /// Uppercased type string shown in `--help` (e.g. "STRING", "INTEGER").
    type_hint: String,
    /// Whether to use `--argjson` (true) or `--arg` (false) in jq.
    json_arg: bool,
    /// Description from the schema, or empty string.
    description: String,
    /// Whether this parameter appears in `required`.
    required: bool,
}

/// Extract `ParamSpec` list from a `Tool`'s `inputSchema`.
///
/// Uses serialization to access the schema without depending on rmcp's private fields.
fn parse_params(tool: &Tool) -> Vec<ParamSpec> {
    let tool_val = serde_json::to_value(tool).unwrap_or_default();
    let schema = match tool_val.get("inputSchema").and_then(|v| v.as_object()) {
        Some(s) => s.clone(),
        None => return vec![],
    };
    let properties = match schema.get("properties").and_then(|v| v.as_object()) {
        Some(p) => p.clone(),
        None => return vec![],
    };
    let required_set: std::collections::HashSet<String> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

    let mut params: Vec<ParamSpec> = properties
        .iter()
        .map(|(name, prop)| {
            let ty = prop
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("string");
            let description = prop
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let (type_hint, json_arg) = match ty {
                "integer" | "number" => ("INTEGER", true),
                "boolean" => ("BOOLEAN", true),
                "array" => ("JSON_ARRAY", true),
                "object" => ("JSON_OBJECT", true),
                _ => ("STRING", false),
            };
            ParamSpec {
                flag: camel_to_kebab(name),
                var: camel_to_kebab(name).replace('-', "_").to_uppercase(),
                name: name.clone(),
                type_hint: type_hint.to_string(),
                json_arg,
                description,
                required: required_set.contains(name.as_str()),
            }
        })
        .collect();

    // Required params first, then optional, both groups alphabetical for stability.
    params.sort_by(|a, b| b.required.cmp(&a.required).then(a.flag.cmp(&b.flag)));
    params
}

/// Generate a bash script wrapper for a single MCP tool.
///
/// The script:
/// - Parses named `--flag value` CLI arguments derived from `inputSchema`.
/// - Validates required parameters before calling the server.
/// - Builds a JSON payload via `jq`.
/// - Calls `cast-mcp-client call <server> <tool> <json>`.
/// - Parses MCP output: text to stdout, `isError` to stderr (exit 1), non-text warned.
pub fn generate_script(server_name: &str, tool: &Tool) -> String {
    let tool_name = tool.name.as_ref();
    let description = tool.description.as_deref().unwrap_or("").trim().to_string();
    let script_name = format!("{}-{}", server_name, camel_to_kebab(tool_name));
    let params = parse_params(tool);

    let mut s = String::new();

    // ── Header ──────────────────────────────────────────────────────────────
    s.push_str("#!/usr/bin/env bash\n");
    // Write the description as a commented block so that multi-line descriptions
    // (common in real MCP servers) don't leak unquoted prose into the script.
    s.push_str(&format!("# {}:", script_name));
    for line in description.lines() {
        s.push_str(&format!("\n# {}", line));
    }
    s.push('\n');
    s.push_str("# Generated by cast-mcp-client generate\n");
    s.push_str(&format!(
        "# Server: {} | Tool: {}\n\n",
        server_name, tool_name
    ));
    s.push_str("set -euo pipefail\n\n");
    s.push_str(&format!("SERVER=\"{}\"\n", server_name));
    s.push_str(&format!("TOOL=\"{}\"\n\n", tool_name));

    // ── Usage function ───────────────────────────────────────────────────────
    s.push_str("usage() { cat <<'EOF'\n");
    s.push_str(&format!("Usage: {} [OPTIONS]\n", script_name));
    if !description.is_empty() {
        s.push_str(&format!("{}\n", description));
    }
    s.push('\n');
    s.push_str("Options:\n");
    for p in &params {
        let req_label = if p.required {
            "(required)"
        } else {
            "(optional)"
        };
        s.push_str(&format!(
            "  --{} {}    {} {}\n",
            p.flag, p.type_hint, req_label, p.description
        ));
    }
    s.push_str("  -h, --help          Show this help\n");
    s.push_str("EOF\n}\n\n");

    // ── Variable declarations ────────────────────────────────────────────────
    for p in &params {
        s.push_str(&format!("{}=\"\"\n", p.var));
    }
    s.push('\n');

    // ── Argument parsing loop ────────────────────────────────────────────────
    s.push_str("while [[ $# -gt 0 ]]; do\n");
    s.push_str("  case \"$1\" in\n");
    for p in &params {
        s.push_str(&format!("    --{}) {}=\"$2\"; shift 2 ;;\n", p.flag, p.var));
    }
    s.push_str("    -h|--help) usage; exit 0 ;;\n");
    s.push_str("    *) echo \"Unknown option: $1\" >&2; usage >&2; exit 1 ;;\n");
    s.push_str("  esac\n");
    s.push_str("done\n\n");

    // ── Required param validation ────────────────────────────────────────────
    for p in params.iter().filter(|p| p.required) {
        s.push_str(&format!(
            "[[ -z \"${{{}:-}}\" ]] && {{ echo \"Error: --{} is required\" >&2; exit 1; }}\n",
            p.var, p.flag
        ));
    }
    if params.iter().any(|p| p.required) {
        s.push('\n');
    }

    // ── JSON payload construction ────────────────────────────────────────────
    s.push_str("PARAMS='{}'\n");
    for p in &params {
        let jq_flag = if p.json_arg { "--argjson" } else { "--arg" };
        if p.required {
            // Always included — validation already guarantees non-empty.
            s.push_str(&format!(
                "PARAMS=$(echo \"$PARAMS\" | jq {} {} \"${{{}}}\" '. + {{\"{}\" : ${}}}')\n",
                jq_flag, p.name, p.var, p.name, p.name
            ));
        } else {
            // Conditionally included only when the user provided a value.
            s.push_str(&format!(
                "[[ -n \"${{{}:-}}\" ]] && PARAMS=$(echo \"$PARAMS\" | jq {} {} \"${{{}}}\" '. + {{\"{}\" : ${}}}')\n",
                p.var, jq_flag, p.name, p.var, p.name, p.name
            ));
        }
    }
    s.push('\n');

    // ── Call cast-mcp-client ─────────────────────────────────────────────────
    s.push_str("RESULT=$(cast-mcp-client call \"$SERVER\" \"$TOOL\" \"$PARAMS\"); STATUS=$?\n");
    s.push_str("[[ $STATUS -ne 0 ]] && { echo \"$RESULT\" >&2; exit $STATUS; }\n\n");

    // ── MCP output parsing ───────────────────────────────────────────────────
    s.push_str("IS_ERROR=$(echo \"$RESULT\" | jq -r '.isError // false')\n");
    s.push_str("[[ \"$IS_ERROR\" == \"true\" ]] && {\n");
    s.push_str("  echo \"$RESULT\" | jq -r '.content[]|select(.type==\"text\")|.text' >&2\n");
    s.push_str("  exit 1\n");
    s.push_str("}\n\n");
    s.push_str("NON_TEXT=$(echo \"$RESULT\" | jq -r '[.content[]|select(.type!=\"text\")|.type]|unique|join(\", \")')\n");
    s.push_str(
        "[[ -n \"$NON_TEXT\" ]] && echo \"Warning: ignored non-text type(s): $NON_TEXT\" >&2\n\n",
    );
    s.push_str("echo \"$RESULT\" | jq -r '[.content[]|select(.type==\"text\")|.text]|join(\"\")'");

    s
}

/// Convert a camelCase or snake_case identifier to kebab-case.
///
/// Rules (no regex):
/// - Insert `-` before an uppercase letter that follows a lowercase/digit.
/// - Insert `-` before an uppercase letter that follows another uppercase AND is
///   itself followed by a lowercase (handles acronyms like "APIKey" → "api-key").
/// - Replace `_` with `-`.
/// - Lowercase the result.
///
/// Examples: `projectSlug` → `project-slug`, `APIKey` → `api-key`,
///           `myAPIKey` → `my-api-key`, `HTMLParser` → `html-parser`.
pub(crate) fn camel_to_kebab(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + 4);
    for (i, &c) in chars.iter().enumerate() {
        if c == '_' {
            out.push('-');
        } else if c.is_uppercase() {
            let prev_lower =
                i > 0 && (chars[i - 1].is_lowercase() || chars[i - 1].is_ascii_digit());
            let prev_upper = i > 0 && chars[i - 1].is_uppercase();
            let next_lower = chars.get(i + 1).is_some_and(|nc| nc.is_lowercase());
            if prev_lower || (prev_upper && next_lower) {
                out.push('-');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Generate bash script wrappers for every tool on the configured servers.
///
/// - Queries all (or filtered) servers concurrently to discover their tools.
/// - Writes one executable `.sh` script per tool into `output_dir` (created if absent).
/// - Prints a JSON envelope to stdout listing every generated script.
///
/// Output schema:
/// ```json
/// { "output_dir": "/abs/path", "scripts": [{ "server", "tool", "path" }] }
/// ```
pub async fn generate_scripts_cmd(
    server_filter: Vec<String>,
    output_dir: &std::path::Path,
    server_map: HashMap<String, config::RemoteServerConfig>,
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
    let targets: Vec<(String, config::RemoteServerConfig)> = if server_filter.is_empty() {
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

        // Retrieve the server URL for the manifest (empty string if somehow absent).
        let server_url = targets_with_url
            .get(&server_name)
            .cloned()
            .unwrap_or_default();

        let mut manifest_tools: serde_json::Map<String, serde_json::Value> =
            serde_json::Map::new();

        for tool in &tools {
            let script_content = generate_script(&server_name, tool);
            let filename = format!("{}-{}.sh", server_name, camel_to_kebab(tool.name.as_ref()));
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
    let generated_at = unix_now();
    let manifest = serde_json::json!({
        "generated_at": generated_at,
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
    server_name: String,
    tool_name: String,
    params: Option<String>,
    server_map: HashMap<String, config::RemoteServerConfig>,
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

    // --- camel_to_kebab ---

    #[test]
    fn test_camel_to_kebab() {
        assert_eq!(camel_to_kebab("projectSlug"), "project-slug");
        assert_eq!(camel_to_kebab("APIKey"), "api-key");
        assert_eq!(camel_to_kebab("myAPIKey"), "my-api-key");
        assert_eq!(
            camel_to_kebab("fetch_cast_documentation"),
            "fetch-cast-documentation"
        );
        assert_eq!(camel_to_kebab("message"), "message");
        assert_eq!(camel_to_kebab("already-kebab"), "already-kebab");
        assert_eq!(camel_to_kebab("HTMLParser"), "html-parser");
    }

    // --- generate_script ---

    #[test]
    fn test_generate_script_content() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "message": { "type": "string",  "description": "The message to send" },
                "count":   { "type": "integer", "description": "Number of repeats" }
            },
            "required": ["message"]
        });
        let tool = rmcp::model::Tool::new_with_raw(
            "dummy_tool".to_string(),
            Some("A mock tool".into()),
            schema.as_object().cloned().unwrap_or_default(),
        );
        let script = generate_script("cast", &tool);

        // Shebang + header
        assert!(script.starts_with("#!/usr/bin/env bash"), "missing shebang");
        assert!(script.contains("cast-dummy-tool"), "script name in header");
        assert!(script.contains("A mock tool"), "description in header");

        // Flags in usage
        assert!(script.contains("--message"), "required flag --message");
        assert!(script.contains("--count"), "optional flag --count");
        assert!(script.contains("(required)"), "required marker");
        assert!(script.contains("(optional)"), "optional marker");
        assert!(script.contains("STRING"), "string type hint");
        assert!(script.contains("INTEGER"), "integer type hint");

        // Validation for required param
        assert!(
            script.contains("--message is required"),
            "required validation message"
        );

        // jq type handling: string uses --arg, integer uses --argjson
        assert!(script.contains("--arg message"), "string param uses --arg");
        assert!(
            script.contains("--argjson count"),
            "integer param uses --argjson"
        );

        // SERVER/TOOL constants
        assert!(script.contains("SERVER=\"cast\""), "SERVER constant");
        assert!(script.contains("TOOL=\"dummy_tool\""), "TOOL constant");

        // Output parsing section
        assert!(script.contains("isError"), "isError check");
        assert!(
            script.contains("cast-mcp-client call"),
            "delegates to cast-mcp-client call"
        );
    }

    #[test]
    fn test_generate_script_multiline_description() {
        // Descriptions from real MCP servers (e.g. Context7) are multi-line prose.
        // Every line that falls outside the heredoc must be prefixed with '#' so
        // bash does not attempt to execute prose as commands.
        let multiline_desc = "First line summary.\n\nYou MUST call this first.\n\nDo not call more than 3 times.";
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "The query" }
            },
            "required": ["query"]
        });
        let tool = rmcp::model::Tool::new_with_raw(
            "search".to_string(),
            Some(multiline_desc.into()),
            schema.as_object().cloned().unwrap_or_default(),
        );
        let script = generate_script("myserver", &tool);

        // Every non-blank line before 'set -euo pipefail' must start with '#'.
        // (Heredoc content and the set line itself are exempt — we only check
        // the header block which is where the description is embedded.)
        let header: String = script
            .lines()
            .take_while(|l| !l.starts_with("set -euo pipefail"))
            .collect::<Vec<_>>()
            .join("\n");
        for line in header.lines() {
            if line.is_empty() {
                continue;
            }
            assert!(
                line.starts_with('#'),
                "Header line is not a comment: {:?}",
                line
            );
        }

        // The description text is present somewhere in the script.
        assert!(script.contains("First line summary."), "description in script");
        assert!(script.contains("You MUST call this first."), "second paragraph present");
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
