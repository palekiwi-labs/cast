use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// The resolved workspace context.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedWorkspace {
    pub root: PathBuf,
    pub container_path: PathBuf,
}

/// Imperative shell — performs all OS interaction.
/// Call this from the command handler.
pub fn get_workspace() -> Result<ResolvedWorkspace> {
    let raw = std::env::current_dir().context("Failed to get current directory")?;
    let root = std::fs::canonicalize(&raw).context("Failed to canonicalize current directory")?;
    let home_dir = dirs::home_dir();
    Ok(resolve_workspace(&root, home_dir.as_deref()))
}

/// Functional core — pure path mapping logic, no I/O.
///
/// container_path mapping:
///   within home_dir  → /home/<dirname>/<rel>
///   outside home_dir → /workspace/<abs_without_leading_slash>
pub fn resolve_workspace(root: &Path, home_dir: Option<&Path>) -> ResolvedWorkspace {
    let container_path = map_container_path(root, home_dir);
    ResolvedWorkspace {
        root: root.to_path_buf(),
        container_path,
    }
}

/// Map a host path to its container-side equivalent.
fn map_container_path(root: &Path, home_dir: Option<&Path>) -> PathBuf {
    if let Some(home) = home_dir {
        if let Ok(rel) = root.strip_prefix(home) {
            // Within home: /home/<dirname>/<rel>
            let home_dirname = home
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("user"));
            return PathBuf::from("/home").join(home_dirname).join(rel);
        }
    }

    // Outside home: /workspace/<absolute_path_without_leading_slash>
    let stripped = root.strip_prefix("/").unwrap_or(root);
    PathBuf::from("/workspace").join(stripped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_is_preserved() {
        let root = Path::new("/home/alice/my-project");
        let result = resolve_workspace(root, None);
        assert_eq!(result.root, PathBuf::from("/home/alice/my-project"));
    }

    #[test]
    fn test_container_path_within_home() {
        let root = Path::new("/home/alice/projects/my-app");
        let home = Path::new("/home/alice");
        let result = resolve_workspace(root, Some(home));
        assert_eq!(
            result.container_path,
            PathBuf::from("/home/alice/projects/my-app")
        );
    }

    #[test]
    fn test_container_path_at_home_root() {
        let root = Path::new("/home/alice");
        let home = Path::new("/home/alice");
        let result = resolve_workspace(root, Some(home));
        assert_eq!(result.container_path, PathBuf::from("/home/alice"));
    }

    #[test]
    fn test_container_path_outside_home() {
        let root = Path::new("/srv/projects/my-app");
        let home = Path::new("/home/alice");
        let result = resolve_workspace(root, Some(home));
        assert_eq!(
            result.container_path,
            PathBuf::from("/workspace/srv/projects/my-app")
        );
    }

    #[test]
    fn test_container_path_no_home_dir() {
        let root = Path::new("/srv/projects/my-app");
        let result = resolve_workspace(root, None);
        assert_eq!(
            result.container_path,
            PathBuf::from("/workspace/srv/projects/my-app")
        );
    }
}
