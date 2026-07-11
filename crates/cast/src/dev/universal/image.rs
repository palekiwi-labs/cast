use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tracing::info;

use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::extra_dirs::resolve_extra_dirs;
use crate::dev::image::{CAST_VERSION, IMAGE_BASE};
use crate::dev::universal::dockerfile;
use crate::docker::client::DockerClient;
use crate::docker::BuildOptions;
use crate::user::ResolvedUser;

/// Number of hex characters taken from the SHA-256 digest for the image tag.
const TAG_HASH_LEN: usize = 12;

/// Compute the universal image tag from the agent→version map.
///
/// The tag is a content hash: the sorted `agent=version` pairs are serialised
/// (`"agent1=ver1;agent2=ver2;…"`), SHA-256 hashed, and the first 12 hex chars
/// are appended to a stable prefix. This means the same set of pinned agents
/// always maps to the same tag, regardless of insertion order.
pub fn universal_image_tag(agent_versions: &BTreeMap<String, String>) -> String {
    format!(
        "{}:{}-universal-{}",
        IMAGE_BASE,
        CAST_VERSION,
        short_content_hash(agent_versions)
    )
}

/// Serialise the map to `k1=v1;k2=v2;…` (sorted by key), SHA-256 hash it, and
/// return the first `TAG_HASH_LEN` hex characters.
fn short_content_hash(agent_versions: &BTreeMap<String, String>) -> String {
    let input: String = agent_versions
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(";");
    let digest = Sha256::digest(input.as_bytes());
    let hex = hex::encode(digest);
    hex[..TAG_HASH_LEN].to_string()
}

