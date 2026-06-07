# Research Report: cast-mcp-client API and Output Patterns

## Research Question
Identify the current API and output patterns of the `cast-mcp-client` to provide context for designing a structured JSON output schema.

## Findings

### 1. Current CLI Commands
The `cast-mcp-client` CLI (defined in `crates/cast-mcp-client/src/main.rs`) supports three primary commands:

- **`list`**: Lists tools exposed by the MCP server.
- **`describe <tool_name>`**: Shows the input schema for a specific tool.
- **`call <tool_name> [params]`**: Calls a tool with JSON arguments.

### 2. Current Output Format (Unstructured)
Currently, all commands output to `stdout` using `println!`.

- **`list`**: Tabular text output.
  - File: `crates/cast-mcp-client/src/lib.rs`
  - Pattern:
    ```rust
    for tool in &tools {
        let description = tool.description.as_deref().unwrap_or("");
        println!("{:<30} {}", tool.name, description);
    }
    ```
- **`describe`**: Formatted text schema representation.
  - File: `crates/cast-mcp-client/src/lib.rs`
  - Pattern:
    ```rust
    pub fn print_tool_schema(name: &str, schema: &serde_json::Map<String, serde_json::Value>) {
        println!("Tool: {}", name);
        // ... (iterates and prints properties)
    }
    ```
- **`call`**: Mixed text and JSON.
  - File: `crates/cast-mcp-client/src/lib.rs`
  - Pattern:
    ```rust
    for item in &result.content {
        match &item.raw {
            RawContent::Text(t) => println!("{}", t.text),
            other => println!("{}", serde_json::to_string_pretty(other)?),
        }
    }
    ```

### 3. Underlying Data Structures (from `rmcp` crate)
The client relies on models from the `rmcp` library (version 1.6.0). These models already support `serde` serialization.

- **`rmcp::model::Tool`**:
  ```rust
  pub struct Tool {
      pub name: Cow<'static, str>,
      pub title: Option<String>,
      pub description: Option<Cow<'static, str>>,
      pub input_schema: Arc<JsonObject>,
      pub output_schema: Option<Arc<JsonObject>>,
      // ...
  }
  ```
- **`rmcp::model::CallToolResult`**:
  ```rust
  pub struct CallToolResult {
      pub content: Vec<Content>,
      pub structured_content: Option<serde_json::Value>,
      pub is_error: Option<bool>,
      // ...
  }
  ```
- **`rmcp::model::Content`**: An `Annotated<RawContent>` where `RawContent` is an enum:
  - `Text(RawTextContent)`
  - `Image(RawImageContent)`
  - `Resource(RawEmbeddedResource)`
  - `Audio(RawAudioContent)`
  - `ResourceLink(RawResource)`

### 4. Opportunities for Structured Output
Since the underlying models already derive `Serialize`, transitioning to JSON output can leverage these existing structures.

- **`list`**: Return `Vec<Tool>` or `{ "tools": Vec<Tool> }`.
- **`describe`**: Return the `Tool` object as JSON.
- **`call`**: Return the `CallToolResult` object as JSON.

## Confidence Notes
High confidence in the current code structure and data models. The `rmcp` models use `camelCase` for fields and `snake_case` for enum tags, which should be preserved in the CLI output for consistency with the MCP specification.
