use super::workspace::ResolvedWorkspace;
use crate::config::Config;
use crate::user::ResolvedUser;
use std::path::Path;

pub fn build_data_volume_args(cfg: &Config, user: &ResolvedUser) -> Vec<String> {
    let name = &cfg.data_volumes_name;
    let username = &user.username;
    vec![
        "-v".to_string(),
        format!("{}-cache:/home/{}/.cache:rw", name, username),
        "-v".to_string(),
        format!("{}-local:/home/{}/.local:rw", name, username),
    ]
}

pub fn build_extra_volume_args(
    cfg: &Config,
    user: &ResolvedUser,
    ws: &ResolvedWorkspace,
    host_home_dir: Option<&Path>,
) -> Vec<String> {
    let mut args = Vec::new();

    let mut entries: Vec<(&String, &crate::config::VolumeConfig)> =
        cfg.extra_data_volumes.iter().collect();
    entries.sort_by_key(|(k, _)| *k);

    for (key, vol) in entries {
        let target = expand_container_target(&vol.target, &user.username, &ws.container_path);
        let mount_spec = build_mount_spec(cfg, key, vol, &target, host_home_dir);
        args.push("-v".to_string());
        args.push(mount_spec);
    }

    args
}

/// Expand container-side target: `~/` → `/home/{username}/`, `./` → `{container_path}/`
fn expand_container_target(target: &str, username: &str, container_path: &Path) -> String {
    if target == "~" {
        format!("/home/{}", username)
    } else if let Some(rest) = target.strip_prefix("~/") {
        format!("/home/{}/{}", username, rest)
    } else if target == "." {
        container_path.to_string_lossy().into_owned()
    } else if let Some(rest) = target.strip_prefix("./") {
        format!("{}/{}", container_path.to_string_lossy(), rest)
    } else {
        target.to_string()
    }
}

