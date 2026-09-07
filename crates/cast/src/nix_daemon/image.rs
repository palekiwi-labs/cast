/// Embedded Dockerfile content for the nix daemon image
const DOCKERFILE: &str = include_str!("../../assets/Dockerfile.nix-daemon");

/// Base name for the nix daemon image
const IMAGE_BASE: &str = "localhost/cast-nix-daemon";

const CAST_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Placeholder in the Dockerfile template replaced by the configured
/// Nix version when the build context is written.
const NIX_VERSION_PLACEHOLDER: &str = "${NIX_VERSION}";

/// Get the image tag for a configured Nix daemon generation.
///
/// Format: `localhost/cast-nix-daemon-<nix_version>:<cast_version>`
pub fn get_generation_image_tag(nix_version: &str) -> String {
    format!("{IMAGE_BASE}-{nix_version}:{CAST_VERSION}")
}

/// Render the Dockerfile for a configured Nix daemon generation.
///
/// The concrete version is baked into the base image reference so the
/// document handed to Docker is always fully resolved: no pre-FROM
/// build argument with an empty default (BuildKit's
/// InvalidDefaultArgInFrom check) and no silent fallback tag.
pub fn render_dockerfile(nix_version: &str) -> String {
    DOCKERFILE.replace(NIX_VERSION_PLACEHOLDER, nix_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changing_nix_version_changes_every_generation_resource() {
        use crate::config::Config;

        let first = Config {
            nix_version: "2.34.5".to_string(),
            ..Config::default()
        };
        let second = Config {
            nix_version: "2.34.6".to_string(),
            ..Config::default()
        };

        assert_ne!(
            get_generation_image_tag(&first.nix_version),
            get_generation_image_tag(&second.nix_version)
        );
        assert_ne!(
            first.effective_nix_daemon_container_name(),
            second.effective_nix_daemon_container_name()
        );
        assert_ne!(
            first.effective_nix_volume_name(),
            second.effective_nix_volume_name()
        );
    }

    #[test]
    fn template_selects_nix_base_via_placeholder() {
        assert!(DOCKERFILE.contains("FROM nixos/nix:${NIX_VERSION}"));
        assert!(!DOCKERFILE.contains("ARG NIX_VERSION"));
        assert!(!DOCKERFILE.contains(include_str!("../../assets/nix-version").trim()));
    }

    #[test]
    fn rendered_dockerfile_pins_configured_nix_version() {
        let dockerfile = render_dockerfile("2.34.6");

        assert!(dockerfile.contains("FROM nixos/nix:2.34.6"));
        assert!(!dockerfile.contains("${NIX_VERSION}"));
    }
}
