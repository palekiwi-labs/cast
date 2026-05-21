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
}
