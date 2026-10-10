use super::workspace::ResolvedWorkspace;
use crate::config::Config;
use crate::user::ResolvedUser;
use std::path::Path;

pub fn build_extra_volume_args(
    cfg: &Config,
    user: &ResolvedUser,
    ws: &ResolvedWorkspace,
    host_home_dir: Option<&Path>,
) -> Vec<String> {
    let mut args = Vec::new();

    for (key, vol) in &cfg.extra_data_volumes {
        let target = expand_container_target(&vol.target, &user.username, &ws.container_path);
        if vol.volume_type == "tmpfs" {
            // Ephemeral, per-container, and owned by the user: Docker would
            // otherwise create a missing mount directory as root.
            args.push("--tmpfs".to_string());
            args.push(format!(
                "{}:{},uid={},gid={},mode=0700",
                target, vol.mode, user.uid, user.gid
            ));
            continue;
        }
        let mount_spec = build_mount_spec(cfg, key, vol, &target, host_home_dir);
        args.push("-v".to_string());
        args.push(mount_spec);
    }

    args
}

/// Fail when a bind-mount source does not exist on the host.
///
/// Docker would otherwise create a missing source as a root-owned directory,
/// even for an intended file mount.
pub fn validate_bind_sources(
    cfg: &Config,
    user: &ResolvedUser,
    ws: &ResolvedWorkspace,
    host_home_dir: Option<&Path>,
) -> anyhow::Result<()> {
    for (key, vol) in &cfg.extra_data_volumes {
        if vol.volume_type != "bind" {
            continue;
        }
        let target = expand_container_target(&vol.target, &user.username, &ws.container_path);
        let source = bind_source(vol, &target, host_home_dir);
        if !Path::new(&source).exists() {
            anyhow::bail!(
                "extra_data_volumes entry '{key}': bind source '{source}' does not exist on the host"
            );
        }
    }
    Ok(())
}

fn bind_source(
    vol: &crate::config::VolumeConfig,
    resolved_target: &str,
    host_home_dir: Option<&Path>,
) -> String {
    vol.source
        .as_deref()
        .map(|s| {
            super::utils::expand_tilde(s, host_home_dir)
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| resolved_target.to_string())
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
        let source = bind_source(vol, resolved_target, host_home_dir);
        format!("{}:{}:{}", source, resolved_target, vol.mode)
    } else {
        // named volume
        let default_vol_name = format!("{}-{}", cfg.volumes_namespace, key);
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

    // --- validate_bind_sources ---

    #[test]
    fn test_validate_bind_sources_rejects_missing_source_naming_key_and_path() {
        let home = std::env::temp_dir().join(format!("cast-bind-missing-{}", std::process::id()));
        let mut cfg = Config::default();
        cfg.extra_data_volumes.insert(
            "pi".to_string(),
            volume("~/.pi", "bind", Some("~/.pi"), "rw"),
        );
        let user = make_user("alice");
        let ws = make_ws("/home/alice/my-app", "/home/alice/my-app");

        let err = validate_bind_sources(&cfg, &user, &ws, Some(&home)).unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("pi"), "error should name the entry: {msg}");
        assert!(
            msg.contains(&home.join(".pi").display().to_string()),
            "error should name the host path: {msg}"
        );
    }

    #[test]
    fn test_validate_bind_sources_accepts_existing_source_and_ignores_non_binds() {
        let home = std::env::temp_dir().join(format!("cast-bind-present-{}", std::process::id()));
        std::fs::create_dir_all(home.join(".pi")).unwrap();
        let mut cfg = Config::default();
        cfg.extra_data_volumes.insert(
            "pi".to_string(),
            volume("~/.pi", "bind", Some("~/.pi"), "rw"),
        );
        cfg.extra_data_volumes
            .insert("state".to_string(), volume("~/.state", "tmpfs", None, "rw"));
        let user = make_user("alice");
        let ws = make_ws("/home/alice/my-app", "/home/alice/my-app");

        let result = validate_bind_sources(&cfg, &user, &ws, Some(&home));

        std::fs::remove_dir_all(&home).unwrap();
        assert!(result.is_ok(), "{result:?}");
    }

    // --- build_extra_volume_args ---

    #[test]
    fn test_extra_volume_args_tmpfs_is_an_ephemeral_user_owned_directory() {
        let mut cfg = Config::default();
        cfg.extra_data_volumes.insert(
            "herdr-state".to_string(),
            volume("~/.config/herdr", "tmpfs", None, "rw"),
        );
        let user = make_user("alice");
        let ws = make_ws("/home/alice/my-app", "/home/alice/my-app");

        let args = build_extra_volume_args(&cfg, &user, &ws, Some(Path::new("/home/alice")));

        assert_eq!(
            args,
            vec![
                "--tmpfs",
                "/home/alice/.config/herdr:rw,uid=1000,gid=1000,mode=0700"
            ]
        );
    }

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
        let mut cfg = Config::default(); // volumes_namespace = "cast"
        cfg.extra_data_volumes.insert(
            "cargo".to_string(),
            volume("/home/alice/.cargo", "volume", None, "rw"),
        );
        let user = make_user("alice");
        let ws = make_ws("/home/alice/my-app", "/home/alice/my-app");

        let args = build_extra_volume_args(&cfg, &user, &ws, Some(Path::new("/home/alice")));

        assert_eq!(args, vec!["-v", "cast-cargo:/home/alice/.cargo:rw"]);
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

        assert_eq!(args, vec!["-v", "cast-cargo:/home/alice/.cargo:rw"]);
    }

    #[test]
    fn test_extra_volume_args_named_volume_dot_target_expansion() {
        let mut cfg = Config::default();
        cfg.extra_data_volumes
            .insert("data".to_string(), volume("./data", "volume", None, "ro"));
        let user = make_user("alice");
        let ws = make_ws("/home/alice/my-app", "/home/alice/my-app");

        let args = build_extra_volume_args(&cfg, &user, &ws, Some(Path::new("/home/alice")));

        assert_eq!(args, vec!["-v", "cast-data:/home/alice/my-app/data:ro"]);
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
}
