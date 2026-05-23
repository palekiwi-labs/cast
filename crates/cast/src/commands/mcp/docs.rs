use include_dir::{include_dir, Dir};
use rmcp::model::Tool;
use serde_json::json;

static DOCS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/docs");

pub fn list_docs() -> Vec<String> {
    fn collect_files(dir: &Dir, paths: &mut Vec<String>) {
        for entry in dir.entries() {
            match entry {
                include_dir::DirEntry::Dir(d) => collect_files(d, paths),
                include_dir::DirEntry::File(f) => {
                    let path = f.path().to_string_lossy();
                    if let Some(stripped) = path.strip_suffix(".md") {
                        paths.push(stripped.to_string());
                    }
                }
            }
        }
    }
    let mut paths = Vec::new();
    collect_files(&DOCS_DIR, &mut paths);
    paths
}

pub fn fetch_doc(id: &str) -> Option<&'static str> {
    let path = format!("{}.md", id);
    DOCS_DIR
        .get_file(path)
        .and_then(|file| file.contents_utf8())
}

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
