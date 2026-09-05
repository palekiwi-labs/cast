/// Embedded Dockerfile content for the nix daemon image
const DOCKERFILE: &str = include_str!("../../assets/Dockerfile.nix-daemon");

/// Base name for the nix daemon image
const IMAGE_BASE: &str = "localhost/cast-nix-daemon";

const CAST_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get the image tag for a configured Nix daemon generation.
pub fn get_generation_image_tag(nix_version: &str) -> String {
    format!("{IMAGE_BASE}-{nix_version}:{CAST_VERSION}")
}

/// Get the embedded Dockerfile content
pub fn get_dockerfile() -> &'static str {
    DOCKERFILE
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
    fn test_get_dockerfile_not_empty() {
        assert!(get_dockerfile().contains("FROM"));
    }

    #[test]
    fn dockerfile_selects_nix_base_with_build_argument() {
        let dockerfile = get_dockerfile();

        assert!(dockerfile.contains("ARG NIX_VERSION"));
        assert!(dockerfile.contains("FROM nixos/nix:${NIX_VERSION}"));
        assert!(!dockerfile.contains(include_str!("../../assets/nix-version").trim()));
    }
}
