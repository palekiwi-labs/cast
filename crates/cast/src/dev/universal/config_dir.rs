use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Return the cross-harness `.agents` config directory path relative to the
/// provided base (the host home directory).
///
/// `.agents` is the vendor-neutral standard (`agentskills.io`) shared across
/// harnesses for global skills (`~/.agents/skills/`).
pub fn get_config_dir(base: &Path) -> PathBuf {
    base.join(".agents")
}

/// Ensure the cross-harness `.agents` config directory exists on the host and
/// return its path.
///
/// Called during universal host preparation so that a missing host directory is
/// created with correct user ownership instead of Docker auto-creating a
/// root-owned path when the bind mount is attached.
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
    fn get_config_dir_appends_agents() {
        let path = get_config_dir(Path::new("/home/alice"));
        assert_eq!(path, Path::new("/home/alice/.agents"));
    }

    #[test]
    fn ensure_config_dir_creates_directory() {
        let temp = tempfile::tempdir().unwrap();
        let result = ensure_config_dir(temp.path()).unwrap();

        assert_eq!(result, temp.path().join(".agents"));
        assert!(result.exists(), "expected .agents to be created");
        assert!(result.is_dir(), "expected .agents to be a directory");
    }
}
