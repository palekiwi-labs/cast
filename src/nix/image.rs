use super::image_hash::compute_hash;

/// Embedded Dockerfile content for the nix daemon image
const DOCKERFILE: &str = include_str!("../../assets/nix/Dockerfile.nix-daemon");

/// Embedded entrypoint script content
const ENTRYPOINT: &str = include_str!("../../assets/nix/entrypoint.sh");

/// Base name for the nix daemon image
const IMAGE_BASE: &str = "localhost/ocx-nix-daemon";

/// Get the full image tag for the nix daemon container
///
/// Format: `localhost/ocx-nix-daemon:sha-<hash>`
///
/// The hash is computed from the embedded Dockerfile and entrypoint script,
/// ensuring that any changes to these files will result in a new image tag.
pub fn get_image_tag() -> String {
    let hash = compute_hash(DOCKERFILE, ENTRYPOINT);
    format!("{}:sha-{}", IMAGE_BASE, hash)
}

/// Get the embedded Dockerfile content
pub fn get_dockerfile() -> &'static str {
    DOCKERFILE
}

/// Get the embedded entrypoint script content
pub fn get_entrypoint() -> &'static str {
    ENTRYPOINT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash_deterministic() {
        let dockerfile = "FROM nixos/nix:latest\nRUN echo hello";
        let entrypoint = "#!/bin/sh\necho world";

        let hash1 = compute_hash(dockerfile, entrypoint);
        let hash2 = compute_hash(dockerfile, entrypoint);

        assert_eq!(hash1, hash2, "Hash should be deterministic");
    }

    #[test]
    fn test_compute_hash_length() {
        let dockerfile = "FROM nixos/nix:latest";
        let entrypoint = "#!/bin/sh";

        let hash = compute_hash(dockerfile, entrypoint);

        assert_eq!(hash.len(), 8, "Hash should be 8 characters");
    }

    #[test]
    fn test_compute_hash_changes_with_content() {
        let dockerfile1 = "FROM nixos/nix:latest";
        let dockerfile2 = "FROM nixos/nix:2.18";
        let entrypoint = "#!/bin/sh";

        let hash1 = compute_hash(dockerfile1, entrypoint);
        let hash2 = compute_hash(dockerfile2, entrypoint);

        assert_ne!(
            hash1, hash2,
            "Hash should change when Dockerfile content changes"
        );
    }

    #[test]
    fn test_compute_hash_changes_with_entrypoint() {
        let dockerfile = "FROM nixos/nix:latest";
        let entrypoint1 = "#!/bin/sh\necho hello";
        let entrypoint2 = "#!/bin/sh\necho goodbye";

        let hash1 = compute_hash(dockerfile, entrypoint1);
        let hash2 = compute_hash(dockerfile, entrypoint2);

        assert_ne!(
            hash1, hash2,
            "Hash should change when entrypoint content changes"
        );
    }

    #[test]
    fn test_get_image_tag_format() {
        let tag = get_image_tag();

        assert!(
            tag.starts_with("localhost/ocx-nix-daemon:sha-"),
            "Image tag should have correct prefix"
        );
        assert_eq!(
            tag.len(),
            "localhost/ocx-nix-daemon:sha-".len() + 8,
            "Image tag should have 8-character hash suffix"
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

    #[test]
    fn test_get_entrypoint_not_empty() {
        let entrypoint = get_entrypoint();
        assert!(!entrypoint.is_empty(), "Entrypoint should not be empty");
        assert!(
            entrypoint.contains("#!/bin/sh"),
            "Entrypoint should have shebang"
        );
    }

    #[test]
    fn test_embedded_assets_affect_tag() {
        // This test verifies that the actual embedded content is used
        let tag = get_image_tag();
        let manual_hash = compute_hash(DOCKERFILE, ENTRYPOINT);

        assert!(
            tag.ends_with(&manual_hash),
            "Image tag should use hash of embedded assets"
        );
    }
}
