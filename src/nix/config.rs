use crate::config::Config;

/// Generate the nix.conf content based on the provided configuration.
///
/// This configuration is injected into the nix daemon container via
/// the NIX_CONFIG environment variable.
pub fn generate_nix_conf(config: &Config) -> String {
    let mut conf = String::new();

    // Base features required for flakes
    conf.push_str("experimental-features = nix-command flakes\n");

    // CRITICAL: Allow non-root users (like the dev container user) to connect to the daemon
    conf.push_str("trusted-users = root *\n");

    // Build substituters list
    let mut substituters = vec!["https://cache.nixos.org".to_string()];
    substituters.extend(config.nix_extra_substituters.iter().cloned());

    conf.push_str(&format!("substituters = {}\n", substituters.join(" ")));

    // Make sure all substituters are trusted
    conf.push_str(&format!(
        "trusted-substituters = {}\n",
        substituters.join(" ")
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
        assert!(conf.contains("trusted-users = root *"));
        assert!(conf.contains("substituters = https://cache.nixos.org\n"));
        assert!(conf.contains("trusted-substituters = https://cache.nixos.org\n"));
        assert!(conf.contains("trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=\n"));
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
        let expected_trusted = "trusted-substituters = https://cache.nixos.org https://cache.myorg.com https://nix-community.cachix.org\n";

        assert!(conf.contains(expected_substituters));
        assert!(conf.contains(expected_trusted));
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
