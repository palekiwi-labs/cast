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

/// Return the Claude Code global config file path relative to the provided base.
pub fn get_config_file(base: &Path) -> PathBuf {
    base.join(".claude.json")
}

/// Ensure the Claude Code global config file exists on the host and return its path.
///
/// Docker bind-mounts a non-existent host path as a directory, which would corrupt
/// the expected file layout. Touching the file here prevents that.
pub fn ensure_config_file(base: &Path) -> Result<PathBuf> {
    let config_file = get_config_file(base);

    if !config_file.exists() {
        fs::write(&config_file, "{}").with_context(|| {
            format!("Failed to create config file at {}", config_file.display())
        })?;
    }

    Ok(config_file)
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

    #[test]
    fn test_get_config_file() {
        let base = Path::new("/home/alice");
        let path = get_config_file(base);
        assert_eq!(path, Path::new("/home/alice/.claude.json"));
    }

    #[test]
    fn test_ensure_config_file_creates_file_with_empty_json() {
        let temp = tempfile::TempDir::new().unwrap();
        let result = ensure_config_file(temp.path()).unwrap();
        assert!(result.exists());
        assert!(result.is_file());
        assert_eq!(result, temp.path().join(".claude.json"));
        let content = fs::read_to_string(&result).unwrap();
        assert_eq!(content, "{}");
    }

    #[test]
    fn test_ensure_config_file_is_idempotent() {
        let temp = tempfile::TempDir::new().unwrap();
        // Write some content so we can verify it is not truncated on second call.
        let path = temp.path().join(".claude.json");
        fs::write(&path, r#"{"key":"value"}"#).unwrap();

        ensure_config_file(temp.path()).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, r#"{"key":"value"}"#);
    }
}
