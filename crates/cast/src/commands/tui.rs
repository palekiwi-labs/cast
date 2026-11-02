//! Bare `cast` hands the terminal to cast-tui, the host TUI shipped by
//! cue-platform (git-style: a separate executable on PATH).

use std::ffi::{OsStr, OsString};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

const TUI_BIN: &str = "cast-tui";

/// Resolves the cast-tui executable: `$CAST_TUI` when set and non-empty,
/// otherwise the first executable `cast-tui` on `path`.
pub fn resolve(cast_tui: Option<&OsStr>, path: Option<&OsStr>) -> Option<PathBuf> {
    if let Some(explicit) = cast_tui.filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(explicit));
    }
    std::env::split_paths(path?)
        .map(|dir| dir.join(TUI_BIN))
        .find(|candidate| is_executable(candidate))
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

/// Replaces this process with cast-tui. The current directory, already
/// moved by `-C`, is inherited. Returns only on failure.
pub fn exec() -> Result<std::convert::Infallible> {
    let cast_tui: Option<OsString> = std::env::var_os("CAST_TUI");
    let path: Option<OsString> = std::env::var_os("PATH");
    let program = resolve(cast_tui.as_deref(), path.as_deref()).ok_or_else(|| {
        anyhow!("{TUI_BIN} not found on PATH; install the cast-tui package or set CAST_TUI")
    })?;
    let err = std::process::Command::new(&program).exec();
    Err(anyhow!("Failed to run {}: {err}", program.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn write(dir: &Path, mode: u32) -> PathBuf {
        let p = dir.join(TUI_BIN);
        std::fs::write(&p, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode)).unwrap();
        p
    }

    #[test]
    fn env_override_wins_over_path() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), 0o755);
        let got = resolve(
            Some(OsStr::new("/custom/tui")),
            Some(dir.path().as_os_str()),
        );
        assert_eq!(got, Some(PathBuf::from("/custom/tui")));
    }

    #[test]
    fn empty_env_override_falls_back_to_path() {
        let dir = tempfile::tempdir().unwrap();
        let expected = write(dir.path(), 0o755);
        let got = resolve(Some(OsStr::new("")), Some(dir.path().as_os_str()));
        assert_eq!(got, Some(expected));
    }

    #[test]
    fn first_executable_on_path_is_chosen() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        write(first.path(), 0o644);
        let expected = write(second.path(), 0o755);
        let path = std::env::join_paths([first.path(), second.path()]).unwrap();
        assert_eq!(resolve(None, Some(&path)), Some(expected));
    }

    #[test]
    fn missing_everywhere_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(resolve(None, Some(dir.path().as_os_str())), None);
        assert_eq!(resolve(None, None), None);
    }
}
