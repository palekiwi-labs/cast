# Research: MCP Configuration Verification

## Research Questions
1. Is `docs/mcp/configuration.md` accurate regarding the `cast.json` schema?
2. Are all argument template features (`{name}`, `{...name}`, `if_present`, `if_true`) implemented as documented?
3. Are there any undocumented configuration options or behaviors?

## Findings

### 1. Schema Accuracy
The documentation correctly identifies the core fields for tool definitions.
- **Source**: `/home/pl/code/palekiwi-labs/cast/src/config/schema.rs`
- **Symbol**: `McpToolConfig`
- **Snippet**:
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

### 2. Argument Template Implementation
The implementation in `src/commands/mcp/exec.rs` confirms the documented behavior for placeholders and conditional blocks.
- **Spread Operator**: `{...name}` correctly expands JSON arrays into multiple arguments.
- **Placeholder Removal**: If a parameter for `{name}` is missing, the placeholder text is removed. If the resulting argument is empty, it is omitted.
- **Source**: `/home/pl/code/palekiwi-labs/cast/src/commands/mcp/exec.rs`
- **Symbol**: `expand_placeholder`
- **Snippet**:
  ```rust
  if template.starts_with("{...") && template.ends_with('}') {
      let key = &template[4..template.len() - 1];
      if let Some(val) = params.get(key) {
          if let Some(arr) = val.as_array() {
              for item in arr {
                  args.push(item.to_string().trim_matches('"').to_string());
              }
              return;
          }
      }
  }
  ```

### 3. Undocumented Behaviors
- **`hostname` Support**: `McpConfig` includes a `hostname` field (defaults to `127.0.0.1`) which is not mentioned in the docs.
- **Implicit Environment**: `PATH` and `TMPDIR` are always inherited for Nix compatibility.
- **Source**: `/home/pl/code/palekiwi-labs/cast/src/commands/mcp/exec.rs`
- **Snippet**:
  ```rust
  // Always inherit PATH and TMPDIR for nix compatibility
  for var in ["PATH", "TMPDIR"] {
      if let Ok(val) = std::env::var(var) {
          cmd.env(var, val);
      }
  }
  ```

## Conclusion
The documentation is **Correct** but **Incomplete**. It should be updated to include the `hostname` configuration, clarify the argument omission logic for missing parameters, and mention the implicit inheritance of `PATH` and `TMPDIR`.
