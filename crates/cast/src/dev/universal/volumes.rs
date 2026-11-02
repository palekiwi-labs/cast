use crate::config::Config;
use crate::dev::run::RunOpts;
use crate::user::ResolvedUser;

/// Build the shared data volume arguments for the dev container.
///
/// Returns `-v` pairs for `{namespace}-cache` and `{namespace}-local`. A single
/// pair of named volumes is shared by every agent running inside the container.
pub fn build_universal_data_volume_args(config: &Config, user: &ResolvedUser) -> Vec<String> {
    let namespace = &config.volumes_namespace;
    let username = &user.username;
    vec![
        "-v".to_string(),
        format!("{namespace}-cache:/home/{username}/.cache:rw"),
        "-v".to_string(),
        format!("{namespace}-local:/home/{username}/.local:rw"),
    ]
}

/// Build the global cast Nix directory bind-mount arguments when it exists.
pub fn build_cast_nix_mount_args(opts: &RunOpts) -> Vec<String> {
    let Some(nix_dir) = opts
        .host_home_dir
        .as_ref()
        .map(|home| home.join(".config/cast/nix"))
        .filter(|path| path.is_dir())
    else {
        return vec![];
    };

    vec![
        "-v".to_string(),
        format!(
            "{}:/home/{}/.config/cast/nix:rw",
            nix_dir.display(),
            opts.user.username
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alice() -> ResolvedUser {
        ResolvedUser {
            username: "alice".to_string(),
            uid: 1000,
            gid: 1000,
        }
    }

    // ── build_universal_data_volume_args ───────────────────────────────────

    #[test]
    fn data_volumes_use_universal_namespace_and_correct_paths() {
        let config = Config::default(); // volumes_namespace = "cast"
        let user = alice();

        let args = build_universal_data_volume_args(&config, &user);

        assert!(
            args.contains(&"cast-cache:/home/alice/.cache:rw".to_string()),
            "expected cache volume, got: {args:?}"
        );
        assert!(
            args.contains(&"cast-local:/home/alice/.local:rw".to_string()),
            "expected local volume, got: {args:?}"
        );
    }

    #[test]
    fn data_volumes_respect_custom_volumes_namespace() {
        let config = Config {
            volumes_namespace: "myteam".to_string(),
            ..Config::default()
        };
        let user = alice();

        let args = build_universal_data_volume_args(&config, &user);

        assert!(args.contains(&"myteam-cache:/home/alice/.cache:rw".to_string()));
        assert!(args.contains(&"myteam-local:/home/alice/.local:rw".to_string()));
    }
}
