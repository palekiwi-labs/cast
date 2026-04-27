use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Ensure the opencode configuration directory exists on the host and return its path.
pub fn ensure_config_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Failed to resolve user config directory")?
        .join("opencode");

    fs::create_dir_all(&config_dir).with_context(|| {
        format!(
            "Failed to create config directory at {}",
            config_dir.display()
        )
    })?;

    Ok(config_dir)
}
