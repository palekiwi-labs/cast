use std::collections::HashMap;

/// Environment variables that should be passed through from the host to the container.
///
/// Only variables explicitly documented in the Claude Code environment variable
/// reference are included. Each entry has a strong documented reason to be here.
///
/// Notably absent:
/// - OPENAI_API_KEY, GOOGLE_GENERATIVE_AI_API_KEY: not Claude Code variables
/// - AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_PROFILE: standard AWS SDK
///   conventions picked up automatically; not Claude Code-specific
/// - GOOGLE_APPLICATION_CREDENTIALS: a host file path that won't exist inside
///   the container — passing it through would cause silent Vertex AI auth failure
/// - CLOUD_ML_REGION: not in the official Claude Code env-vars reference
pub const PASSTHROUGH_VARS: &[&str] = &[
    // Authentication
    "ANTHROPIC_API_KEY",
    // Provider selection
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    // API routing
    "ANTHROPIC_BASE_URL",
    // Behaviour
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
    "CLAUDE_CODE_MAX_OUTPUT_TOKENS",
    // AWS Bedrock (region is documented; credentials flow via AWS SDK conventions)
    "AWS_REGION",
    // Google Vertex AI
    "GOOGLE_CLOUD_PROJECT",
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
        // removed vars must not appear
        env.insert("OPENAI_API_KEY".to_string(), "sk-oai".to_string());
        env.insert(
            "GOOGLE_APPLICATION_CREDENTIALS".to_string(),
            "/home/user/.config/gcloud/creds.json".to_string(),
        );

        let args = build_passthrough_env_args(&env);

        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"ANTHROPIC_API_KEY".to_string()));
        assert!(args.contains(&"AWS_REGION".to_string()));
        assert!(!args.contains(&"UNKNOWN_VAR".to_string()));
        assert!(!args.contains(&"OPENAI_API_KEY".to_string()));
        assert!(!args.contains(&"GOOGLE_APPLICATION_CREDENTIALS".to_string()));
        assert_eq!(args.len(), 4);
    }

    #[test]
    fn test_build_passthrough_env_args_empty() {
        let env = HashMap::new();
        let args = build_passthrough_env_args(&env);
        assert!(args.is_empty());
    }
}
