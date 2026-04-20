use sha2::{Digest, Sha256};

/// Compute SHA256 hash of two content strings (Dockerfile + entrypoint).
///
/// Returns the first 8 characters of the hex-encoded hash.
pub fn compute_hash(dockerfile: &str, entrypoint: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(dockerfile.as_bytes());
    hasher.update(entrypoint.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash_is_deterministic() {
        let dockerfile = "FROM nixos/nix:latest\nRUN echo hello";
        let entrypoint = "#!/bin/sh\necho world";

        let hash1 = compute_hash(dockerfile, entrypoint);
        let hash2 = compute_hash(dockerfile, entrypoint);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_is_eight_chars() {
        let hash = compute_hash("FROM debian:trixie-slim", "#!/bin/sh");

        assert_eq!(hash.len(), 8);
    }

    #[test]
    fn test_compute_hash_changes_with_dockerfile() {
        let entrypoint = "#!/bin/sh";

        let hash1 = compute_hash("FROM nixos/nix:latest", entrypoint);
        let hash2 = compute_hash("FROM nixos/nix:2.18", entrypoint);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_changes_with_entrypoint() {
        let dockerfile = "FROM debian:trixie-slim";

        let hash1 = compute_hash(dockerfile, "#!/bin/sh\necho hello");
        let hash2 = compute_hash(dockerfile, "#!/bin/sh\necho goodbye");

        assert_ne!(hash1, hash2);
    }
}
