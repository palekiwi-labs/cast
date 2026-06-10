use std::collections::HashMap;

/// Environment variables that should be passed through from the host to the container.
///
/// Covers Anthropic direct API, Bedrock, and Vertex AI access patterns, plus
/// Claude Code-specific control knobs.
pub const PASSTHROUGH_VARS: &[&str] = &[
    // LLM Provider API Keys
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "GOOGLE_GENERATIVE_AI_API_KEY",
    // Claude Code specific
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "ANTHROPIC_BASE_URL",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
    "CLAUDE_CODE_MAX_OUTPUT_TOKENS",
    // AWS Bedrock
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_REGION",
    "AWS_PROFILE",
    // Google Vertex
    "GOOGLE_APPLICATION_CREDENTIALS",
    "GOOGLE_CLOUD_PROJECT",
    "CLOUD_ML_REGION",
];

/// Generates docker run arguments for Claude Code environment variables.
///
/// Only includes variables from `PASSTHROUGH_VARS` that are actually set in the
/// injected `env` map. Uses the Docker name-only syntax (`-e VAR_NAME`) which allows
/// the container process to inherit the value directly from the host environment
/// without the value ever appearing in the command-line arguments.
pub fn build_passthrough_env_args(env: &HashMap<String, String>) -> Vec<String> {
    PASSTHROUGH_VARS
        .iter()
        .filter(|&&var| env.contains_key(var))
        .flat_map(|&var| ["-e".to_string(), var.to_string()])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_passthrough_env_args() {
        let mut env = HashMap::new();
        env.insert("ANTHROPIC_API_KEY".to_string(), "sk-123".to_string());
        env.insert("AWS_REGION".to_string(), "us-east-1".to_string());
        env.insert("UNKNOWN_VAR".to_string(), "foo".to_string());

        let args = build_passthrough_env_args(&env);

        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"ANTHROPIC_API_KEY".to_string()));
        assert!(args.contains(&"AWS_REGION".to_string()));
        assert!(!args.contains(&"UNKNOWN_VAR".to_string()));
        assert_eq!(args.len(), 4);
    }

    #[test]
    fn test_build_passthrough_env_args_empty() {
        let env = HashMap::new();
        let args = build_passthrough_env_args(&env);
        assert!(args.is_empty());
    }
}
