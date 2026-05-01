use crate::config::Config;
use crate::user::ResolvedUser;

/// Resolve the final command vector to pass to the container.
///
/// If `user_flake_present` is true, the base command is wrapped inside
/// `nix develop <flake_dir> -c <base>` so the container uses the user's
/// personal Nix flake environment.
pub fn resolve_opencode_command(
    cfg: &Config,
    user: &ResolvedUser,
    user_flake_present: bool,
) -> Vec<String> {
    let base = cfg.opencode_command.clone();

    if user_flake_present {
        let flake_dir = format!("/home/{}/.config/cast/nix", user.username);
        let mut cmd = vec![
            "nix".to_string(),
            "develop".to_string(),
            flake_dir,
            "-c".to_string(),
        ];
        cmd.extend(base);
        cmd
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::user::ResolvedUser;

    fn alice() -> ResolvedUser {
        ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        }
    }

    #[test]
    fn test_default_cmd_no_flake() {
        let cfg = Config::default();
        let cmd = resolve_opencode_command(&cfg, &alice(), false);
        assert_eq!(cmd, vec!["opencode"]);
    }

    #[test]
    fn test_default_cmd_with_flake() {
        let cfg = Config::default();
        let cmd = resolve_opencode_command(&cfg, &alice(), true);
        assert_eq!(
            cmd,
            vec![
                "nix",
                "develop",
                "/home/alice/.config/cast/nix",
                "-c",
                "opencode",
            ]
        );
    }
}
