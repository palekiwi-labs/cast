const DOCKERFILE: &str = include_str!("../../assets/Dockerfile.dev");
const IMAGE_BASE: &str = "localhost/ocx";
const OCX_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get the full image tag: `localhost/ocx:{ocx_version}-opencode-{opencode_version}`
pub fn get_image_tag(opencode_version: &str) -> String {
    format!(
        "{}:{}-opencode-{}",
        IMAGE_BASE, OCX_VERSION, opencode_version
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
        assert_eq!(
            get_image_tag("1.4.7"),
            format!(
                "localhost/ocx:{}-opencode-1.4.7",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    #[test]
    fn test_get_dockerfile_has_correct_base_image() {
        assert!(get_dockerfile().contains("FROM debian:trixie-slim"));
    }
}