/// Human-readable composition for the `cast.universal.agents` Docker label.
/// Uses the same sorted serialisation as the hash, comma-separated.
fn compose_label(agent_versions: &BTreeMap<String, String>) -> String {
    agent_versions
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Ensure the universal image exists locally, building it if necessary.
///
/// The image tag is derived from the **full** `config.agent_versions` map (not
/// just the agents passed in `agents_with_versions`) so it is stable for a given
/// configuration regardless of which subset triggered the build.
pub fn ensure_universal_image(
    agents_with_versions: &[(&dyn Agent, &str)],
    docker: &DockerClient,
    config: &Config,
    user: &ResolvedUser,
    opts: BuildOptions,
) -> Result<()> {
    let image_tag = universal_image_tag(&config.agent_versions);

    if !opts.force && docker.image_exists(&image_tag)? {
        info!(%image_tag, "universal image exists, skipping build");
        eprintln!("universal dev image already exists: {}", image_tag);
        if opts.no_cache {
            eprintln!(
                "Hint: You passed --no-cache. \
                 To force a rebuild of the existing image, use --force."
            );
        }
        return Ok(());
    }

    info!(%image_tag, "universal image not found or force rebuild, building");
    eprintln!("Building universal dev image: {}", image_tag);

    let agents: Vec<&dyn Agent> = agents_with_versions.iter().map(|(a, _)| *a).collect();
    let dockerfile = dockerfile::assemble(&agents);

    let temp_dir = TempDir::new()?;
    let context_path = temp_dir.path();
    let dockerfile_path = context_path.join("Dockerfile");
    fs::write(&dockerfile_path, &dockerfile)?;

    let extra_dirs = resolve_extra_dirs(config, &user.username);
    let uid_str = user.uid.to_string();
    let gid_str = user.gid.to_string();
    let label = compose_label(&config.agent_versions);

    // Assemble build-arg tuples: per-agent version args first, then the
    // standard user/dir args shared with the per-agent image path.
    let mut build_arg_values: Vec<(String, String)> = agents_with_versions
        .iter()
        .map(|(agent, version)| {
            (
                format!("{}_VERSION", agent.name().to_uppercase()),
                (*version).to_string(),
            )
        })
        .collect();
    build_arg_values.push(("USERNAME".to_string(), user.username.clone()));
    build_arg_values.push(("UID".to_string(), uid_str));
    build_arg_values.push(("GID".to_string(), gid_str));
    build_arg_values.push(("EXTRA_DIRS".to_string(), extra_dirs));
    let build_args_refs: Vec<(&str, &str)> = build_arg_values
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let docker_args = build_universal_docker_args(
        &image_tag,
        &build_args_refs,
        &label,
        context_path,
        opts.no_cache,
    );

    docker.stream_command(docker_args)?;

    Ok(())
}

/// Construct `docker build` arguments for the universal image.
///
/// Mirrors [`crate::docker::args::build_docker_build_args`] but adds a
/// `--label` for inspectability. The context path is always the final
/// positional argument.
fn build_universal_docker_args(
    image_tag: &str,
    build_args: &[(&str, &str)],
    label: &str,
    context_path: &Path,
    no_cache: bool,
) -> Vec<String> {
    let mut args = vec!["build".to_string(), "-t".to_string(), image_tag.to_string()];

    for (key, value) in build_args {
        args.push("--build-arg".to_string());
        args.push(format!("{key}={value}"));
    }

    args.push("--label".to_string());
    args.push(format!("cast.universal.agents={label}"));

    if no_cache {
        args.push("--no-cache".to_string());
    }

    args.push(context_path.to_string_lossy().to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn versions(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn tag_changes_when_version_changes() {
        let v1 = versions(&[("opencode", "0.1.0")]);
        let v2 = versions(&[("opencode", "0.2.0")]);
        assert_ne!(
            universal_image_tag(&v1),
            universal_image_tag(&v2),
            "changing a version must change the tag"
        );
    }

    #[test]
    fn tag_is_stable_for_same_map() {
        let map = versions(&[("opencode", "0.1.0"), ("pi", "0.2.0")]);
        assert_eq!(
            universal_image_tag(&map),
            universal_image_tag(&map),
            "same map must always produce the same tag"
        );
    }

    #[test]
    fn tag_changes_when_agent_set_changes() {
        let base = versions(&[("opencode", "0.1.0")]);
        let with_pi = versions(&[("opencode", "0.1.0"), ("pi", "0.2.0")]);
        assert_ne!(
            universal_image_tag(&base),
            universal_image_tag(&with_pi),
            "adding an agent must change the tag"
        );
        assert_ne!(
            universal_image_tag(&with_pi),
            universal_image_tag(&base),
            "removing an agent must change the tag"
        );
    }

    #[test]
    fn tag_is_independent_of_insertion_order() {
        // BTreeMap is sorted by key, so insertion order must not matter.
        let mut a = BTreeMap::new();
        a.insert("opencode".to_string(), "0.1.0".to_string());
        a.insert("claudecode".to_string(), "1.0.0".to_string());
        a.insert("pi".to_string(), "0.2.0".to_string());

        let mut b = BTreeMap::new();
        b.insert("pi".to_string(), "0.2.0".to_string());
        b.insert("opencode".to_string(), "0.1.0".to_string());
        b.insert("claudecode".to_string(), "1.0.0".to_string());

        assert_eq!(
            universal_image_tag(&a),
            universal_image_tag(&b),
            "insertion order must not affect the tag"
        );
    }

    #[test]
    fn tag_uses_stable_prefix_and_hash_suffix() {
        let map = versions(&[("opencode", "0.1.0")]);
        let tag = universal_image_tag(&map);
        assert!(
            tag.starts_with(&format!("{}:{}-universal-", IMAGE_BASE, CAST_VERSION)),
            "tag must use the stable prefix, got: {tag}"
        );
        let suffix = tag.rsplit('-').next().unwrap();
        assert_eq!(
            suffix.len(),
            TAG_HASH_LEN,
            "hash suffix must be {TAG_HASH_LEN} hex chars, got: {suffix}"
        );
        assert!(
            suffix.chars().all(|c| c.is_ascii_hexdigit()),
            "hash suffix must be hex, got: {suffix}"
        );
    }

    #[test]
    fn build_args_include_per_agent_versions_label_and_context() {
        use std::path::Path;

        let build_args: &[(&str, &str)] = &[
            ("OPENCODE_VERSION", "0.1.0"),
            ("PI_VERSION", "0.2.0"),
            ("USERNAME", "alice"),
            ("UID", "1000"),
            ("GID", "1000"),
            ("EXTRA_DIRS", "\"/home/alice/.cargo\""),
        ];
        let ctx = Path::new("/tmp/build-ctx");

        let args = build_universal_docker_args(
            "localhost/cast:0.1.0-universal-abc123",
            build_args,
            "opencode=0.1.0,pi=0.2.0",
            ctx,
            true, // no_cache
        );

        // -t tag
        assert!(args.contains(&"-t".to_string()));
        assert!(
            args.contains(&"localhost/cast:0.1.0-universal-abc123".to_string()),
            "missing image tag"
        );

        // per-agent version build args
        assert!(
            args.contains(&"OPENCODE_VERSION=0.1.0".to_string()),
            "missing OPENCODE_VERSION build arg"
        );
        assert!(
            args.contains(&"PI_VERSION=0.2.0".to_string()),
            "missing PI_VERSION build arg"
        );

        // standard build args
        assert!(args.contains(&"USERNAME=alice".to_string()));
        assert!(args.contains(&"UID=1000".to_string()));
        assert!(args.contains(&"GID=1000".to_string()));
        assert!(args.contains(&"EXTRA_DIRS=\"/home/alice/.cargo\"".to_string()));

        // label
        let label_idx = args
            .iter()
            .position(|a| a == "--label")
            .expect("missing --label");
        assert_eq!(
            args[label_idx + 1],
            "cast.universal.agents=opencode=0.1.0,pi=0.2.0"
        );

        // no-cache flag present
        assert!(args.contains(&"--no-cache".to_string()));

        // context path is the LAST positional argument
        assert_eq!(args.last().unwrap(), "/tmp/build-ctx");

        // label comes before the context path
        assert!(label_idx < args.len() - 1);
    }
}
