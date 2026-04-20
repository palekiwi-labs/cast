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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_run_args_with_minimal_params() {
        let args = build_run_args(
            "ocx-nix-daemon",
            "localhost/ocx-nix-daemon:sha-12345678",
            vec![],
            None,
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

    #[test]
    fn test_build_run_args_with_opts() {
        let args = build_run_args(
            "ocx-nix-daemon",
            "localhost/ocx-nix-daemon:sha-12345678",
            vec!["-v".to_string(), "ocx-nix:/nix:rw".to_string()],
            None,
        );

        assert_eq!(
            args,
            vec![
                "run",
                "--name",
                "ocx-nix-daemon",
                "-v",
                "ocx-nix:/nix:rw",
                "localhost/ocx-nix-daemon:sha-12345678"
            ]
        );
    }

    #[test]
    fn test_build_run_args_with_cmd() {
        let args = build_run_args(
            "ocx-nix-daemon",
            "localhost/ocx-nix-daemon:sha-12345678",
            vec![],
            Some(vec!["nix".to_string(), "develop".to_string()]),
        );

        assert_eq!(
            args,
            vec![
                "run",
                "--name",
                "ocx-nix-daemon",
                "localhost/ocx-nix-daemon:sha-12345678",
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
