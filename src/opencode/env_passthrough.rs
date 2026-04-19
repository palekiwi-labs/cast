/// Environment variables that should be passed through from the host to the container.
///
/// This includes LLM provider API keys, core OpenCode configuration flags, and path settings.
/// Note that `OPENCODE_CONFIG_DIR` is intentionally excluded here as it receives special
/// mount handling in the main `docker run` builder.
pub const PASSTHROUGH_VARS: &[&str] = &[
    // LLM Provider API Keys & Credentials
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "GOOGLE_GENERATIVE_AI_API_KEY",
    "AZURE_OPENAI_API_KEY",
    "OPENROUTER_API_KEY",
    "MISTRAL_API_KEY",
    "GROQ_API_KEY",
    "XAI_API_KEY",
    "DEEPSEEK_API_KEY",
    "TOGETHER_API_KEY",
    "PERPLEXITY_API_KEY",
    "FIREWORKS_API_KEY",
    // Core Configuration Flags
    "OPENCODE_AUTO_SHARE",
    "OPENCODE_DISABLE_AUTOUPDATE",
    "OPENCODE_DISABLE_PRUNE",
    "OPENCODE_DISABLE_AUTOCOMPACT",
    "OPENCODE_DISABLE_TERMINAL_TITLE",
    "OPENCODE_DISABLE_PROJECT_CONFIG",
    "OPENCODE_DISABLE_SHARE",
    "OPENCODE_EXPERIMENTAL",
    "OPENCODE_PERMISSION",
    "OPENCODE_MODELS_URL",
    "OPENCODE_GIT_BASH_PATH",
    "OPENCODE_SERVER_USERNAME",
    "OPENCODE_SERVER_PASSWORD",
    // Configuration with Paths (users must provide container paths)
    "OPENCODE_CONFIG",
    "OPENCODE_CONFIG_CONTENT",
    "OPENCODE_MODELS_PATH",
];

/// Generates docker run arguments for OpenCode environment variables.
///
/// Only includes variables from `PASSTHROUGH_VARS` that are actually set in the
/// host environment. Uses the Docker name-only syntax (`-e VAR_NAME`) which allows
/// the container process to inherit the value directly from the host environment
/// without the value ever appearing in the command-line arguments.
pub fn build_passthrough_env_args() -> Vec<String> {
    PASSTHROUGH_VARS
        .iter()
        .filter(|&&var| std::env::var(var).is_ok())
        .flat_map(|&var| ["-e".to_string(), var.to_string()])
        .collect()
}
