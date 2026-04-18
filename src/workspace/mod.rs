use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Abstracts host system directory lookup so resolution logic is fully testable.
pub trait ResolveWorkspace {
    fn current_dir(&self) -> Result<PathBuf>;
}

/// Production implementation — reads the real current working directory.
pub struct HostWorkspace;

impl ResolveWorkspace for HostWorkspace {
    fn current_dir(&self) -> Result<PathBuf> {
        std::env::current_dir().context("Failed to get current directory")
    }
}

/// The resolved workspace context.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedWorkspace {
    pub root: PathBuf,
    pub container_path: PathBuf,
}

/// Resolve the workspace root and container path mapping.
///
/// Resolution steps:
///   root:           current_dir() via the trait, then canonicalized
///   container_path: within home_dir → /home/<dirname>/<rel>
///                   outside home_dir → /workspace/<abs_without_leading_slash>
pub fn resolve_workspace(
    host: &impl ResolveWorkspace,
    home_dir: Option<&Path>,
) -> Result<ResolvedWorkspace> {
    let raw = host.current_dir()?;

    // Canonicalize to resolve symlinks and relative segments; fall back to raw
    // path when canonicalization fails (e.g. non-existent path in tests).
    let root = std::fs::canonicalize(&raw).unwrap_or(raw);

    let container_path = map_container_path(&root, home_dir);

    Ok(ResolvedWorkspace {
        root,
        container_path,
    })
}

/// Map a host path to its container-side equivalent.
fn map_container_path(root: &Path, home_dir: Option<&Path>) -> PathBuf {
    if let Some(home) = home_dir
        && let Ok(rel) = root.strip_prefix(home) {
            // Within home: /home/<dirname>/<rel>
            let home_dirname = home
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("user"));
            return PathBuf::from("/home").join(home_dirname).join(rel);
        }

    // Outside home: /workspace/<absolute_path_without_leading_slash>
    let stripped = root.strip_prefix("/").unwrap_or(root);
    PathBuf::from("/workspace").join(stripped)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockWorkspace {
        current_dir: PathBuf,
    }

    impl ResolveWorkspace for MockWorkspace {
        fn current_dir(&self) -> Result<PathBuf> {
            Ok(self.current_dir.clone())
        }
    }

    fn mock(path: &str) -> MockWorkspace {
        MockWorkspace {
            current_dir: PathBuf::from(path),
        }
    }

    #[test]
    fn test_root_is_current_dir() {
        let host = mock("/home/alice/my-project");
        let result = resolve_workspace(&host, None).unwrap();
        assert_eq!(result.root, PathBuf::from("/home/alice/my-project"));
    }

    #[test]
    fn test_container_path_within_home() {
        let host = mock("/home/alice/projects/my-app");
        let home = PathBuf::from("/home/alice");
        let result = resolve_workspace(&host, Some(home.as_path())).unwrap();
        assert_eq!(
            result.container_path,
            PathBuf::from("/home/alice/projects/my-app")
        );
    }

    #[test]
    fn test_container_path_at_home_root() {
        let host = mock("/home/alice");
        let home = PathBuf::from("/home/alice");
        let result = resolve_workspace(&host, Some(home.as_path())).unwrap();
        assert_eq!(result.container_path, PathBuf::from("/home/alice"));
    }

    #[test]
    fn test_container_path_outside_home() {
        let host = mock("/srv/projects/my-app");
        let home = PathBuf::from("/home/alice");
        let result = resolve_workspace(&host, Some(home.as_path())).unwrap();
        assert_eq!(
            result.container_path,
            PathBuf::from("/workspace/srv/projects/my-app")
        );
    }

    #[test]
    fn test_container_path_no_home_dir() {
        let host = mock("/srv/projects/my-app");
        let result = resolve_workspace(&host, None).unwrap();
        assert_eq!(
            result.container_path,
            PathBuf::from("/workspace/srv/projects/my-app")
        );
    }

    #[test]
    fn test_host_workspace_resolves_on_real_system() {
        let host = HostWorkspace;
        assert!(host.current_dir().is_ok());
    }
}
