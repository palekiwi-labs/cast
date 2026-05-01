use crate::config::Config;
use crate::user::ResolvedUser;

/// Resolve the final command vector to pass to the container.
///
/// If `user_flake_present` is true, the base command is wrapped inside
/// `nix develop <flake_dir> -c <base>` so the container uses the user's
/// personal Nix flake environment.
pub fn resolve_pi_command(
    _cfg: &Config,
    user: &ResolvedUser,
    user_flake_present: bool,
) -> Vec<String> {
    let base = vec!["pi".to_string()];

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
    use crate::user::ResolvedUser;

    #[test]
    fn test_resolve_pi_command_no_flake() {
        let cfg = Config::default();
        let user = ResolvedUser {
            uid: 1000,
            gid: 1000,
            username: "testuser".to_string(),
        };

        assert_eq!(
            resolve_pi_command(&cfg, &user, false),
            vec!["pi".to_string()]
        );
    }

    #[test]
    fn test_resolve_pi_command_with_flake() {
        let cfg = Config::default();
        let user = ResolvedUser {
            uid: 1000,
            gid: 1000,
            username: "testuser".to_string(),
        };

        assert_eq!(
            resolve_pi_command(&cfg, &user, true),
            vec![
                "nix".to_string(),
                "develop".to_string(),
                "/home/testuser/.config/cast/nix".to_string(),
                "-c".to_string(),
                "pi".to_string()
            ]
        );
    }
}
