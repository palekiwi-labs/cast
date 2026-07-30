use crate::config::Config;

/// Generate the nix.conf content based on the provided configuration.
///
/// This configuration is injected into the nix daemon container via
/// the NIX_CONFIG environment variable.
pub fn generate_nix_conf(config: &Config) -> String {
    let mut conf = String::new();

    // Base features required for flakes
    conf.push_str("experimental-features = nix-command flakes\n");

    // Only root (the daemon itself) is trusted. The dev container user is
    // merely *allowed* to connect to the daemon socket, not privilege-elevated.
    // Substituters and public keys are baked into this server-side config, so
    // a non-trusted user still benefits from the configured caches without
    // being able to override them or import unsigned paths.
    conf.push_str("trusted-users = root\n");
    conf.push_str("allowed-users = *\n");

    // Build substituters list
    let mut substituters = vec!["https://cache.nixos.org".to_string()];
    substituters.extend(config.nix_extra_substituters.iter().cloned());

    conf.push_str(&format!("substituters = {}\n", substituters.join(" ")));

    // Make sure all substituters are trusted, in both URL spellings.
    //
    // A non-trusted client forwards its own `substituters` value to the daemon,
    // which accepts only entries present in `trusted-substituters` (union
    // `substituters`), compared as exact strings. Nix compensates in one
    // direction only -- it retries a client value with a trailing slash
    // appended -- so a client value that already ends in `/` never matches a
    // trusted entry that does not. Nix's own built-in default is
    // `https://cache.nixos.org/`, which is exactly that case. Publishing both
    // spellings of every substituter makes the match direction-agnostic.
    let trusted_substituters: Vec<String> = substituters
        .iter()
        .flat_map(|s| {
            let variant = match s.strip_suffix('/') {
                Some(base) => base.to_string(),
                None => format!("{s}/"),
            };
            [s.clone(), variant]
        })
        .collect();

    conf.push_str(&format!(
        "trusted-substituters = {}\n",
        trusted_substituters.join(" ")
    ));

    // Build trusted public keys list
    let mut trusted_keys =
        vec!["cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=".to_string()];
    trusted_keys.extend(config.nix_extra_trusted_public_keys.iter().cloned());

    conf.push_str(&format!(
        "trusted-public-keys = {}\n",
        trusted_keys.join(" ")
    ));

    conf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_nix_conf_minimal() {
        let config = Config::default();
        let conf = generate_nix_conf(&config);

        assert!(conf.contains("experimental-features = nix-command flakes"));
        assert!(conf.contains("trusted-users = root\n"));
        assert!(conf.contains("allowed-users = *\n"));
        assert!(conf.contains("substituters = https://cache.nixos.org\n"));
        assert!(
            conf.contains(
                "trusted-substituters = https://cache.nixos.org https://cache.nixos.org/\n"
            )
        );
        assert!(conf.contains(
            "trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=\n"
        ));
    }

    #[test]
    fn test_generate_nix_conf_with_extra_substituters() {
        let config = Config {
            nix_extra_substituters: vec![
                "https://cache.myorg.com".to_string(),
                "https://nix-community.cachix.org".to_string(),
            ],
            ..Config::default()
        };

        let conf = generate_nix_conf(&config);

        let expected_substituters = "substituters = https://cache.nixos.org https://cache.myorg.com https://nix-community.cachix.org\n";
        let expected_trusted = "trusted-substituters = \
             https://cache.nixos.org https://cache.nixos.org/ \
             https://cache.myorg.com https://cache.myorg.com/ \
             https://nix-community.cachix.org https://nix-community.cachix.org/\n";

        assert!(conf.contains(expected_substituters));
        assert!(conf.contains(expected_trusted));
    }

    #[test]
    fn trusted_substituters_include_trailing_slash_variants() {
        let config = Config {
            nix_extra_substituters: vec!["https://cache.numtide.com".to_string()],
            ..Config::default()
        };

        let conf = generate_nix_conf(&config);

        // Nix's built-in client default is `https://cache.nixos.org/` (with a
        // trailing slash). The daemon matches client-forwarded substituters
        // against `trusted-substituters` by exact string and only compensates
        // one way, so both spellings must be published or the client value is
        // rejected with "ignoring untrusted substituter".
        assert!(
            conf.contains(
                "trusted-substituters = https://cache.nixos.org https://cache.nixos.org/ \
                 https://cache.numtide.com https://cache.numtide.com/\n"
            ),
            "trusted-substituters must list both slash variants, got:\n{conf}"
        );
    }

    #[test]
    fn trusted_substituters_variant_of_slashed_entry_drops_the_slash() {
        let config = Config {
            nix_extra_substituters: vec!["https://cache.myorg.com/".to_string()],
            ..Config::default()
        };

        let conf = generate_nix_conf(&config);

        assert!(
            conf.contains("https://cache.myorg.com/ https://cache.myorg.com\n"),
            "a slashed substituter must also be trusted unslashed, got:\n{conf}"
        );
    }

    #[test]
    fn test_generate_nix_conf_with_extra_keys() {
        let config = Config {
            nix_extra_trusted_public_keys: vec![
                "myorg.com-1:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx=".to_string(),
            ],
            ..Config::default()
        };

        let conf = generate_nix_conf(&config);

        let expected_keys = "trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= myorg.com-1:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx=\n";

        assert!(conf.contains(expected_keys));
    }
}
