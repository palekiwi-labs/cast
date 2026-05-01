use crate::config::Config;
use crate::user::ResolvedUser;

/// Resolve the final command vector to pass to the container.
pub fn resolve_pi_command(_cfg: &Config, _user: &ResolvedUser) -> Vec<String> {
    vec!["pi".to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::ResolvedUser;

    #[test]
    fn test_resolve_pi_command() {
        let cfg = Config::default();
        let user = ResolvedUser {
            uid: 1000,
            gid: 1000,
            username: "testuser".to_string(),
        };

        assert_eq!(resolve_pi_command(&cfg, &user), vec!["pi".to_string()]);
    }
}
