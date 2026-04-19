use std::path::{Path, PathBuf};

/// Expand a leading `~/` in `path` to the provided `home_dir`.
///
/// If `home_dir` is `None` or `path` does not start with `~/`, the path is
/// returned as-is. This is the host-side equivalent of the container-side
/// tilde expansion in `nix::extra_dirs`, which uses a username string instead.
pub fn expand_tilde(path: &str, home_dir: Option<&Path>) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = home_dir {
            return home.join(rest);
        }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tilde_expanded_with_home() {
        let home = Path::new("/home/alice");
        assert_eq!(
            expand_tilde("~/foo/bar", Some(home)),
            PathBuf::from("/home/alice/foo/bar"),
        );
    }

    #[test]
    fn test_tilde_expanded_to_home_root() {
        let home = Path::new("/home/alice");
        // "~/" with nothing after — joins to the home dir itself
        assert_eq!(
            expand_tilde("~/", Some(home)),
            PathBuf::from("/home/alice/"),
        );
    }

    #[test]
    fn test_tilde_no_home_dir_fallback() {
        assert_eq!(expand_tilde("~/foo", None), PathBuf::from("~/foo"),);
    }

    #[test]
    fn test_no_tilde_absolute_path() {
        let home = Path::new("/home/alice");
        assert_eq!(
            expand_tilde("/absolute/path", Some(home)),
            PathBuf::from("/absolute/path"),
        );
    }

    #[test]
    fn test_no_tilde_relative_path() {
        let home = Path::new("/home/alice");
        assert_eq!(
            expand_tilde("relative/path", Some(home)),
            PathBuf::from("relative/path"),
        );
    }
}
