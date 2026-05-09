use crate::commands::mcp::exec;
use crate::config::{McpConfig, McpToolConfig};
use rmcp::{
    model::{
        CallToolRequestMethod, CallToolRequestParams, CallToolResult, Content, Implementation,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
    ErrorData as McpError, RoleServer, ServerHandler,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// A dynamic MCP handler that serves tools defined in `cast.json` at runtime.
///
/// Implements [`ServerHandler`] manually (bypassing `#[tool_router]`) so that tools
/// can be loaded from config rather than being known at compile time.
#[derive(Clone)]
pub struct McpHandler {
    config: Arc<McpConfig>,
    host_env: Arc<HashMap<String, String>>,
}

impl McpHandler {
    pub fn new(config: McpConfig, host_env: HashMap<String, String>) -> Self {
        Self {
            config: Arc::new(config),
            host_env: Arc::new(host_env),
        }
    }
}

/// Converts a [`McpToolConfig`] into an [`rmcp::model::Tool`] for `list_tools` responses.
///
/// If `parameters` is not a JSON object, an empty schema map is used as a safe fallback.
pub fn tool_config_to_rmcp_tool(name: &str, config: &McpToolConfig) -> Tool {
    let schema = match &config.parameters {
        Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };
    Tool::new_with_raw(name.to_string(), Some(config.description.clone().into()), schema)
}

impl ServerHandler for McpHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("cast-mcp", env!("CARGO_PKG_VERSION")))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = self
            .config
            .tools
            .iter()
            .map(|(name, config)| tool_config_to_rmcp_tool(name, config))
            .collect();

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: Default::default(),
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // 1. Look up the tool in config
        let tool_config = self
            .config
            .tools
            .get(&*request.name)
            .ok_or_else(McpError::method_not_found::<CallToolRequestMethod>)?;

        // 2. Extract arguments as a JSON Value
        let args_map = request.arguments.unwrap_or_default();
        let args_value = Value::Object(args_map);

        // 3. Compile and validate against the tool's JSON schema
        let validator = jsonschema::validator_for(&tool_config.parameters).map_err(|e| {
            McpError::internal_error(
                format!("Invalid schema for tool '{}': {}", request.name, e),
                None,
            )
        })?;

        let validation_errors: Vec<String> = validator
            .iter_errors(&args_value)
            .map(|e| e.to_string())
            .collect();

        if !validation_errors.is_empty() {
            return Err(McpError::invalid_request(
                format!("Invalid arguments: {}", validation_errors.join("; ")),
                None,
            ));
        }

        // 4. Map argument templates to concrete CLI arguments
        let mapped_args = exec::map_args(&tool_config.args, &args_value).map_err(|e| {
            McpError::internal_error(format!("Argument mapping error: {}", e), None)
        })?;

        // 5. Execute the command via the secure execution engine
        let exec_result = exec::run_command(tool_config, mapped_args, &self.host_env)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Command execution error: {}", e), None)
            })?;

        // 6. Convert to MCP response, preserving the subprocess error flag
        let content: Vec<Content> = exec_result
            .content
            .into_iter()
            .map(|c| Content::text(c.text))
            .collect();

        if exec_result.is_error {
            // CallToolResult is #[non_exhaustive]; build via success() then override is_error
            let mut result = CallToolResult::success(content);
            result.is_error = Some(true);
            Ok(result)
        } else {
            Ok(CallToolResult::success(content))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ArgTemplate, McpConfig};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn echo_tool_config() -> McpToolConfig {
        McpToolConfig {
            description: "Echo a message to stdout".to_string(),
            command: "echo".to_string(),
            args: vec![ArgTemplate::Literal("{message}".to_string())],
            env: None,
            working_dir: None,
            parameters: json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                },
                "required": ["message"]
            }),
        }
    }

    // --- tool_config_to_rmcp_tool unit tests ---

    #[test]
    fn test_conversion_sets_name() {
        let tool = tool_config_to_rmcp_tool("echo", &echo_tool_config());
        assert_eq!(tool.name, "echo");
    }

    #[test]
    fn test_conversion_sets_description() {
        let tool = tool_config_to_rmcp_tool("echo", &echo_tool_config());
        assert_eq!(
            tool.description.as_deref(),
            Some("Echo a message to stdout")
        );
    }

    #[test]
    fn test_conversion_passes_schema_through() {
        let tool = tool_config_to_rmcp_tool("echo", &echo_tool_config());
        let schema = tool.input_schema.as_ref();
        assert!(schema.contains_key("type"), "schema should have 'type' key");
        assert!(
            schema.contains_key("properties"),
            "schema should have 'properties' key"
        );
    }

    #[test]
    fn test_conversion_non_object_parameters_falls_back_to_empty_schema() {
        let config = McpToolConfig {
            description: "No schema".to_string(),
            command: "true".to_string(),
            args: vec![],
            env: None,
            working_dir: None,
            parameters: json!(null),
        };
        let tool = tool_config_to_rmcp_tool("noop", &config);
        assert!(
            tool.input_schema.is_empty(),
            "non-object parameters should yield an empty schema"
        );
    }

    #[test]
    fn test_list_produces_one_tool_per_config_entry() {
        let mut tools = BTreeMap::new();
        tools.insert("tool_a".to_string(), echo_tool_config());
        tools.insert("tool_b".to_string(), echo_tool_config());
        let config = McpConfig {
            port: None,
            hostname: None,
            tools,
        };

        let rmcp_tools: Vec<Tool> = config
            .tools
            .iter()
            .map(|(n, c)| tool_config_to_rmcp_tool(n, c))
            .collect();

        assert_eq!(rmcp_tools.len(), 2);
        let names: Vec<&str> = rmcp_tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"tool_a"));
        assert!(names.contains(&"tool_b"));
    }

    // --- jsonschema validation unit tests ---

    #[test]
    fn test_valid_args_pass_schema_validation() {
        let config = echo_tool_config();
        let args = json!({ "message": "hello" });
        let validator = jsonschema::validator_for(&config.parameters).unwrap();
        assert!(validator.is_valid(&args));
    }

    #[test]
    fn test_missing_required_arg_fails_schema_validation() {
        let config = echo_tool_config();
        let args = json!({});
        let validator = jsonschema::validator_for(&config.parameters).unwrap();
        let errors: Vec<_> = validator.iter_errors(&args).collect();
        assert!(!errors.is_empty(), "missing required field should produce errors");
    }

    #[test]
    fn test_wrong_type_fails_schema_validation() {
        let config = echo_tool_config();
        let args = json!({ "message": 42 });
        let validator = jsonschema::validator_for(&config.parameters).unwrap();
        let errors: Vec<_> = validator.iter_errors(&args).collect();
        assert!(!errors.is_empty(), "wrong type should produce a validation error");
    }

    // NOTE: Full end-to-end transport tests (list_tools / call_tool via
    // tokio::io::duplex + serve_directly) are deferred to Slice 5, where the
    // HTTP server layer is in place and a proper rmcp client can be wired up.
    // The unit tests above cover all business logic that does not require the
    // MCP JSON-RPC transport.
}
