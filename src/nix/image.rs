/// Embedded Dockerfile content for the nix daemon image
const DOCKERFILE: &str = include_str!("../../assets/nix/Dockerfile.nix-daemon");

/// Base name for the nix daemon image
const IMAGE_BASE: &str = "localhost/ocx-nix-daemon";

/// Get the full image tag for the nix daemon container
///
/// Format: `localhost/ocx-nix-daemon:v<version>`
pub fn get_image_tag() -> String {
    format!("{}:v{}", IMAGE_BASE, env!("CARGO_PKG_VERSION"))
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
        let tag = get_image_tag();

        assert!(
            tag.starts_with("localhost/ocx-nix-daemon:v"),
            "Image tag should have correct prefix"
        );
    }

    #[test]
    fn test_get_dockerfile_not_empty() {
        let dockerfile = get_dockerfile();
        assert!(!dockerfile.is_empty(), "Dockerfile should not be empty");
        assert!(
            dockerfile.contains("FROM"),
            "Dockerfile should contain FROM instruction"
        );
    }
}