/// Build the `{source}:{target}:{mode}` mount spec for a single volume entry.
fn build_mount_spec(
    cfg: &Config,
    key: &str,
    vol: &crate::config::VolumeConfig,
    resolved_target: &str,
    host_home_dir: Option<&Path>,
) -> String {
    if vol.volume_type == "bind" {
        let source = vol
            .source
            .as_deref()
            .map(|s| {
                super::utils::expand_tilde(s, host_home_dir)
                    .to_string_lossy()
                    .into_owned()
            })
            .unwrap_or_else(|| resolved_target.to_string());
        format!("{}:{}:{}", source, resolved_target, vol.mode)
    } else {
        // named volume
        let default_vol_name = format!("{}-{}", cfg.data_volumes_name, key);
        let vol_name = vol.source.as_deref().unwrap_or(&default_vol_name);
        format!("{}:{}:{}", vol_name, resolved_target, vol.mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, VolumeConfig};
    use std::path::PathBuf;

    fn make_user(username: &str) -> ResolvedUser {
        ResolvedUser {
            username: username.to_string(),
            uid: 1000,
            gid: 1000,
        }
    }

    fn make_ws(root: &str, container_path: &str) -> ResolvedWorkspace {
        ResolvedWorkspace {
            root: PathBuf::from(root),
            container_path: PathBuf::from(container_path),
        }
    }

    fn volume(target: &str, volume_type: &str, source: Option<&str>, mode: &str) -> VolumeConfig {
        VolumeConfig {
            target: target.to_string(),
            volume_type: volume_type.to_string(),
            source: source.map(str::to_string),
            mode: mode.to_string(),
        }
    }

    // --- build_data_volume_args ---

    // --- build_extra_volume_args ---

    #[test]
    fn test_extra_volume_args_empty_returns_empty_vec() {
        let cfg = Config::default();
        let user = make_user("alice");
        let ws = make_ws("/home/alice/my-app", "/home/alice/my-app");

        let args = build_extra_volume_args(&cfg, &user, &ws, Some(Path::new("/home/alice")));

        assert!(args.is_empty());
    }

    #[test]
    fn test_extra_volume_args_named_volume_plain_target() {
        let mut cfg = Config::default(); // data_volumes_name = "ocx"
        cfg.extra_data_volumes.insert(
            "cargo".to_string(),
            volume("/home/alice/.cargo", "volume", None, "rw"),
        );
        let user = make_user("alice");
        let ws = make_ws("/home/alice/my-app", "/home/alice/my-app");

        let args = build_extra_volume_args(&cfg, &user, &ws, Some(Path::new("/home/alice")));

        assert_eq!(args, vec!["-v", "ocx-cargo:/home/alice/.cargo:rw"]);
    }

    #[test]
    fn test_extra_volume_args_named_volume_tilde_target_expansion() {
        let mut cfg = Config::default();
        cfg.extra_data_volumes.insert(
            "cargo".to_string(),
            volume("~/.cargo", "volume", None, "rw"),
        );
        let user = make_user("alice");
        let ws = make_ws("/home/alice/my-app", "/home/alice/my-app");

        let args = build_extra_volume_args(&cfg, &user, &ws, Some(Path::new("/home/alice")));

        assert_eq!(args, vec!["-v", "ocx-cargo:/home/alice/.cargo:rw"]);
    }

    #[test]
    fn test_extra_volume_args_named_volume_dot_target_expansion() {
        let mut cfg = Config::default();
        cfg.extra_data_volumes
            .insert("data".to_string(), volume("./data", "volume", None, "ro"));
        let user = make_user("alice");
        let ws = make_ws("/home/alice/my-app", "/home/alice/my-app");

        let args = build_extra_volume_args(&cfg, &user, &ws, Some(Path::new("/home/alice")));

        assert_eq!(args, vec!["-v", "ocx-data:/home/alice/my-app/data:ro"]);
    }

    #[test]
    fn test_extra_volume_args_named_volume_explicit_source() {
        let mut cfg = Config::default();
        cfg.extra_data_volumes.insert(
            "cargo".to_string(),
            volume("~/.cargo", "volume", Some("my-cargo-vol"), "rw"),
        );
        let user = make_user("alice");
        let ws = make_ws("/home/alice/my-app", "/home/alice/my-app");

        let args = build_extra_volume_args(&cfg, &user, &ws, Some(Path::new("/home/alice")));

        assert_eq!(args, vec!["-v", "my-cargo-vol:/home/alice/.cargo:rw"]);
    }

    #[test]
    fn test_extra_volume_args_bind_mount_tilde_source_expansion() {
        let mut cfg = Config::default();
        cfg.extra_data_volumes.insert(
            "secrets".to_string(),
            volume("/container/secrets", "bind", Some("~/.secrets"), "ro"),
        );
        let user = make_user("alice");
        let ws = make_ws("/home/alice/my-app", "/home/alice/my-app");

        let args = build_extra_volume_args(&cfg, &user, &ws, Some(Path::new("/home/alice")));

        assert_eq!(
            args,
            vec!["-v", "/home/alice/.secrets:/container/secrets:ro"]
        );
    }

    #[test]
    fn test_extra_volume_args_bind_mount_no_home_dir_fallback() {
        let mut cfg = Config::default();
        cfg.extra_data_volumes.insert(
            "secrets".to_string(),
            volume("/container/secrets", "bind", Some("~/.secrets"), "ro"),
        );
        let user = make_user("alice");
        let ws = make_ws("/home/alice/my-app", "/home/alice/my-app");

        // No home dir available — tilde is kept as-is
        let args = build_extra_volume_args(&cfg, &user, &ws, None);

        assert_eq!(args, vec!["-v", "~/.secrets:/container/secrets:ro"]);
    }

    // --- build_data_volume_args ---

    #[test]
    fn test_data_volume_args_produces_cache_and_local_mounts() {
        let cfg = Config::default(); // data_volumes_name = "ocx"
        let user = make_user("alice");

        let args = build_data_volume_args(&cfg, &user);

        assert_eq!(
            args,
            vec![
                "-v",
                "ocx-cache:/home/alice/.cache:rw",
                "-v",
                "ocx-local:/home/alice/.local:rw",
            ]
        );
    }
}
