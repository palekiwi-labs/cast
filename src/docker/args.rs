pub fn build_run_args(
    name: &str,
    image: &str,
    opts: Vec<String>,
    cmd: Option<Vec<String>>
) ->Vec<String> {
    let mut args: Vec<String> = vec!["run".to_string()];

    args.push("--name".to_string());
    args.push(name.to_string());

    args.extend(opts);

    args.push(image.to_string());

    if let Some(cmd) = cmd {
        args.extend(cmd);
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_run_args_with_minimal_params() {
        let args = build_run_args(
            "ocx-nix-daemon",
            "localhost/ocx-nix-daemon:sha-12345678",
            vec![],
            None
        );

        assert_eq!(
            args,
            vec![
                "run",
                "--name",
                "ocx-nix-daemon",
                "localhost/ocx-nix-daemon:sha-12345678"
            ]
        );
    }
}
