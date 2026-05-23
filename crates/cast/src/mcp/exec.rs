use crate::config::{ArgTemplate, McpEnvConfig, McpToolConfig};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;

pub struct CallToolResult {
    pub content: Vec<McpContent>,
    pub is_error: bool,
}

pub struct McpContent {
    pub text: String,
}

pub async fn run_command(
    tool: &McpToolConfig,
    mapped_args: Vec<String>,
    host_env: &HashMap<String, String>,
) -> Result<CallToolResult> {
    let default_env = McpEnvConfig::default();
    let env_config = tool.env.as_ref().unwrap_or(&default_env);
    let resolved_env = resolve_env(env_config, host_env);
    let (exe, args) = build_exec_command(tool, mapped_args);

    let mut cmd = Command::new(&exe);
    cmd.args(args);
    cmd.env_clear();
    cmd.envs(resolved_env);

    if let Some(dir) = &tool.working_dir {
        cmd.current_dir(dir);
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return Ok(CallToolResult {
                content: vec![McpContent {
                    text: format!("Failed to spawn command '{}': {}", exe, e),
                }],
                is_error: true,
            });
        }
    };

    let output = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => {
            return Ok(CallToolResult {
                content: vec![McpContent {
                    text: format!("Failed to read command output: {}", e),
                }],
                is_error: true,
            });
        }
    };

    let is_error = !output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let mut combined_output = stdout;
    if !stderr.is_empty() {
        if !combined_output.is_empty() && !combined_output.ends_with('\n') {
            combined_output.push('\n');
        }
        combined_output.push_str(&stderr);
    }

    // If combined output is still empty but it's an error, provide a hint
    if combined_output.is_empty() && is_error {
        combined_output = format!("Command failed with status: {}", output.status);
    }

    Ok(CallToolResult {
        content: vec![McpContent {
            text: combined_output,
        }],
        is_error,
    })
}

pub fn resolve_env(
    config: &McpEnvConfig,
    host_env: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut final_env = HashMap::new();

    // 1. Always retain critical system variables (for Nix compatibility)
    for key in &["PATH", "TMPDIR"] {
        if let Some(val) = host_env.get(*key) {
            final_env.insert(key.to_string(), val.clone());
        }
    }

    // 2. Map inherited variables
    for key in &config.inherit {
        if let Some(val) = host_env.get(key) {
            final_env.insert(key.clone(), val.clone());
        }
    }

    // 3. Set static variables (overrides inheritance)
    for (key, val) in &config.set {
        final_env.insert(key.clone(), val.clone());
    }

    final_env
}

pub fn build_exec_command(tool: &McpToolConfig, mapped_args: Vec<String>) -> (String, Vec<String>) {
    (tool.command.clone(), mapped_args)
}

pub fn map_args(templates: &[ArgTemplate], args: &Value) -> Result<Vec<String>> {
    let mut final_args = Vec::new();

    for template in templates {
        match template {
            ArgTemplate::Literal(s) => {
                final_args.extend(expand_placeholder(s, args)?);
            }
            ArgTemplate::Conditional(cond) => {
                let mut should_include = true;

                if let Some(key) = &cond.if_present {
                    should_include &= args.get(key).is_some() && !args[key].is_null();
                }
                if let Some(key) = &cond.if_true {
                    should_include &= args.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
                }

                if (cond.if_present.is_some() || cond.if_true.is_some()) && should_include {
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
    fn test_map_args_conditional_both() {
        let templates = vec![ArgTemplate::Conditional(ConditionalBlock {
            if_present: Some("format".to_string()),
            if_true: Some("fail_fast".to_string()),
            args: vec!["--flag".to_string()],
        })];

        // Both true -> include
        let args = json!({ "format": "json", "fail_fast": true });
        assert_eq!(map_args(&templates, &args).unwrap(), vec!["--flag"]);

        // One false -> exclude
        let args = json!({ "format": "json", "fail_fast": false });
        assert!(map_args(&templates, &args).unwrap().is_empty());

        // One missing -> exclude
        let args = json!({ "fail_fast": true });
        assert!(map_args(&templates, &args).unwrap().is_empty());
    }

    #[test]
    fn test_resolve_env() {
        let mut host_env = HashMap::new();
        host_env.insert("PATH".to_string(), "/usr/bin".to_string());
        host_env.insert("TMPDIR".to_string(), "/tmp/nix-shell.123".to_string());
        host_env.insert("USER".to_string(), "alice".to_string());
        host_env.insert("HOME".to_string(), "/home/alice".to_string());
        host_env.insert("MY_VAR".to_string(), "original".to_string());

        let config = McpEnvConfig {
            inherit: vec!["USER".to_string(), "MY_VAR".to_string()],
            set: vec![("MY_VAR".to_string(), "overridden".to_string())]
                .into_iter()
                .collect(),
        };

        let resolved = resolve_env(&config, &host_env);

        // PATH and TMPDIR are always present
        assert_eq!(resolved.get("PATH").unwrap(), "/usr/bin");
        assert_eq!(resolved.get("TMPDIR").unwrap(), "/tmp/nix-shell.123");
        // USER is inherited
        assert_eq!(resolved.get("USER").unwrap(), "alice");
        // MY_VAR is overridden by set
        assert_eq!(resolved.get("MY_VAR").unwrap(), "overridden");
        // HOME is NOT inherited
        assert!(!resolved.contains_key("HOME"));
    }

    #[test]
    fn test_build_exec_command() {
        let tool = McpToolConfig {
            description: "test".to_string(),
            command: "ls".to_string(),
            args: vec![],
            env: Some(McpEnvConfig::default()),
            working_dir: Some("/tmp".to_string()),
            parameters: json!({}),
        };
        let mapped_args = vec!["-la".to_string()];
        let (exe, args) = build_exec_command(&tool, mapped_args);
        assert_eq!(exe, "ls");
        assert_eq!(args, vec!["-la"]);
    }
}
