use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Worktree-scoped context for a service command invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceContext {
    pub worktree_root: PathBuf,
    pub git_common_dir: PathBuf,
    pub relative_cwd: PathBuf,
    pub workspace_id: String,
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
        let common_dir_output = Command::new("git")
            .arg("-C")
            .arg(&cwd)
            .args(["rev-parse", "--git-common-dir"])
            .output()
            .context("Failed to run git while resolving the common directory")?;
        if !common_dir_output.status.success() {
            bail!("Failed to resolve the Git common directory");
        }
        let git_common_dir = PathBuf::from(
            String::from_utf8(common_dir_output.stdout)
                .context("Git returned a non-UTF-8 common directory path")?
                .trim(),
        );
        let git_common_dir = if git_common_dir.is_absolute() {
            git_common_dir
        } else {
            cwd.join(git_common_dir)
        }
        .canonicalize()
        .context("Failed to resolve the Git common directory")?;
        let digest = Sha256::digest(worktree_root.as_os_str().as_encoded_bytes());
        let workspace_id = hex::encode(&digest[..6]);

        Ok(Self {
            worktree_root,
            git_common_dir,
            relative_cwd,
            workspace_id,
        })
    }

    pub fn container_name(&self, service_name: Option<&str>) -> String {
        let basename = self
            .worktree_root
            .file_name()
            .expect("Git worktree root should have a basename")
            .to_string_lossy();
        let base = format!("cast-{basename}-{}", self.workspace_id);

        match service_name {
            Some(name) => format!("{base}-{name}"),
            None => base,
        }
    }

    pub fn multiplexer_session_name(&self, service_name: Option<&str>) -> String {
        let base = format!("cast-{}", self.workspace_id);

        match service_name {
            Some(name) => format!("{base}-{name}"),
            None => base,
        }
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

    #[test]
    fn linked_worktrees_have_distinct_service_identities() {
        let temp = tempfile::tempdir().expect("create temp directory");
        let primary = temp.path().join("primary");
        let linked = temp.path().join("linked");
        fs::create_dir_all(&primary).expect("create primary worktree");
        let init = Command::new("git")
            .args(["init", "--quiet"])
            .arg(&primary)
            .status()
            .expect("run git init");
        assert!(init.success(), "git init should succeed");
        fs::write(primary.join("README.md"), "service context test").expect("write tracked file");
        let commit = Command::new("git")
            .arg("-C")
            .arg(&primary)
            .args([
                "-c",
                "user.name=Cast Test",
                "-c",
                "user.email=cast@example.invalid",
                "add",
                ".",
            ])
            .status()
            .expect("stage initial file");
        assert!(commit.success(), "git add should succeed");
        let commit = Command::new("git")
            .arg("-C")
            .arg(&primary)
            .args([
                "-c",
                "user.name=Cast Test",
                "-c",
                "user.email=cast@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "initial",
            ])
            .status()
            .expect("create initial commit");
        assert!(commit.success(), "git commit should succeed");
        let add_worktree = Command::new("git")
            .arg("-C")
            .arg(&primary)
            .args(["worktree", "add", "--quiet", "-b", "linked"])
            .arg(&linked)
            .status()
            .expect("create linked worktree");
        assert!(add_worktree.success(), "git worktree add should succeed");

        let primary = ServiceContext::resolve(&primary).expect("resolve primary worktree");
        let linked = ServiceContext::resolve(&linked).expect("resolve linked worktree");

        assert_ne!(primary.worktree_root, linked.worktree_root);
        assert_ne!(primary.workspace_id, linked.workspace_id);
        assert_eq!(primary.git_common_dir, linked.git_common_dir);
        assert!(linked.git_common_dir.is_absolute());
    }

    #[test]
    fn service_container_names_include_worktree_identity_and_optional_name() {
        let context = ServiceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/projects/my-app/.git"),
            relative_cwd: PathBuf::new(),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        };

        assert_eq!(context.container_name(None), "cast-my-app-a1b2c3d4e5f6");
        assert_eq!(
            context.container_name(Some("isolated")),
            "cast-my-app-a1b2c3d4e5f6-isolated"
        );
    }

    #[test]
    fn non_git_directories_are_rejected() {
        let directory = tempfile::tempdir().expect("create temp directory");

        let error = ServiceContext::resolve(directory.path())
            .expect_err("non-Git directory should be rejected");

        assert!(error
            .to_string()
            .contains("Current directory is not in a Git worktree"));
    }

    #[test]
    fn multiplexer_sessions_are_isolated_by_worktree_and_service_name() {
        let context = ServiceContext {
            worktree_root: PathBuf::from("/home/alice/projects/my-app"),
            git_common_dir: PathBuf::from("/home/alice/projects/my-app/.git"),
            relative_cwd: PathBuf::new(),
            workspace_id: "a1b2c3d4e5f6".to_string(),
        };

        assert_eq!(context.multiplexer_session_name(None), "cast-a1b2c3d4e5f6");
        assert_eq!(
            context.multiplexer_session_name(Some("isolated")),
            "cast-a1b2c3d4e5f6-isolated"
        );
    }
}
