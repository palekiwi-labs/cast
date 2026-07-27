use cast_agent::rundir::{create_run_dir, resolve_base};
use std::path::PathBuf;

#[test]
fn flag_wins_over_env_and_tmpdir() {
    let base = resolve_base(
        Some(PathBuf::from("/flag/dir")),
        Some("/env/dir".into()),
        Some("/tmp/xyz".into()),
    );
    assert_eq!(base, PathBuf::from("/flag/dir"));
}

#[test]
fn env_wins_over_tmpdir() {
    let base = resolve_base(None, Some("/env/dir".into()), Some("/tmp/xyz".into()));
    assert_eq!(base, PathBuf::from("/env/dir"));
}

#[test]
fn tmpdir_default_layout() {
    let base = resolve_base(None, None, Some("/tmp/xyz".into()));
    assert_eq!(base, PathBuf::from("/tmp/xyz/cast-agent/runs"));
}

#[test]
fn falls_back_to_slash_tmp_when_no_tmpdir() {
    let base = resolve_base(None, None, None);
    assert_eq!(base, PathBuf::from("/tmp/cast-agent/runs"));
}

#[test]
fn create_run_dir_makes_a_unique_subdir_under_base() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().to_path_buf();

    let a = create_run_dir(&base, "opencode").unwrap();
    let b = create_run_dir(&base, "opencode").unwrap();

    assert!(a.is_dir());
    assert!(b.is_dir());
    assert_ne!(a, b, "two runs must not collide");
    assert!(a.starts_with(&base));

    let name = a.file_name().unwrap().to_string_lossy();
    assert!(name.contains("opencode"), "subdir name embeds the harness");
}
