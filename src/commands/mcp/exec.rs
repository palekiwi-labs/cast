use crate::config::ArgTemplate;
use anyhow::Result;
use serde_json::Value;

pub fn map_args(templates: &[ArgTemplate], args: &Value) -> Result<Vec<String>> {
    let mut final_args = Vec::new();

    for template in templates {
        match template {
            ArgTemplate::Literal(s) => {
                final_args.extend(expand_placeholder(s, args)?);
            }
            ArgTemplate::Conditional(cond) => {
                let mut should_include = false;

                if let Some(key) = &cond.if_present {
                    should_include = args.get(key).is_some() && !args[key].is_null();
                }
                if let Some(key) = &cond.if_true {
                    should_include = args.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
                }

                if should_include {
                    for inner_arg in &cond.args {
                        final_args.extend(expand_placeholder(inner_arg, args)?);
                    }
                }
            }
        }
    }

    Ok(final_args)
}

fn expand_placeholder(template: &str, args: &Value) -> Result<Vec<String>> {
    // 1. Check for spread operator: "{...name}"
    if template.starts_with("{...") && template.ends_with('}') {
        let name = &template[4..template.len() - 1];
        if let Some(arr) = args.get(name).and_then(|v| v.as_array()) {
            return Ok(arr
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| v.to_string())
                })
                .collect());
        }
        return Ok(Vec::new());
    }

    // 2. Handle {name} placeholders
    let mut result = template.to_string();
    let mut i = 0;
    while let Some(start) = result[i..].find('{') {
        let start = i + start;
        if let Some(end_rel) = result[start..].find('}') {
            let end = start + end_rel;
            let name = &result[start + 1..end];

            if let Some(val) = args.get(name) {
                let replacement = val
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| val.to_string());
                result.replace_range(start..end + 1, &replacement);
                i = start + replacement.len();
            } else {
                // Placeholder not found, remove it
                result.replace_range(start..end + 1, "");
                i = start;
            }
        } else {
            break;
        }
    }

    if result.is_empty() && template.starts_with('{') && template.ends_with('}') {
        return Ok(Vec::new());
    }

    Ok(vec![result])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConditionalBlock;
    use serde_json::json;

    #[test]
    fn test_map_args_basic() {
        let templates = vec![
            ArgTemplate::Literal("rspec".to_string()),
            ArgTemplate::Literal("{test_file}".to_string()),
        ];
        let args = json!({ "test_file": "spec/my_spec.rb" });
        let result = map_args(&templates, &args).unwrap();
        assert_eq!(result, vec!["rspec", "spec/my_spec.rb"]);
    }

    #[test]
    fn test_map_args_spread() {
        let templates = vec![
            ArgTemplate::Literal("ls".to_string()),
            ArgTemplate::Literal("{...files}".to_string()),
        ];
        let args = json!({ "files": ["a.txt", "b.txt"] });
        let result = map_args(&templates, &args).unwrap();
        assert_eq!(result, vec!["ls", "a.txt", "b.txt"]);
    }

    #[test]
    fn test_map_args_conditional_present() {
        let templates = vec![
            ArgTemplate::Conditional(ConditionalBlock {
                if_present: Some("format".to_string()),
                if_true: None,
                args: vec!["--format".to_string(), "{format}".to_string()],
            }),
            ArgTemplate::Literal("spec/file.rb".to_string()),
        ];

        // Case 1: Present
        let args = json!({ "format": "json" });
        let result = map_args(&templates, &args).unwrap();
        assert_eq!(result, vec!["--format", "json", "spec/file.rb"]);

        // Case 2: Absent
        let args = json!({});
        let result = map_args(&templates, &args).unwrap();
        assert_eq!(result, vec!["spec/file.rb"]);
    }

    #[test]
    fn test_map_args_conditional_true() {
        let templates = vec![ArgTemplate::Conditional(ConditionalBlock {
            if_present: None,
            if_true: Some("fail_fast".to_string()),
            args: vec!["--fail-fast".to_string()],
        })];

        // Case 1: True
        let args = json!({ "fail_fast": true });
        let result = map_args(&templates, &args).unwrap();
        assert_eq!(result, vec!["--fail-fast"]);

        // Case 2: False
        let args = json!({ "fail_fast": false });
        let result = map_args(&templates, &args).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_map_args_mixed() {
        let templates = vec![ArgTemplate::Literal("--file={test_file}".to_string())];
        let args = json!({ "test_file": "data.txt" });
        let result = map_args(&templates, &args).unwrap();
        assert_eq!(result, vec!["--file=data.txt"]);
    }
}
