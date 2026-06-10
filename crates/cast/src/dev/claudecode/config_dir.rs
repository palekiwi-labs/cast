use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Return the Claude Code configuration directory path relative to the provided base.
pub fn get_config_dir(base: &Path) -> PathBuf {
    base.join(".claude")
}

/// Ensure the Claude Code configuration directory exists on the host and return its path.
pub fn ensure_config_dir(base: &Path) -> Result<PathBuf> {
    let config_dir = get_config_dir(base);

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
        let base = Path::new("/home/alice");
        let path = get_config_dir(base);
        assert_eq!(path, Path::new("/home/alice/.claude"));
    }

    #[test]
    fn test_ensure_config_dir_creates_directory() {
        let temp = tempfile::TempDir::new().unwrap();
        let result = ensure_config_dir(temp.path()).unwrap();
        assert!(result.exists());
        assert_eq!(result, temp.path().join(".claude"));
    }
}
