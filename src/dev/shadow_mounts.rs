use crate::workspace::ResolvedWorkspace;
use std::path::PathBuf;

/// A resolved shadow mount, ready to be translated into Docker CLI arguments.
#[derive(Debug, Clone, PartialEq)]
pub enum ShadowMount {
    /// A directory to be masked with a read-only tmpfs overlay.
    Directory(PathBuf),
    /// A file to be masked by bind-mounting /dev/null over it.
    File(PathBuf),
}

/// Resolve a list of relative forbidden paths into `ShadowMount` values.
///
/// Each path is joined against `workspace.root` to check existence on the host,
/// and against `workspace.container_path` to determine the mount target inside
/// the container. Paths that escape the workspace root (absolute or containing
/// `..`) or do not exist on the host are silently skipped.
pub fn resolve_shadow_mounts(
    forbidden_paths: &[String],
    workspace: &ResolvedWorkspace,
) -> Vec<ShadowMount> {
    forbidden_paths
        .iter()
        .filter_map(|rel| {
            let host_path = workspace.root.join(rel);

            if !host_path.starts_with(&workspace.root) {
                return None;
            }

            let container_path = workspace.container_path.join(rel);

            if host_path.is_dir() {
                Some(ShadowMount::Directory(container_path))
            } else if host_path.is_file() {
                Some(ShadowMount::File(container_path))
            } else {
                None
            }
        })
        .collect()
}

/// Build Docker CLI arguments for a slice of resolved shadow mounts.
///
/// - `Directory` -> `--tmpfs <container_path>:ro,noexec,nosuid,size=1k,mode=000`
/// - `File` -> `-v /dev/null:<container_path>:ro`
pub fn build_shadow_mount_args(mounts: &[ShadowMount]) -> Vec<String> {
    let mut args = Vec::new();

    for mount in mounts {
        match mount {
            ShadowMount::Directory(container_path) => {
                args.push("--tmpfs".to_string());
                args.push(format!(
                    "{}:ro,noexec,nosuid,size=1k,mode=000",
                    container_path.display()
                ));
            }
            ShadowMount::File(container_path) => {
                args.push("-v".to_string());
                args.push(format!("/dev/null:{}:ro", container_path.display()));
            }
        }
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn workspace(tmp: &TempDir) -> ResolvedWorkspace {
        ResolvedWorkspace {
            root: tmp.path().to_path_buf(),
            container_path: PathBuf::from("/home/user/project"),
        }
    }

    // --- resolve_shadow_mounts ---

    #[test]
    fn test_resolve_empty_list() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace(&tmp);
        assert_eq!(resolve_shadow_mounts(&[], &ws), vec![]);
    }

    #[test]
    fn test_resolve_nonexistent_path_is_skipped() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace(&tmp);
        let result = resolve_shadow_mounts(&["does-not-exist".to_string()], &ws);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_resolve_existing_directory() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace(&tmp);
        std::fs::create_dir(tmp.path().join("secrets")).unwrap();

        let result = resolve_shadow_mounts(&["secrets".to_string()], &ws);

        assert_eq!(
            result,
            vec![ShadowMount::Directory(PathBuf::from(
                "/home/user/project/secrets"
            ))]
        );
    }

    #[test]
    fn test_resolve_existing_file() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace(&tmp);
        std::fs::write(tmp.path().join(".env"), "SECRET=1").unwrap();

        let result = resolve_shadow_mounts(&[".env".to_string()], &ws);

        assert_eq!(
            result,
            vec![ShadowMount::File(PathBuf::from("/home/user/project/.env"))]
        );
    }

    #[test]
    fn test_resolve_mixed_skips_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace(&tmp);
        std::fs::create_dir(tmp.path().join("secrets")).unwrap();
        std::fs::write(tmp.path().join(".env"), "SECRET=1").unwrap();

        let result = resolve_shadow_mounts(
            &[
                "secrets".to_string(),
                "ghost".to_string(),
                ".env".to_string(),
            ],
            &ws,
        );

        assert_eq!(
            result,
            vec![
                ShadowMount::Directory(PathBuf::from("/home/user/project/secrets")),
                ShadowMount::File(PathBuf::from("/home/user/project/.env")),
            ]
        );
    }

    // --- path safety ---

    #[test]
    fn test_resolve_absolute_path_is_skipped() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace(&tmp);
        let result = resolve_shadow_mounts(&["/etc/passwd".to_string()], &ws);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_resolve_parent_traversal_is_skipped() {
        let tmp = TempDir::new().unwrap();
        let ws = workspace(&tmp);
        let result = resolve_shadow_mounts(&["../outside".to_string()], &ws);
        assert_eq!(result, vec![]);
    }

    // --- build_shadow_mount_args ---

    #[test]
    fn test_args_empty() {
        assert_eq!(build_shadow_mount_args(&[]), Vec::<String>::new());
    }

    #[test]
    fn test_args_directory() {
        let mounts = vec![ShadowMount::Directory(PathBuf::from(
            "/home/user/project/secrets",
        ))];

        assert_eq!(
            build_shadow_mount_args(&mounts),
            vec![
                "--tmpfs",
                "/home/user/project/secrets:ro,noexec,nosuid,size=1k,mode=000",
            ]
        );
    }

    #[test]
    fn test_args_file() {
        let mounts = vec![ShadowMount::File(PathBuf::from("/home/user/project/.env"))];

        assert_eq!(
            build_shadow_mount_args(&mounts),
            vec!["-v", "/dev/null:/home/user/project/.env:ro"]
        );
    }

    #[test]
    fn test_args_mixed_order_preserved() {
        let mounts = vec![
            ShadowMount::Directory(PathBuf::from("/home/user/project/secrets")),
            ShadowMount::File(PathBuf::from("/home/user/project/.env")),
        ];

        assert_eq!(
            build_shadow_mount_args(&mounts),
            vec![
                "--tmpfs",
                "/home/user/project/secrets:ro,noexec,nosuid,size=1k,mode=000",
                "-v",
                "/dev/null:/home/user/project/.env:ro",
            ]
        );
    }
}
