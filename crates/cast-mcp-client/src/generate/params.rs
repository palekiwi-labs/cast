use rmcp::model::Tool;

/// Metadata extracted from a single JSON Schema property.
pub(crate) struct ParamSpec {
    /// Original JSON property name (used as JSON key and jq variable name).
    pub name: String,
    /// Kebab-case version used for the bash `--flag` name.
    pub flag: String,
    /// ALL_CAPS bash variable name.
    pub var: String,
    /// Uppercased type string shown in `--help` (e.g. "STRING", "INTEGER").
    pub type_hint: String,
    /// Whether to use `--argjson` (true) or `--arg` (false) in jq.
    pub json_arg: bool,
    /// Description from the schema, or empty string.
    pub description: String,
    /// Whether this parameter appears in `required`.
    pub required: bool,
}

/// Extract `ParamSpec` list from a `Tool`'s `inputSchema`.
///
/// Uses serialization to access the schema without depending on rmcp's
/// private fields.
pub(crate) fn parse_params(tool: &Tool) -> Vec<ParamSpec> {
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

/// Convert a camelCase or snake_case identifier to kebab-case.
///
/// Rules (no regex):
/// - Insert `-` before an uppercase letter that follows a lowercase/digit.
/// - Insert `-` before an uppercase letter that follows another uppercase AND
///   is itself followed by a lowercase (handles acronyms like "APIKey" →
///   "api-key").
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
