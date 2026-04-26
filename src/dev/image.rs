const DOCKERFILE: &str = include_str!("../../assets/nix/Dockerfile.nix-dev");
const IMAGE_BASE: &str = "localhost/ocx";

/// Get the full image tag for the nix dev container.
///
/// Format: `localhost/ocx:v{ocx_version}-opencode-{opencode_version}`
pub fn get_image_tag(opencode_version: &str) -> String {
    format!(
        "{}:v{}-opencode-{}",
        IMAGE_BASE,
        env!("CARGO_PKG_VERSION"),
        opencode_version
    )
}

/// Get the embedded Dockerfile content for the nix dev image.
pub fn get_dockerfile() -> &'static str {
    DOCKERFILE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_image_tag_format() {
        let tag = get_image_tag("1.4.7");

        assert!(tag.starts_with("localhost/ocx:v"));
        assert!(tag.contains("-opencode-1.4.7"));
    }

    #[test]
    fn test_get_dockerfile_is_not_empty() {
        assert!(!get_dockerfile().is_empty());
    }

    #[test]
    fn test_get_dockerfile_has_correct_base_image() {
        assert!(get_dockerfile().contains("FROM debian:trixie-slim"));
    }
}
