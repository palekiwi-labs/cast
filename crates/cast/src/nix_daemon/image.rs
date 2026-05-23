/// Embedded Dockerfile content for the nix daemon image
const DOCKERFILE: &str = include_str!("../../assets/Dockerfile.nix-daemon");

/// Base name for the nix daemon image
const IMAGE_BASE: &str = "localhost/cast-nix-daemon";

const CAST_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get the full image tag for the nix daemon container
///
/// Format: `localhost/cast-nix-daemon:<version>`
pub fn get_image_tag() -> String {
    format!("{}:{}", IMAGE_BASE, CAST_VERSION)
}

/// Get the embedded Dockerfile content
pub fn get_dockerfile() -> &'static str {
    DOCKERFILE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_image_tag_format() {
        assert_eq!(
            get_image_tag(),
            format!("localhost/cast-nix-daemon:{}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn test_get_dockerfile_not_empty() {
        assert!(get_dockerfile().contains("FROM"));
    }
}
