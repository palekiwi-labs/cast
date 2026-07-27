use cast_agent::prompt::choose_prompt;

#[test]
fn file_wins_over_stdin_and_positional() {
    let got = choose_prompt(
        Some("from-file".into()),
        Some("from-stdin".into()),
        Some("from-arg".into()),
    )
    .unwrap();
    assert_eq!(got, "from-file");
}

#[test]
fn stdin_wins_over_positional() {
    let got = choose_prompt(None, Some("from-stdin".into()), Some("from-arg".into())).unwrap();
    assert_eq!(got, "from-stdin");
}

#[test]
fn positional_used_when_no_file_or_stdin() {
    let got = choose_prompt(None, None, Some("from-arg".into())).unwrap();
    assert_eq!(got, "from-arg");
}

#[test]
fn errors_when_no_prompt_source() {
    assert!(choose_prompt(None, None, None).is_err());
}

#[test]
fn blank_stdin_falls_through_to_positional() {
    // An empty/whitespace-only stdin capture is treated as absent.
    let got = choose_prompt(None, Some("   \n".into()), Some("from-arg".into())).unwrap();
    assert_eq!(got, "from-arg");
}
