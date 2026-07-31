use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// The embedded global-flake template. Written to a new user's
/// `~/.config/cast/nix/flake.nix` when none exists, so `cast run <agent>`
/// works with no manual setup. Harnesses are provided by its named devShells.
const GLOBAL_FLAKE_TEMPLATE: &str = include_str!("../../assets/global-flake-template/flake.nix");

/// Directory (relative to the host home) holding the global cast flake.
const GLOBAL_FLAKE_SUBDIR: &str = ".config/cast/nix";

/// Scaffold the global cast flake from the embedded template if it is absent.
///
/// Given the host home directory, ensures
/// `<home>/.config/cast/nix/flake.nix` exists. If it is missing, the directory
/// tree is created and the embedded template is written. Returns `Ok(true)`
/// when a new flake was scaffolded and `Ok(false)` when one was already
/// present (left untouched).
pub fn scaffold_if_missing(home_dir: &Path) -> Result<bool> {
    let flake_dir = home_dir.join(GLOBAL_FLAKE_SUBDIR);
    let flake_path = flake_dir.join("flake.nix");

    if flake_path.exists() {
        return Ok(false);
    }

    fs::create_dir_all(&flake_dir)
        .with_context(|| format!("creating global flake dir {}", flake_dir.display()))?;
    fs::write(&flake_path, GLOBAL_FLAKE_TEMPLATE)
        .with_context(|| format!("writing global flake {}", flake_path.display()))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn scaffolds_flake_when_missing() {
        let home = TempDir::new().unwrap();
        let created = scaffold_if_missing(home.path()).unwrap();

        assert!(created, "expected a fresh scaffold to report true");
        let flake = home.path().join(".config/cast/nix/flake.nix");
        assert!(flake.exists(), "flake.nix should have been written");

        let contents = fs::read_to_string(&flake).unwrap();
        assert_eq!(contents, GLOBAL_FLAKE_TEMPLATE);
    }

    #[test]
    fn leaves_existing_flake_untouched() {
        let home = TempDir::new().unwrap();
        let flake_dir = home.path().join(".config/cast/nix");
        fs::create_dir_all(&flake_dir).unwrap();
        let flake = flake_dir.join("flake.nix");
        fs::write(&flake, "# user's own flake").unwrap();

        let created = scaffold_if_missing(home.path()).unwrap();

        assert!(!created, "expected an existing flake to report false");
        let contents = fs::read_to_string(&flake).unwrap();
        assert_eq!(
            contents, "# user's own flake",
            "existing flake must not be overwritten"
        );
    }

    #[test]
    fn template_defines_expected_shells() {
        // Named per-harness shells plus a universal shell.
        assert!(GLOBAL_FLAKE_TEMPLATE.contains("opencode ="));
        assert!(GLOBAL_FLAKE_TEMPLATE.contains("pi ="));
        assert!(GLOBAL_FLAKE_TEMPLATE.contains("claudecode ="));
        assert!(GLOBAL_FLAKE_TEMPLATE.contains("universal ="));
        assert!(GLOBAL_FLAKE_TEMPLATE.contains("default ="));

        assert!(GLOBAL_FLAKE_TEMPLATE.contains("llm-agents"));
    }

    #[test]
    fn template_declares_no_nix_config() {
        // A `nixConfig` block makes nix prompt for approval on every devshell
        // entry -- a prompt the non-trusted dev user cannot usefully answer,
        // because the daemon rejects the settings either way. Caches belong in
        // the seeded `cast.json`, which reaches the daemon server-side.
        assert!(
            !GLOBAL_FLAKE_TEMPLATE.contains("nixConfig"),
            "template must not declare a nixConfig block"
        );
    }
}
