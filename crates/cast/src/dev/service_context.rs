use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Worktree-scoped context for a service command invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceContext {
    pub worktree_root: PathBuf,
    pub relative_cwd: PathBuf,
}

impl ServiceContext {
    pub fn resolve(cwd: &Path) -> Result<Self> {
        let cwd = cwd
            .canonicalize()
            .with_context(|| format!("Failed to resolve current directory {}", cwd.display()))?;
        let output = Command::new("git")
            .arg("-C")
            .arg(&cwd)
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .context("Failed to run git while resolving the worktree")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "Current directory is not in a Git worktree: {}",
                stderr.trim()
            );
        }

        let worktree_root = PathBuf::from(
            String::from_utf8(output.stdout)
                .context("Git returned a non-UTF-8 worktree path")?
                .trim(),
        )
        .canonicalize()
        .context("Failed to resolve the Git worktree root")?;
        let relative_cwd = cwd
            .strip_prefix(&worktree_root)
            .context("Current directory is outside the resolved Git worktree")?
            .to_path_buf();

        Ok(Self {
            worktree_root,
            relative_cwd,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn root_and_subdirectory_resolve_to_the_same_worktree() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let worktree = temp.path().join("project");
        let subdirectory = worktree.join("crates/app");
        fs::create_dir_all(&subdirectory).expect("create project directory");
        let status = Command::new("git")
            .args(["init", "--quiet"])
            .arg(&worktree)
            .status()
            .expect("run git init");
        assert!(status.success(), "git init should succeed");

        let from_root = ServiceContext::resolve(&worktree).expect("resolve worktree root");
        let from_subdirectory =
            ServiceContext::resolve(&subdirectory).expect("resolve worktree subdirectory");

        assert_eq!(from_root.worktree_root, from_subdirectory.worktree_root);
        assert_eq!(from_root.relative_cwd, std::path::Path::new(""));
        assert_eq!(
            from_subdirectory.relative_cwd,
            std::path::Path::new("crates/app")
        );
    }
}
