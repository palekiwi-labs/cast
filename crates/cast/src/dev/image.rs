use std::fs;
use tempfile::TempDir;

use crate::config::Config;
use crate::dev::extra_dirs::resolve_extra_dirs;
use crate::docker::BuildOptions;
use crate::docker::args;
use crate::docker::client::DockerClient;
use crate::user::ResolvedUser;
use anyhow::Result;
use tracing::info;

const IMAGE_BASE: &str = "localhost/cast";
const CAST_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The single, harness-free dev image Dockerfile. Harnesses are provided by the
/// sandbox Nix devShell selected at run time, not baked into the image.
const DEV_DOCKERFILE: &str = include_str!("../../assets/Dockerfile.dev");

/// Get the tag for the single dev image: `localhost/cast:{cast_version}`.
pub fn image_tag() -> String {
    format!("{}:{}", IMAGE_BASE, CAST_VERSION)
}

/// Ensure the single dev image exists locally, building it if necessary.
pub fn ensure_dev_image(
    docker: &DockerClient,
    config: &Config,
    user: &ResolvedUser,
    opts: BuildOptions,
) -> Result<()> {
    let image_tag = image_tag();

    if !opts.force && docker.image_exists(&image_tag)? {
        info!(%image_tag, "image exists, skipping build");
        // info! writes to the file log; eprintln! writes to the console.
        // Status messages go to stderr so stdout remains clean for
        // programmatic consumers (e.g. `cast run --headless --format json`).
        eprintln!("dev image already exists: {}", image_tag);
        if opts.no_cache {
            eprintln!(
                "Hint: You passed --no-cache. If you want to force a rebuild of the existing image, use --force."
            );
        }
        return Ok(());
    }

    info!(%image_tag, "image not found or force rebuild, building");
    eprintln!("Building dev image: {}", image_tag);

    let temp_dir = TempDir::new()?;
    let context_path = temp_dir.path();

    let dockerfile_path = context_path.join("Dockerfile");
    fs::write(&dockerfile_path, DEV_DOCKERFILE)?;

    let extra_dirs = resolve_extra_dirs(config, &user.username);
    let uid_str = user.uid.to_string();
    let gid_str = user.gid.to_string();

    let build_args = [
        ("USERNAME", user.username.as_str()),
        ("UID", uid_str.as_str()),
        ("GID", gid_str.as_str()),
        ("EXTRA_DIRS", extra_dirs.as_str()),
    ];

    let docker_build_args =
        args::build_docker_build_args(&image_tag, context_path, &build_args, opts.no_cache);
    docker.stream_command(docker_build_args)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spike_image_tag_is_isolated_from_releases() {
        assert_eq!(image_tag(), "localhost/cast:0.3.0-herdr-spike");
    }

    #[test]
    fn dev_dockerfile_has_no_harness_installs() {
        // The single image must not bake in any harness binary.
        assert!(!DEV_DOCKERFILE.contains("anomalyco/opencode"));
        assert!(!DEV_DOCKERFILE.contains("badlogic/pi-mono"));
        assert!(!DEV_DOCKERFILE.contains("npm install"));
        assert!(!DEV_DOCKERFILE.contains("claude-code"));
    }

    #[test]
    fn dev_dockerfile_disables_flake_config() {
        // Under the hardened daemon (trusted-users = root) the dev user is
        // non-trusted, so flake-forwarded substituters/keys are rejected
        // regardless. Set accept-flake-config = false explicitly to
        // deterministically suppress the prompt path. Caches are provisioned
        // daemon-side via cast.json instead.
        assert!(
            DEV_DOCKERFILE.contains("accept-flake-config = false"),
            "system nix.conf must disable accept-flake-config; caches are \
             provisioned daemon-side via cast.json"
        );
    }

    #[test]
    fn dev_dockerfile_creates_union_config_dirs() {
        // Union of all supported harness config dirs is created.
        assert!(DEV_DOCKERFILE.contains(".claude"));
        assert!(DEV_DOCKERFILE.contains(".pi"));
        assert!(DEV_DOCKERFILE.contains(".config"));
        assert!(DEV_DOCKERFILE.contains(".cache"));
        assert!(DEV_DOCKERFILE.contains(".local"));
    }

    #[test]
    fn dev_dockerfile_creates_and_chowns_cross_harness_agents_dir() {
        // The vendor-neutral cross-harness .agents dir must be pre-created and
        // chowned so a bind-mounted ~/.agents lands on a user-owned directory.
        // Exactly two occurrences: one in mkdir, one in chown.
        assert_eq!(
            DEV_DOCKERFILE.matches("/home/${USERNAME}/.agents").count(),
            2,
            "Dockerfile.dev must reference /home/${{USERNAME}}/.agents in both mkdir and chown"
        );
    }

    #[test]
    fn dev_dockerfile_configures_git_safe_directory() {
        assert!(DEV_DOCKERFILE.contains(r#"git config --system safe.directory "*""#));
    }
}
