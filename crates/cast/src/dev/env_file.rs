use std::collections::BTreeSet;
use std::path::Path;
use tracing::{debug, warn};

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

/// Names the image or the container user owns; forwarding a host value for
/// them breaks the sandbox itself (nix PATH, home-directory mounts, or the
/// read-only store client), and no host value can be meaningful inside the
/// container. They pass config approval like any other entry and are
/// dropped when the passthrough args are built, with a warning per name.
pub const RESERVED_ENV_NAMES: &[&str] = &["PATH", "HOME", "NIX_REMOTE"];

/// Reserved names listed in `allowlist`, deduplicated and sorted, for the
/// run boundary to surface on the console (the file log already records
/// each drop; the args builder stays console-silent so its unit tests
/// print nothing).
pub fn reserved_names_in(allowlist: &[String]) -> BTreeSet<String> {
    allowlist
        .iter()
        .filter(|name| RESERVED_ENV_NAMES.contains(&name.as_str()))
        .cloned()
        .collect()
}

/// Returns valueless `-e NAME` args for allowlisted host env vars: docker
/// reads each value from its own inherited environment, so values never
/// reach cast's argv.
///
/// A name is skipped when it is not a valid shell variable name
/// (`[A-Za-z_][A-Za-z0-9_]*`), is reserved (see `RESERVED_ENV_NAMES`), or is
/// absent from `host_env_names`. Duplicates collapse and output is sorted
/// by name.
pub fn build_env_passthrough_args(
    allowlist: &[String],
    host_env_names: &BTreeSet<String>,
) -> Vec<String> {
    let reserved: BTreeSet<&str> = allowlist
        .iter()
        .map(String::as_str)
        .filter(|name| RESERVED_ENV_NAMES.contains(name))
        .collect();
    for name in &reserved {
        warn!(name = %name, "dropping reserved env_passthrough name");
    }

    let names: Vec<&str> = allowlist
        .iter()
        .map(String::as_str)
        .filter(|name| !reserved.contains(name))
        .filter(|name| is_valid_env_name(name))
        .filter(|name| host_env_names.contains(*name))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    if !names.is_empty() {
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
    fn test_env_passthrough_drops_reserved_names() {
        let host = host(&["PATH", "HOME", "NIX_REMOTE"]);
        let allowlist = names(&["PATH", "HOME", "NIX_REMOTE"]);
        let args = build_env_passthrough_args(&allowlist, &host);
        assert!(
            args.is_empty(),
            "reserved names must never be forwarded: {args:?}"
        );
    }

    #[test]
    fn test_env_passthrough_reserved_drop_spares_other_names() {
        let host = host(&["PATH", "GH_TOKEN"]);
        let allowlist = names(&["PATH", "GH_TOKEN"]);
        let args = build_env_passthrough_args(&allowlist, &host);
        assert_eq!(args, vec!["-e", "GH_TOKEN"]);
    }

    #[test]
    fn test_reserved_names_in_dedupes_sorts_and_ignores_others() {
        let allowlist = names(&["GH_TOKEN", "HOME", "PATH", "HOME"]);
        let reserved: Vec<String> = reserved_names_in(&allowlist).into_iter().collect();
        assert_eq!(reserved, names(&["HOME", "PATH"]));
    }

    #[test]
    fn test_reserved_names_in_empty_when_allowlist_has_none() {
        assert!(reserved_names_in(&names(&["GH_TOKEN"])).is_empty());
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
