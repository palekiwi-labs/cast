use crate::config::Config;
use anyhow::{Context, Result};
use std::env;
use std::process::Command;

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
    fn uid(&self) -> Result<u32>;
    fn gid(&self) -> Result<u32>;
}

/// Production implementation — reads $USER and shells out to `id -u` / `id -g` / `id -un`.
pub struct HostUser;

impl ResolveUser for HostUser {
    fn username(&self) -> Option<String> {
        env::var("USER").ok().or_else(|| get_username().ok())
    }

    fn uid(&self) -> Result<u32> {
        get_id("-u").context("Failed to retrieve host UID")
    }

    fn gid(&self) -> Result<u32> {
        get_id("-g").context("Failed to retrieve host GID")
    }
}

fn get_id(flag: &str) -> Result<u32> {
    let output = Command::new("id")
        .arg(flag)
        .output()
        .context("Failed to execute `id` command (is it in PATH?)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "`id {}` command failed with status: {}\n{}",
            flag,
            output.status,
            stderr.trim()
        );
    }

    let s = std::str::from_utf8(&output.stdout).context("Output of `id` is not valid UTF-8")?;

    s.trim()
        .parse::<u32>()
        .context("Failed to parse `id` output as an integer")
}

fn get_username() -> Result<String> {
    let output = Command::new("id")
        .arg("-un")
        .output()
        .context("Failed to execute `id -un` command (is it in PATH?)")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "`id -un` command failed with status: {}\n{}",
            output.status,
            stderr.trim()
        );
    }

    let s = std::str::from_utf8(&output.stdout).context("Output of `id -un` is not valid UTF-8")?;

    Ok(s.trim().to_string())
}

/// Resolve the container user identity from config, environment, and host fallbacks.
///
/// Priority (each field resolved independently):
///   username: config.username → OCX_USERNAME env → host username → "user"
///   uid:      config.uid      → OCX_UID env      → host uid
///   gid:      config.gid      → OCX_GID env      → host gid
pub fn resolve_user(config: &Config, host: &impl ResolveUser) -> Result<ResolvedUser> {
    let username = config
        .username
        .clone()
        .or_else(|| env::var("OCX_USERNAME").ok())
        .or_else(|| host.username())
        .unwrap_or_else(|| "user".to_string());

    let uid = match config
        .uid
        .or_else(|| env::var("OCX_UID").ok().and_then(|v| v.parse().ok()))
    {
        Some(uid) => uid,
        None => host.uid()?,
    };

    let gid = match config
        .gid
        .or_else(|| env::var("OCX_GID").ok().and_then(|v| v.parse().ok()))
    {
        Some(gid) => gid,
        None => host.gid()?,
    };

    Ok(ResolvedUser { username, uid, gid })
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
        fn uid(&self) -> Result<u32> {
            Ok(self.uid)
        }
        fn gid(&self) -> Result<u32> {
            Ok(self.gid)
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

        let result = resolve_user(&config, &host("alice", 1001, 1002)).unwrap();

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

        let result = resolve_user(&config, &no_host_username(1001, 1002)).unwrap();

        assert_eq!(result.username, "user");
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

        let result = resolve_user(&config, &host("alice", 1001, 1002)).unwrap();

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
    fn test_host_user_resolves_on_real_system() {
        // Sanity check that HostUser can actually shell out and parse results
        // on the developer's machine. Fails fast if `id` is missing or broken.
        let host = HostUser;
        assert!(host.uid().is_ok());
        assert!(host.gid().is_ok());
        assert!(host.username().is_some());
    }

    #[test]
    fn test_host_values_used_when_config_is_empty() {
        let config = Config {
            username: None,
            uid: None,
            gid: None,
            ..Config::default()
        };

        let result = resolve_user(&config, &host("alice", 1001, 1002)).unwrap();

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
