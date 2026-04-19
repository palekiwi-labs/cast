use std::fs;
use tempfile::TempDir;

use crate::config::Config;
use crate::nix::dev_image::{get_dockerfile, get_entrypoint, get_image_tag};
use crate::nix::docker::{DockerClient, Result};
use crate::nix::extra_dirs::resolve_extra_dirs;
use crate::user::ResolvedUser;

/// Build the nix dev image locally.
pub fn build_dev<D: DockerClient>(
    docker: &D,
    config: &Config,
    user: &ResolvedUser,
    version: &str,
    force: bool,
    no_cache: bool,
) -> Result<()> {
    let image_tag = get_image_tag(version);

    if !force && docker.image_exists(&image_tag)? {
        println!("Nix dev image already exists: {}", image_tag);
        return Ok(());
    }

    println!("Building nix dev image: {}", image_tag);

    let temp_dir = TempDir::new()?;
    let context_path = temp_dir.path();

    let dockerfile_path = context_path.join("Dockerfile.nix-dev");
    fs::write(&dockerfile_path, get_dockerfile())?;

    let entrypoint_path = context_path.join("entrypoint.sh");
    fs::write(&entrypoint_path, get_entrypoint())?;

    let extra_dirs = resolve_extra_dirs(config, &user.username);
    let uid_str = user.uid.to_string();
    let gid_str = user.gid.to_string();

    let build_args = [
        ("OPENCODE_VERSION", version),
        ("USERNAME", &user.username),
        ("UID", &uid_str),
        ("GID", &gid_str),
        ("EXTRA_DIRS", &extra_dirs),
    ];

    docker.build_image(&image_tag, context_path, &build_args, no_cache)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::user::ResolvedUser;
    use std::cell::RefCell;
    use std::path::Path;

    #[derive(Default)]
    struct MockDocker {
        image_exists_return: bool,
        built_images: RefCell<Vec<(String, Vec<(String, String)>)>>,
        no_cache_calls: RefCell<Vec<bool>>,
    }

    impl crate::nix::docker::DockerClient for MockDocker {
        fn is_container_running(&self, _name: &str) -> crate::nix::docker::Result<bool> {
            Ok(false)
        }

        fn image_exists(&self, _tag: &str) -> crate::nix::docker::Result<bool> {
            Ok(self.image_exists_return)
        }

        fn build_image(
            &self,
            tag: &str,
            _context_path: &Path,
            build_args: &[(&str, &str)],
            no_cache: bool,
        ) -> crate::nix::docker::Result<()> {
            let args: Vec<(String, String)> = build_args
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            self.built_images.borrow_mut().push((tag.to_string(), args));
            self.no_cache_calls.borrow_mut().push(no_cache);
            Ok(())
        }

        fn run_container(
            &self,
            _name: &str,
            _image: &str,
            _volumes: &[&str],
            _env_vars: &[(&str, &str)],
            _detached: bool,
            _remove: bool,
        ) -> crate::nix::docker::Result<()> {
            Ok(())
        }

        fn stop_container(&self, _name: &str) -> crate::nix::docker::Result<()> {
            Ok(())
        }
    }

    fn test_user() -> ResolvedUser {
        ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        }
    }

    #[test]
    fn test_build_dev_skips_when_image_exists_and_not_force() {
        let docker = MockDocker {
            image_exists_return: true,
            ..Default::default()
        };
        let config = Config::default();

        build_dev(&docker, &config, &test_user(), "1.4.7", false, false).unwrap();

        assert!(docker.built_images.borrow().is_empty());
    }

    #[test]
    fn test_build_dev_builds_when_image_exists_but_force() {
        let docker = MockDocker {
            image_exists_return: true,
            ..Default::default()
        };
        let config = Config::default();

        build_dev(&docker, &config, &test_user(), "1.4.7", true, false).unwrap();

        assert_eq!(docker.built_images.borrow().len(), 1);
    }

    #[test]
    fn test_build_dev_builds_with_correct_args() {
        let docker = MockDocker::default();
        let config = Config::default();
        let user = test_user();

        build_dev(&docker, &config, &user, "1.4.7", false, false).unwrap();

        let built = docker.built_images.borrow();
        let (tag, args) = &built[0];

        assert!(tag.starts_with("localhost/ocx:v1.4.7-sha-"));

        let args_map: std::collections::HashMap<_, _> =
            args.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

        assert_eq!(args_map.get("OPENCODE_VERSION"), Some(&"1.4.7"));
        assert_eq!(args_map.get("USERNAME"), Some(&"alice"));
        assert_eq!(args_map.get("UID"), Some(&"1000"));
        assert_eq!(args_map.get("GID"), Some(&"1000"));
        assert_eq!(args_map.get("EXTRA_DIRS"), Some(&""));
    }

    #[test]
    fn test_build_dev_propagates_no_cache() {
        let docker = MockDocker::default();
        let config = Config::default();
        let user = test_user();

        // Call with no_cache = true
        build_dev(&docker, &config, &user, "1.4.7", false, true).unwrap();

        assert_eq!(docker.no_cache_calls.borrow().len(), 1);
        assert_eq!(docker.no_cache_calls.borrow()[0], true);
    }
}
