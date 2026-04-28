use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Return the opencode configuration directory path without ensuring it exists.
pub fn get_config_dir() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("Failed to resolve user config directory")?
        .join("opencode"))
}

/// Ensure the opencode configuration directory exists on the host and return its path.
pub fn ensure_config_dir() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;

    fs::create_dir_all(&config_dir).with_context(|| {
        format!(
            "Failed to create config directory at {}",
            config_dir.display()
        )
    })?;

    Ok(config_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_dir() {
        let path = get_config_dir().unwrap();
        assert!(path.to_string_lossy().contains("opencode"));
    }
}
