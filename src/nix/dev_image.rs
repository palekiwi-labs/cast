use super::image_hash::compute_hash;

const DOCKERFILE: &str = include_str!("../../assets/nix/Dockerfile.nix-dev");
const ENTRYPOINT: &str = include_str!("../../assets/nix/entrypoint-dev.sh");
const IMAGE_BASE: &str = "localhost/ocx";

/// Get the full image tag for the nix dev container.
///
/// Format: `localhost/ocx:v{version}-sha-{hash}`
pub fn get_image_tag(version: &str) -> String {
    let hash = compute_hash(DOCKERFILE, ENTRYPOINT);
    format!("{}:v{}-sha-{}", IMAGE_BASE, version, hash)
}

/// Get the embedded Dockerfile content for the nix dev image.
pub fn get_dockerfile() -> &'static str {
    DOCKERFILE
}

/// Get the embedded entrypoint script content for the nix dev image.
pub fn get_entrypoint() -> &'static str {
    ENTRYPOINT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_image_tag_format() {
        let tag = get_image_tag("1.4.7");

        assert!(tag.starts_with("localhost/ocx:v1.4.7-sha-"));
        assert_eq!(tag.len(), "localhost/ocx:v1.4.7-sha-".len() + 8);
    }

    #[test]
    fn test_get_image_tag_hash_is_hex() {
        let tag = get_image_tag("1.0.0");
        let hash = tag.split("-sha-").nth(1).unwrap();

        assert_eq!(hash.len(), 8);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_get_dockerfile_is_not_empty() {
        assert!(!get_dockerfile().is_empty());
    }

    #[test]
    fn test_get_dockerfile_has_correct_base_image() {
        assert!(get_dockerfile().contains("FROM debian:trixie-slim"));
    }

    #[test]
    fn test_get_entrypoint_is_not_empty() {
        assert!(!get_entrypoint().is_empty());
    }
}
