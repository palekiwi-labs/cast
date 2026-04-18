use crate::config::Config;
use std::env;

/// The resolved container user identity.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedUser {
    pub username: String,
    pub uid: u32,
    pub gid: u32,
}

/// Abstracts host system user lookups so resolution logic is fully testable.
pub trait ResolveUser {
    fn username(&self) -> Option<String>;
    fn uid(&self) -> u32;
    fn gid(&self) -> u32;
}

/// Production implementation — reads $USER and shells out to `id -u` / `id -g`.
pub struct HostUser;

impl ResolveUser for HostUser {
    fn username(&self) -> Option<String> {
        env::var("USER").ok()
    }

    fn uid(&self) -> u32 {
        std::process::Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(1000)
    }

    fn gid(&self) -> u32 {
        std::process::Command::new("id")
            .arg("-g")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(1000)
    }
}

/// Resolve the container user identity from config, environment, and host fallbacks.
///
/// Priority (each field resolved independently):
///   username: config.username → OCX_USERNAME env → host username → "user"
///   uid:      config.uid      → OCX_UID env      → host uid
///   gid:      config.gid      → OCX_GID env      → host gid
pub fn resolve_user(config: &Config, host: &impl ResolveUser) -> ResolvedUser {
    let username = config
        .username
        .clone()
        .or_else(|| env::var("OCX_USERNAME").ok())
        .or_else(|| host.username())
        .unwrap_or_else(|| "user".to_string());

    let uid = config
        .uid
        .or_else(|| env::var("OCX_UID").ok().and_then(|v| v.parse().ok()))
        .unwrap_or_else(|| host.uid());

    let gid = config
        .gid
        .or_else(|| env::var("OCX_GID").ok().and_then(|v| v.parse().ok()))
        .unwrap_or_else(|| host.gid());

    ResolvedUser { username, uid, gid }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockHostUser {
        username: Option<String>,
        uid: u32,
        gid: u32,
    }

    impl ResolveUser for MockHostUser {
        fn username(&self) -> Option<String> {
            self.username.clone()
        }
        fn uid(&self) -> u32 {
            self.uid
        }
        fn gid(&self) -> u32 {
            self.gid
        }
    }

    fn host(username: &str, uid: u32, gid: u32) -> MockHostUser {
        MockHostUser {
            username: Some(username.to_string()),
            uid,
            gid,
        }
    }

    fn no_host_username(uid: u32, gid: u32) -> MockHostUser {
        MockHostUser {
            username: None,
            uid,
            gid,
        }
    }

    #[test]
    fn test_mixed_sources_username_from_config_ids_from_host() {
        let config = Config {
            username: Some("carol".to_string()),
            uid: None,
            gid: None,
            ..Config::default()
        };

        let result = resolve_user(&config, &host("alice", 1001, 1002));

        assert_eq!(
            result,
            ResolvedUser {
                username: "carol".to_string(),
                uid: 1001,
                gid: 1002,
            }
        );
    }

    #[test]
    fn test_username_fallback_to_user_when_host_has_none() {
        let config = Config {
            username: None,
            uid: None,
            gid: None,
            ..Config::default()
        };

        // Host provides no username
        let result = resolve_user(&config, &no_host_username(1001, 1002));

        assert_eq!(result.username, "user");
        // uid/gid still come from host
        assert_eq!(result.uid, 1001);
        assert_eq!(result.gid, 1002);
    }

    #[test]
    fn test_config_values_take_priority_over_host() {
        let config = Config {
            username: Some("bob".to_string()),
            uid: Some(2001),
            gid: Some(2002),
            ..Config::default()
        };

        // Host provides different values — should be ignored
        let result = resolve_user(&config, &host("alice", 1001, 1002));

        assert_eq!(
            result,
            ResolvedUser {
                username: "bob".to_string(),
                uid: 2001,
                gid: 2002,
            }
        );
    }

    #[test]
    fn test_host_values_used_when_config_is_empty() {
        let config = Config {
            username: None,
            uid: None,
            gid: None,
            ..Config::default()
        };

        let result = resolve_user(&config, &host("alice", 1001, 1002));

        assert_eq!(
            result,
            ResolvedUser {
                username: "alice".to_string(),
                uid: 1001,
                gid: 1002,
            }
        );
    }
}
