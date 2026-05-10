use rmcp::model::Tool;
use serde_json::json;

pub struct DocEntry {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub content: &'static str,
}

pub const DOC_ENTRIES: &[DocEntry] = &[DocEntry {
    id: "mcp-config",
    title: "MCP Configuration",
    description: "Guide for configuring dynamic MCP tools in cast",
    content: include_str!("../../../docs/mcp/configuration.md"),
}];

pub fn builtin_tools() -> Vec<Tool> {
    vec![
        Tool::new_with_raw(
            "list_cast_documentation".to_string(),
            Some("List available cast documentation entries".into()),
            json!({
                "type": "object",
                "properties": {}
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
        Tool::new_with_raw(
            "fetch_cast_documentation".to_string(),
            Some("Fetch a specific cast documentation entry by ID".into()),
            json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "The ID of the documentation entry to fetch"
                    }
                },
                "required": ["id"]
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    ]
}
