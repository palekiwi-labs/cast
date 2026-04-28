use std::path::Path;

/// Returns --env-file <path> args for cast.env files that exist on the host.
///
/// Checks two hardcoded paths (includes both if both exist, global first):
/// - Global:        ~/.config/cast/cast.env
/// - Project-local: {cwd}/cast.env
pub fn build_env_file_args(cwd: &Path, host_home_dir: Option<&Path>) -> Vec<String> {
    let mut args = Vec::new();

    // Global: ~/.config/cast/cast.env
    if let Some(home) = host_home_dir {
        let global_env = home.join(".config/cast/cast.env");
        if global_env.exists() {
            args.push("--env-file".to_string());
            args.push(global_env.to_string_lossy().into_owned());
        }
    }

    // Project-local: {cwd}/cast.env
    let local_env = cwd.join("cast.env");
    if local_env.exists() {
        args.push("--env-file".to_string());
        args.push(local_env.to_string_lossy().into_owned());
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_build_env_file_args_none_exists() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();
        let home = temp.path().join("home");

        let args = build_env_file_args(cwd, Some(&home));
        assert!(args.is_empty());
    }

    #[test]
    fn test_build_env_file_args_global_only() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();
        let home = temp.path().join("home");
        let global_env = home.join(".config/cast/cast.env");
        std::fs::create_dir_all(global_env.parent().unwrap()).unwrap();
        std::fs::write(&global_env, "FOO=bar").unwrap();

        let args = build_env_file_args(cwd, Some(&home));
        assert_eq!(args, vec!["--env-file", global_env.to_str().unwrap()]);
    }

    #[test]
    fn test_build_env_file_args_local_only() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();
        let home = temp.path().join("home");
        let local_env = cwd.join("cast.env");
        std::fs::write(&local_env, "FOO=bar").unwrap();

        let args = build_env_file_args(cwd, Some(&home));
        assert_eq!(args, vec!["--env-file", local_env.to_str().unwrap()]);
    }

    #[test]
    fn test_build_env_file_args_both_exist() {
        let temp = TempDir::new().unwrap();
        let cwd = temp.path();
        let home = temp.path().join("home");

        let global_env = home.join(".config/cast/cast.env");
        std::fs::create_dir_all(global_env.parent().unwrap()).unwrap();
        std::fs::write(&global_env, "GLOBAL=1").unwrap();

        let local_env = cwd.join("cast.env");
        std::fs::write(&local_env, "LOCAL=1").unwrap();

        let args = build_env_file_args(cwd, Some(&home));
        assert_eq!(
            args,
            vec![
                "--env-file",
                global_env.to_str().unwrap(),
                "--env-file",
                local_env.to_str().unwrap()
            ]
        );
    }
}
