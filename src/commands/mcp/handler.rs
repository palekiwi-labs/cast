use crate::commands::mcp::exec;
use crate::config::{McpConfig, McpToolConfig};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

/// A dynamic MCP handler that serves tools defined in `cast.json` at runtime.
///
/// Implements [`ServerHandler`] manually (bypassing `#[tool_router]`) so that tools
/// can be loaded from config rather than being known at compile time.
///
/// JSON Schema validators are pre-compiled at construction time so that each
/// `call_tool` invocation only performs validation, never compilation.
#[derive(Clone, Debug)]
pub struct McpHandler {
    inner: Arc<McpHandlerInner>,
}

#[derive(Debug)]
struct McpHandlerInner {
    config: McpConfig,
    host_env: HashMap<String, String>,
    /// Pre-compiled validators, keyed by tool name.
    validators: HashMap<String, jsonschema::Validator>,
    /// Pre-computed tools for list_tools responses.
    cached_tools: Vec<Tool>,
}

impl McpHandler {
    pub fn new(config: McpConfig, host_env: HashMap<String, String>) -> anyhow::Result<Self> {
        let mut validators = HashMap::new();
        let mut cached_tools = Vec::new();

        for (name, tool) in &config.tools {
            let validator = jsonschema::validator_for(&tool.parameters)
                .map_err(|e| anyhow::anyhow!("Invalid JSON schema for tool '{}': {}", name, e))?;
            validators.insert(name.clone(), validator);
            cached_tools.push(tool_config_to_rmcp_tool(name, tool));
        }

        Ok(Self {
            inner: Arc::new(McpHandlerInner {
                config,
                host_env,
                validators,
                cached_tools,
            }),
        })
    }

