use crate::config::Config;

/// Resolve the EXTRA_DIRS build argument for the nix dev image.
///
/// Filters `config.extra_data_volumes` to entries of type `"volume"`, expands
/// a leading `~/` to `/home/{username}/`, and joins all resolved paths with a
/// single space. Returns an empty string when there are no matching entries.
pub fn resolve_extra_dirs(config: &Config, username: &str) -> String {
    let mut dirs: Vec<String> = config
        .extra_data_volumes
        .values()
        .filter(|v| v.volume_type == "volume")
        .map(|v| {
            if let Some(rest) = v.target.strip_prefix("~/") {
                format!("/home/{}/{}", username, rest)
            } else {
                v.target.clone()
            }
        })
        .collect();

    dirs.sort();
    dirs.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, VolumeConfig};

    fn volume(target: &str, volume_type: &str) -> VolumeConfig {
        VolumeConfig {
            target: target.to_string(),
            volume_type: volume_type.to_string(),
            source: None,
            mode: "rw".to_string(),
        }
    }

    #[test]
    fn test_empty_extra_data_volumes_returns_empty_string() {
        let config = Config::default();
        assert_eq!(resolve_extra_dirs(&config, "alice"), "");
    }

    #[test]
    fn test_non_volume_type_is_excluded() {
        let mut config = Config::default();
        config
            .extra_data_volumes
            .insert("bind-mount".to_string(), volume("/data", "bind"));

        assert_eq!(resolve_extra_dirs(&config, "alice"), "");
    }

    #[test]
    fn test_volume_type_with_plain_target_is_included() {
        let mut config = Config::default();
        config
            .extra_data_volumes
            .insert("cargo".to_string(), volume("/home/alice/.cargo", "volume"));

        assert_eq!(resolve_extra_dirs(&config, "alice"), "/home/alice/.cargo");
    }

    #[test]
    fn test_tilde_prefix_is_expanded_to_home_dir() {
        let mut config = Config::default();
        config
            .extra_data_volumes
            .insert("cargo".to_string(), volume("~/.cargo", "volume"));

        assert_eq!(resolve_extra_dirs(&config, "alice"), "/home/alice/.cargo");
    }

    #[test]
    fn test_multiple_volume_entries_joined_with_space() {
        let mut config = Config::default();
        config
            .extra_data_volumes
            .insert("cargo".to_string(), volume("~/.cargo", "volume"));
        config
            .extra_data_volumes
            .insert("local".to_string(), volume("~/.local", "volume"));

        let result = resolve_extra_dirs(&config, "alice");
        assert_eq!(result, "/home/alice/.cargo /home/alice/.local");
    }
}
