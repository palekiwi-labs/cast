use std::collections::HashMap;

/// Environment variables that should be passed through from the host to the container.
pub const PASSTHROUGH_VARS: &[&str] = &[
    // LLM Provider API Keys & Credentials
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "GOOGLE_GENERATIVE_AI_API_KEY",
    "GEMINI_API_KEY",
    "AZURE_OPENAI_API_KEY",
    "OPENROUTER_API_KEY",
    "MISTRAL_API_KEY",
    "GROQ_API_KEY",
    "XAI_API_KEY",
    "DEEPSEEK_API_KEY",
    "TOGETHER_API_KEY",
    "PERPLEXITY_API_KEY",
    "FIREWORKS_API_KEY",
    "CLOUDFLARE_API_KEY",
    // Pi-specific variables
    "PI_CODING_AGENT_DIR",
    "PI_CODING_AGENT_SESSION_DIR",
    "PI_PACKAGE_DIR",
    "PI_OFFLINE",
    "PI_TELEMETRY",
    // AWS Bedrock
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_REGION",
    "AWS_PROFILE",
];

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
        env.insert("PI_OFFLINE".to_string(), "true".to_string());
        env.insert("UNKNOWN_VAR".to_string(), "foo".to_string());

        let args = build_passthrough_env_args(&env);

        // Sort to ensure consistent comparison if order changes (though iter() on slice is stable)
        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"ANTHROPIC_API_KEY".to_string()));
        assert!(args.contains(&"PI_OFFLINE".to_string()));
        assert_eq!(args.len(), 4);
    }
}
