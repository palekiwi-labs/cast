use std::path::PathBuf;

/// Resolve the config base directory as `$HOME/.config`.
///
/// Prefer this over [`dirs::config_dir`] because on macOS `dirs::config_dir`
/// returns `~/Library/Application Support`, which does not map 1-to-1 to paths
/// inside a Linux container. Both platforms store cast / opencode config under
/// `~/.config`, so we always derive the path from `$HOME`.
///
/// Note: `$XDG_CONFIG_HOME` is intentionally not respected. Container agents
/// always read from `~/.config`, so the host path must match that layout
/// regardless of any host-specific XDG overrides.
///
/// Returns `None` if the home directory cannot be determined.
pub fn home_config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_config_dir_ends_with_dot_config() {
        let path = home_config_dir().expect("home dir should be resolvable in test environment");
        assert!(
            path.ends_with(".config"),
            "expected path to end with .config, got: {}",
            path.display()
        );
    }

    #[test]
    fn test_home_config_dir_is_under_home() {
        let home = dirs::home_dir().expect("home dir should be resolvable");
        let config = home_config_dir().expect("config dir should be resolvable");
        assert_eq!(config, home.join(".config"));
    }
}
