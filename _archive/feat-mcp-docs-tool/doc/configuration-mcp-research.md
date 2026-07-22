# Research: Dynamic MCP Tool Configuration and Execution

This report documents how dynamic MCP tools are configured and executed in `cast`, based on an analysis of the codebase.

## Research Questions Answered

### 1. How are dynamic tools structured in configuration?
Dynamic tools are defined in the `mcp.tools` section of `cast.json`. The configuration is deserialized into the `McpToolConfig` struct.

**Source**: `src/config/schema.rs`
**Symbol**: `McpToolConfig`
```rust
pub struct McpToolConfig {
    pub description: String,
    pub command: String,
    pub args: Vec<ArgTemplate>,
    #[serde(default)]
    pub env: Option<McpEnvConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    pub parameters: serde_json::Value,
}
```

### 2. How are arguments mapped from MCP parameters to CLI?
The execution engine uses `ArgTemplate` to define how parameters are transformed into CLI arguments.

**Source**: `src/config/schema.rs`
**Symbol**: `ArgTemplate`, `ConditionalBlock`
```rust
pub enum ArgTemplate {
    Literal(String),
    Conditional(ConditionalBlock),
}

pub struct ConditionalBlock {
    pub if_present: Option<String>,
    pub if_true: Option<String>,
    pub args: Vec<String>,
}
```

### 3. How does placeholder expansion work?
The `expand_placeholder` function in `src/commands/mcp/exec.rs` handles parameter injection:
- **Standard `{name}`**: Replaced by the parameter value. If the parameter is missing, it's replaced with an empty string. If the resulting argument is empty and the template was exactly `{name}`, it is omitted.
- **Spread Operator `{...name}`**: Expands an array parameter into multiple CLI arguments.

**Source**: `src/commands/mcp/exec.rs`
**Snippet** (Spread):
```rust
if template.starts_with("{...") && template.ends_with('}') {
    let name = &template[4..template.len() - 1];
    if let Some(arr) = args.get(name).and_then(|v| v.as_array()) {
        return Ok(arr.iter().map(|v| v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string())).collect());
    }
    return Ok(Vec::new());
}
```

### 4. How are conditional blocks evaluated?
`ConditionalBlock` allows including arguments based on the presence or boolean value of a parameter.

**Source**: `src/commands/mcp/exec.rs`
**Snippet**:
```rust
if let Some(key) = &cond.if_present {
    should_include &= args.get(key).is_some() && !args[key].is_null();
}
if let Some(key) = &cond.if_true {
    should_include &= args.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
}
```

## Sourced Findings

### Tool Execution Flow
The `McpHandler` in `src/commands/mcp/handler.rs` routes calls:
1. Checks for built-in tools (`list_cast_documentation`, `fetch_cast_documentation`).
2. Looks up dynamic tools in `self.inner.config.tools`.
3. Validates parameters using the `jsonschema` crate.
4. Maps arguments via `exec::map_args`.
5. Executes the command via `exec::run_command` (using `std::process::Command`).

### Environment and Working Directory
Tools can specify a `working_dir` and `env` (inherit specific vars or set new ones).
**Source**: `src/config/schema.rs`, `McpEnvConfig` struct.

## Confidence Notes
- **High**: The implementation of argument mapping and placeholder expansion is clear and verified.
- **High**: The routing logic in `McpHandler` is consistent with the research.
