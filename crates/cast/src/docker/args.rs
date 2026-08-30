use std::path::Path;

pub fn build_run_args(
    name: &str,
    image: &str,
    opts: Vec<String>,
    cmd: Option<Vec<String>>,
) -> Vec<String> {
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

/// Build arguments for `docker ps` command to check if a container is running
pub fn build_ps_args(name: &str) -> Vec<String> {
    vec![
        "ps".to_string(),
        "--filter".to_string(),
        format!("name=^{}$", name),
        "--format".to_string(),
        "{{.Names}}".to_string(),
    ]
}

/// Build arguments for `docker ps --all` to check whether a container exists.
pub fn build_ps_all_args(name: &str) -> Vec<String> {
    vec![
        "ps".to_string(),
        "--all".to_string(),
        "--filter".to_string(),
        format!("name=^{}$", name),
        "--format".to_string(),
        "{{.Names}}".to_string(),
    ]
}

/// Build arguments for observing full commands in a running container.
pub fn build_top_args(name: &str) -> Vec<String> {
    vec![
        "top".to_string(),
        name.to_string(),
        "-eo".to_string(),
        "pid,args".to_string(),
    ]
}

/// Build arguments for the most recent container log lines.
pub fn build_logs_args(name: &str, tail: usize) -> Vec<String> {
    vec![
        "logs".to_string(),
        "--tail".to_string(),
        tail.to_string(),
        name.to_string(),
    ]
}

/// Build arguments for `docker images` command to check if an image exists
pub fn build_image_exists_args(tag: &str) -> Vec<String> {
    vec![
        "images".to_string(),
        "--filter".to_string(),
        format!("reference={}", tag),
        "--format".to_string(),
        "{{.Repository}}:{{.Tag}}".to_string(),
    ]
}

/// Build arguments for `docker build` command
pub fn build_docker_build_args(
    tag: &str,
    context_path: &Path,
    build_args: &[(&str, &str)],
    no_cache: bool,
) -> Vec<String> {
    let mut args = vec!["build".to_string(), "-t".to_string(), tag.to_string()];

    for (key, value) in build_args {
        args.push("--build-arg".to_string());
        args.push(format!("{}={}", key, value));
    }

    if no_cache {
        args.push("--no-cache".to_string());
    }

    args.push(context_path.to_string_lossy().to_string());
    args
}

/// Build arguments for `docker stop` command
pub fn build_stop_args(name: &str) -> Vec<String> {
    vec!["stop".to_string(), name.to_string()]
}

/// Build arguments for `docker rm` command.
pub fn build_remove_args(name: &str) -> Vec<String> {
    vec!["rm".to_string(), name.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_run_args_with_minimal_params() {
        let args = build_run_args(
            "cast-nix-daemon",
            "localhost/cast-nix-daemon:sha-12345678",
            vec![],
            None,
        );

        assert_eq!(
            args,
            vec![
                "run",
                "--name",
                "cast-nix-daemon",
                "localhost/cast-nix-daemon:sha-12345678"
            ]
        );
    }

    #[test]
    fn test_build_run_args_with_opts() {
        let args = build_run_args(
            "cast-nix-daemon",
            "localhost/cast-nix-daemon:sha-12345678",
            vec!["-v".to_string(), "cast-nix:/nix:rw".to_string()],
            None,
        );

        assert_eq!(
            args,
            vec![
                "run",
                "--name",
                "cast-nix-daemon",
                "-v",
                "cast-nix:/nix:rw",
                "localhost/cast-nix-daemon:sha-12345678"
            ]
        );
    }

    #[test]
    fn test_build_run_args_with_cmd() {
        let args = build_run_args(
            "cast-nix-daemon",
            "localhost/cast-nix-daemon:sha-12345678",
            vec![],
            Some(vec!["nix".to_string(), "develop".to_string()]),
        );

        assert_eq!(
            args,
            vec![
                "run",
                "--name",
                "cast-nix-daemon",
                "localhost/cast-nix-daemon:sha-12345678",
                "nix",
                "develop"
            ]
        );
    }

    #[test]
    fn test_build_ps_args() {
        let args = build_ps_args("my-container");

        assert_eq!(
            args,
            vec![
                "ps",
                "--filter",
                "name=^my-container$",
                "--format",
                "{{.Names}}"
            ]
        );
    }

    #[test]
    fn build_ps_all_args_includes_stopped_containers() {
        assert_eq!(
            build_ps_all_args("cast-project-a1b2c3d4e5f6"),
            vec![
                "ps",
                "--all",
                "--filter",
                "name=^cast-project-a1b2c3d4e5f6$",
                "--format",
                "{{.Names}}",
            ]
        );
    }

    #[test]
    fn build_top_args_reports_full_process_commands() {
        assert_eq!(
            build_top_args("cast-project-a1b2c3d4e5f6"),
            vec!["top", "cast-project-a1b2c3d4e5f6", "-eo", "pid,args"],
        );
    }

    #[test]
    fn build_logs_args_limits_startup_failure_output() {
        assert_eq!(
            build_logs_args("cast-project-a1b2c3d4e5f6", 100),
            vec!["logs", "--tail", "100", "cast-project-a1b2c3d4e5f6"],
        );
    }

    #[test]
    fn test_build_image_exists_args() {
        let args = build_image_exists_args("my-image:tag");

        assert_eq!(
            args,
            vec![
                "images",
                "--filter",
                "reference=my-image:tag",
                "--format",
                "{{.Repository}}:{{.Tag}}"
            ]
        );
    }

    #[test]
    fn test_build_docker_build_args() {
        let context = Path::new("/tmp/build");
        let args = build_docker_build_args("my-image:tag", context, &[("FOO", "bar")], true);

        assert_eq!(
            args,
            vec![
                "build",
                "-t",
                "my-image:tag",
                "--build-arg",
                "FOO=bar",
                "--no-cache",
                "/tmp/build"
            ]
        );
    }

    #[test]
    fn test_build_stop_args() {
        let args = build_stop_args("my-container");
        assert_eq!(args, vec!["stop", "my-container"]);
    }
}