    /// Core tool-execution pipeline, decoupled from the MCP transport layer.
    ///
    /// Extracted from [`ServerHandler::call_tool`] so that integration tests can
    /// drive the full request → validation → exec → response path without
    /// constructing an `rmcp` `RequestContext`.
    pub(crate) async fn execute_tool(
        &self,
        request: CallToolRequestParams,
    ) -> Result<CallToolResult, McpError> {
        info!(tool = %request.name, "call_tool requested");

        // 1. Look up the tool in config
        let tool_config = self.inner.config.tools.get(&*request.name).ok_or_else(|| {
            warn!(tool = %request.name, "unknown tool requested");
            McpError::invalid_params(format!("Unknown tool: '{}'", request.name), None)
        })?;

        // 2. Extract arguments as a JSON Value
        let args_map = request.arguments.unwrap_or_default();
        let args_value = Value::Object(args_map);

        // 3. Retrieve pre-compiled validator (compiled once in McpHandler::new)
        let validator = self.inner.validators.get(&*request.name).expect(
            "validator map is always in sync with config.tools (enforced by fail-fast McpHandler::new)",
        );

        let validation_errors: Vec<String> = validator
            .iter_errors(&args_value)
            .map(|e| e.to_string())
            .collect();

        if !validation_errors.is_empty() {
            warn!(tool = %request.name, errors = ?validation_errors, "argument validation failed");
            return Err(McpError::invalid_params(
                format!("Invalid arguments: {}", validation_errors.join("; ")),
                None,
            ));
        }

        // 4. Map argument templates to concrete CLI arguments
        let mapped_args = exec::map_args(&tool_config.args, &args_value).map_err(|e| {
            error!(tool = %request.name, err = %e, "argument mapping failed");
            McpError::internal_error(format!("Argument mapping error: {}", e), None)
        })?;

        // 5. Execute the command via the secure execution engine
        let exec_result = exec::run_command(tool_config, mapped_args, &self.inner.host_env)
            .await
            .map_err(|e| {
                error!(tool = %request.name, err = %e, "command execution failed");
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

/// Converts a [`McpToolConfig`] into an [`rmcp::model::Tool`] for `list_tools` responses.
///
/// If `parameters` is not a JSON object, an empty schema map is used as a safe fallback.
pub fn tool_config_to_rmcp_tool(name: &str, config: &McpToolConfig) -> Tool {
    let schema = match &config.parameters {
        Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };
    Tool::new_with_raw(
        name.to_string(),
        Some(config.description.clone().into()),
        schema,
    )
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
        Ok(ListToolsResult {
            tools: self.inner.cached_tools.clone(),
            next_cursor: None,
            meta: Default::default(),
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        self.execute_tool(request).await
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
            port: 8080,
            hostname: "localhost".to_string(),
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
        assert!(
            !errors.is_empty(),
            "missing required field should produce errors"
        );
    }

    #[test]
    fn test_wrong_type_fails_schema_validation() {
        let config = echo_tool_config();
        let args = json!({ "message": 42 });
        let validator = jsonschema::validator_for(&config.parameters).unwrap();
        let errors: Vec<_> = validator.iter_errors(&args).collect();
        assert!(
            !errors.is_empty(),
            "wrong type should produce a validation error"
        );
    }

    // --- execute_tool pipeline integration tests ---
    //
    // These tests drive the full validate → map → exec → respond path by calling
    // execute_tool directly, avoiding the need to construct an rmcp RequestContext.

    fn make_handler(tools: BTreeMap<String, McpToolConfig>) -> McpHandler {
        let config = McpConfig {
            port: 8080,
            hostname: "localhost".to_string(),
            tools,
        };
        let mut host_env = HashMap::new();
        if let Ok(path) = std::env::var("PATH") {
            host_env.insert("PATH".to_string(), path);
        }
        McpHandler::new(config, host_env).expect("failed to create McpHandler in test")
    }

    #[tokio::test]
    async fn test_pipeline_success_returns_output() {
        let mut tools = BTreeMap::new();
        tools.insert("echo".to_string(), echo_tool_config());
        let handler = make_handler(tools);

        let args = json!({ "message": "hello" }).as_object().unwrap().clone();
        let request = CallToolRequestParams::new("echo").with_arguments(args);

        let result = handler.execute_tool(request).await.expect("should succeed");
        assert_ne!(result.content.len(), 0, "response should contain content");
        assert!(
            result.is_error.unwrap_or(false) == false,
            "successful exec should not be flagged as error"
        );
    }

    #[tokio::test]
    async fn test_pipeline_unknown_tool_returns_invalid_params() {
        let handler = make_handler(BTreeMap::new());

        let request = CallToolRequestParams::new("no_such_tool");
        let err = handler
            .execute_tool(request)
            .await
            .expect_err("should fail");

        // -32602 is the JSON-RPC code for InvalidParams
        assert_eq!(
            err.code.0, -32602,
            "unknown tool should yield InvalidParams (-32602)"
        );
    }

    #[test]
    fn test_new_with_invalid_schema_fails_fast() {
        let mut tools = BTreeMap::new();
        let mut config = echo_tool_config();
        // Set an invalid JSON Schema (type should be a string or array of strings)
        config.parameters = json!({
            "type": 123
        });
        tools.insert("broken".to_string(), config);

        let mcp_config = McpConfig {
            port: 8080,
            hostname: "localhost".to_string(),
            tools,
        };
        let res = McpHandler::new(mcp_config, HashMap::new());
        assert!(
            res.is_err(),
            "McpHandler::new should fail with invalid schema"
        );
        let err = res.unwrap_err().to_string();
        assert!(err.contains("Invalid JSON schema for tool 'broken'"));
    }

    #[tokio::test]
    async fn test_pipeline_invalid_args_returns_invalid_params() {
        let mut tools = BTreeMap::new();
        tools.insert("echo".to_string(), echo_tool_config());
        let handler = make_handler(tools);

        // Omit the required `message` field to trigger schema validation failure
        let request = CallToolRequestParams::new("echo");
        let err = handler
            .execute_tool(request)
            .await
            .expect_err("should fail");

        // -32602 is the JSON-RPC code for InvalidParams
        assert_eq!(
            err.code.0, -32602,
            "schema violation should yield InvalidParams (-32602)"
        );
    }
}
