use cast_agent::harness::{Harness, OpenCode};

#[test]
fn headless_args_are_run_format_json() {
    let h = OpenCode;
    assert_eq!(h.headless_args(), vec!["run", "--format", "json"]);
}
