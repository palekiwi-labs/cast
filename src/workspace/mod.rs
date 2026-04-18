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
pub fn get_workspace(username: &str) -> Result<ResolvedWorkspace> {
    let root = std::env::current_dir().context("Failed to get current directory")?;
    let home_dir = dirs::home_dir();
    let container_path = map_container_path(&root, home_dir.as_deref(), username);
    Ok(ResolvedWorkspace {
        root,
        container_path,
    })
}

/// Map a host path to its container-side equivalent.
///
///   within home_dir  → /home/<username>/<rel>
///   outside home_dir → /workspace/<abs_without_leading_slash>
fn map_container_path(root: &Path, home_dir: Option<&Path>, username: &str) -> PathBuf {
    if let Some(home) = home_dir
        && let Ok(rel) = root.strip_prefix(home) {
            PathBuf::from("/home").join(username).join(rel)
    } else {
        let stripped = root.strip_prefix("/").unwrap_or(root);
        PathBuf::from("/workspace").join(stripped)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_path_linux_within_home() {
        let root = Path::new("/home/alice/projects/my-app");
        let home = Path::new("/home/alice");
        assert_eq!(
            map_container_path(root, Some(home), "alice"),
            PathBuf::from("/home/alice/projects/my-app")
        );
    }

    #[test]
    fn test_container_path_macos_within_home() {
        let root = Path::new("/Users/alice/projects/my-app");
        let home = Path::new("/Users/alice");
        assert_eq!(
            map_container_path(root, Some(home), "alice"),
            PathBuf::from("/home/alice/projects/my-app")
        );
    }

    #[test]
    fn test_container_path_at_home_root() {
        let root = Path::new("/home/alice");
        let home = Path::new("/home/alice");
        assert_eq!(
            map_container_path(root, Some(home), "alice"),
            PathBuf::from("/home/alice")
        );
    }

    #[test]
    fn test_container_path_outside_home() {
        let root = Path::new("/srv/projects/my-app");
        let home = Path::new("/home/alice");
        assert_eq!(
            map_container_path(root, Some(home), "alice"),
            PathBuf::from("/workspace/srv/projects/my-app")
        );
    }

    #[test]
    fn test_container_path_no_home_dir() {
        let root = Path::new("/srv/projects/my-app");
        assert_eq!(
            map_container_path(root, None, "alice"),
            PathBuf::from("/workspace/srv/projects/my-app")
        );
    }
}
