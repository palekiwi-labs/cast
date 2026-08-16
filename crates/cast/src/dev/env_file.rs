use std::collections::BTreeSet;
use std::path::Path;
use tracing::debug;

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

/// Returns valueless `-e NAME` args for allowlisted host env vars.
///
/// The value is deliberately never emitted: `docker run -e NAME` makes docker
/// read the value from its own (inherited) environment, so secrets stay out of
/// cast's argv and therefore out of `ps` for other host users.
///
/// Takes host env variable NAMES only, never values: the function has no use
/// for a value, and the missing value channel is what makes a leak from here
/// unrepresentable rather than merely unlikely. Filtering (e.g. dropping names
/// whose host value is empty) belongs at the boundary that legitimately holds
/// values — see `run_in_container`.
///
/// A name is skipped when it is not a valid shell variable name
/// (`[A-Za-z_][A-Za-z0-9_]*`) or is absent from `host_env_names`. Duplicates
/// emit a single pair, and output is sorted by name so it is independent of
/// the order names appear in `cast.json`.
pub fn build_env_passthrough_args(
    allowlist: &[String],
    host_env_names: &BTreeSet<String>,
) -> Vec<String> {
    let names: Vec<&str> = allowlist
        .iter()
        .map(String::as_str)
        .filter(|name| is_valid_env_name(name))
        .filter(|name| host_env_names.contains(*name))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    if !names.is_empty() {
        // Names only. Logging a value here would defeat the whole design.
        debug!(names = ?names, "passing through host env vars");
    }

    names
        .into_iter()
        .flat_map(|name| ["-e".to_string(), name.to_string()])
        .collect()
}

fn is_valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn host(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn names(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_env_passthrough_emits_valueless_flag_for_present_name() {
        let host = host(&["GH_TOKEN"]);
        let args = build_env_passthrough_args(&names(&["GH_TOKEN"]), &host);
        // Valueless form: docker reads the value from its own inherited env,
        // so the secret never reaches cast's argv.
        assert_eq!(args, vec!["-e", "GH_TOKEN"]);
    }

    #[test]
    fn test_env_passthrough_skips_name_unset_on_host() {
        let host = host(&["OTHER"]);
        let args = build_env_passthrough_args(&names(&["GH_TOKEN"]), &host);
        assert!(args.is_empty());
    }

    #[test]
    fn test_env_passthrough_skips_invalid_names() {
        let host = host(&["", "1BAD", "HAS-DASH", "HAS SPACE", "HAS=EQUALS"]);
        let allowlist = names(&["", "1BAD", "HAS-DASH", "HAS SPACE", "HAS=EQUALS"]);
        let args = build_env_passthrough_args(&allowlist, &host);
        assert!(args.is_empty(), "invalid names emitted: {args:?}");
    }

    #[test]
    fn test_env_passthrough_accepts_leading_underscore_and_digits() {
        let host = host(&["_PRIVATE", "VAR2"]);
        let args = build_env_passthrough_args(&names(&["_PRIVATE", "VAR2"]), &host);
        // Byte order: '_' (0x5F) sorts after uppercase letters.
        assert_eq!(args, vec!["-e", "VAR2", "-e", "_PRIVATE"]);
    }

    #[test]
    fn test_env_passthrough_sorts_independently_of_config_order() {
        let host = host(&["ZULU", "ALPHA", "MIKE"]);
        let args = build_env_passthrough_args(&names(&["ZULU", "MIKE", "ALPHA"]), &host);
        assert_eq!(args, vec!["-e", "ALPHA", "-e", "MIKE", "-e", "ZULU"]);
    }

    #[test]
    fn test_env_passthrough_dedupes_repeated_name() {
        let host = host(&["GH_TOKEN"]);
        let allowlist = names(&["GH_TOKEN", "GH_TOKEN"]);
        let args = build_env_passthrough_args(&allowlist, &host);
        assert_eq!(args, vec!["-e", "GH_TOKEN"]);
    }

    #[test]
    fn test_env_passthrough_empty_allowlist_emits_nothing() {
        let host = host(&["GH_TOKEN"]);
        let args = build_env_passthrough_args(&[], &host);
        assert!(args.is_empty());
    }

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
