# Plan: Add `claudecode` Agent to cast

## Goal

Add a third agent harness `claudecode` to `cast`, following the same structural
pattern as `opencode` and `pi`. ClaudeCode is installed via npm
(`@anthropic-ai/claude-code`).

## Installation Method: npm

Chosen over Debian APT because:
- Clean JSON version API: `GET https://registry.npmjs.org/@anthropic-ai/claude-code/latest`
- Pinning: `npm install -g @anthropic-ai/claude-code@<VERSION>`
- No GPG keying / sources.list setup in the Dockerfile
- Fits naturally into the existing `VersionFetcher` + `VersionResolver` infrastructure

---

## Files to Create

### `crates/cast/assets/Dockerfile.dev.claudecode`

Base image: `node:lts-trixie-slim` (Node LTS on Debian trixie, slim variant,
~78–80 MB compressed). Node + npm are pre-installed by the official image;
no `apt install nodejs npm` step needed.

Key differences from the other agent Dockerfiles:
- `FROM node:lts-trixie-slim` instead of `FROM debian:trixie-slim`
- No architecture detection needed (npm install is arch-agnostic)
- `CMD ["claude"]`
- git config: `user.name "claudecode"`, `user.email "claudecode@local"`
- Create `~/.claude` directory (bind-mounted at runtime)

```dockerfile
FROM node:lts-trixie-slim

ARG AGENT_VERSION=latest

RUN npm install -g @anthropic-ai/claude-code@${AGENT_VERSION}

# Nix config (store mounted from nix-daemon volume)
RUN mkdir -p /etc/nix && \
    echo "experimental-features = nix-command flakes" > /etc/nix/nix.conf

ENV PATH="/nix/var/nix/profiles/default/bin:${PATH}"
ENV GC_NPROCS=1
ENV NIX_REMOTE=daemon

ARG USERNAME=user
ARG UID=1000
ARG GID=1000
ARG EXTRA_DIRS=""

USER root

RUN     git config --system user.name "claudecode" && \
    git config --system user.email "claudecode@local" && \
    git config --system init.defaultBranch "main" && \
    git config --system commit.gpgsign false && \
    git config --system core.autocrlf input && \
    git config --system pull.rebase false

# Create user + directories
RUN set -e; \
    if getent group ${GID} >/dev/null 2>&1; then \
        GROUP_NAME=$(getent group ${GID} | cut -d: -f1); \
    else \
        groupadd -g ${GID} --non-unique ${USERNAME} 2>/dev/null || true; \
        GROUP_NAME=${USERNAME}; \
    fi && \
    if ! getent passwd ${USERNAME} >/dev/null 2>&1; then \
        useradd -u ${UID} -g ${GID} -m -d /home/${USERNAME} --non-unique -s /bin/bash ${USERNAME} 2>/dev/null || true; \
    fi && \
    mkdir -p /home/${USERNAME} && \
    mkdir -p /workspace \
             /home/${USERNAME}/.cache \
             /home/${USERNAME}/.claude \
             /home/${USERNAME}/.config \
             /home/${USERNAME}/.local \
             ${EXTRA_DIRS} && \
    chown -R ${UID}:${GID} /workspace \
                           /home/${USERNAME}/.cache \
                           /home/${USERNAME}/.claude \
                           /home/${USERNAME}/.config \
                           /home/${USERNAME}/.local \
                           /home/${USERNAME} \
                           ${EXTRA_DIRS} 2>/dev/null || true

USER ${USERNAME}
WORKDIR /workspace

CMD ["claude"]
```

---

### `crates/cast/src/dev/claudecode/mod.rs`

```rust
pub mod config_dir;
pub mod env;

use std::collections::HashMap;
use anyhow::{Context, Result};
use crate::config::Config;
use crate::dev::agent::Agent;
use crate::dev::run::RunOpts;
use crate::dev::version::fetcher::NpmRegistryFetcher;
use crate::dev::version::{self, VersionResolver};
use crate::user::ResolvedUser;

pub fn resolve_version(config: &Config) -> Result<String> {
    let requested = config
        .agent_versions
        .get("claudecode")
        .map(|s| s.as_str())
        .unwrap_or("latest");
    let cache_path = version::cache::get_cache_path("claudecode");
    let resolver = VersionResolver::new(cache_path, config.version_cache_ttl_hours);
    let fetcher = NpmRegistryFetcher { package: "@anthropic-ai/claude-code" };
    resolver.resolve(requested, &fetcher)
}

pub struct ClaudeCode;

impl Agent for ClaudeCode {
    fn name(&self) -> &'static str { "claudecode" }

    fn dockerfile(&self) -> &'static str {
        include_str!("../../../assets/Dockerfile.dev.claudecode")
    }

    fn resolve_version(&self, config: &Config) -> Result<String> {
        resolve_version(config)
    }

    fn prepare_host(&self, _config: &Config, opts: &RunOpts) -> Result<()> {
        let home = opts.host_home_dir.as_deref()
            .context("Failed to resolve user home directory")?;
        config_dir::ensure_config_dir(home)?;
        Ok(())
    }

    fn base_command(&self) -> &'static str { "claude" }

    fn extra_run_args(
        &self,
        config: &Config,
        opts: &RunOpts,
        env: &HashMap<String, String>,
    ) -> Result<Vec<String>> {
        let mut args = env::build_passthrough_env_args(env);

        // ~/.claude bind mount
        let home = opts.host_home_dir.as_deref()
            .context("Failed to resolve user home directory")?;
        let claude_config_dir = config_dir::get_config_dir(home);
        args.extend([
            "-v".to_string(),
            format!(
                "{}:/home/{}/.claude:rw",
                claude_config_dir.display(),
                opts.user.username
            ),
        ]);

        // User flake mount
        let user_flake_host_dir = opts.host_home_dir.as_ref()
            .filter(|h| h.join(".config/cast/nix/flake.nix").exists())
            .map(|h| h.join(".config/cast/nix"));
        if let Some(flake_dir) = &user_flake_host_dir {
            args.extend([
                "-v".to_string(),
                format!(
                    "{}:/home/{}/.config/cast/nix:rw",
                    flake_dir.display(),
                    opts.user.username
                ),
            ]);
        }

        // Persistent data volumes
        args.extend(build_data_volume_args(config, &opts.user));

        Ok(args)
    }
}

fn build_data_volume_args(cfg: &Config, user: &ResolvedUser) -> Vec<String> {
    let namespace = &cfg.volumes_namespace;
    let username = &user.username;
    vec![
        "-v".to_string(),
        format!("{}-claudecode-cache:/home/{}/.cache:rw", namespace, username),
        "-v".to_string(),
        format!("{}-claudecode-local:/home/{}/.local:rw", namespace, username),
    ]
}
```

---

### `crates/cast/src/dev/claudecode/env.rs`

```rust
use std::collections::HashMap;

pub const PASSTHROUGH_VARS: &[&str] = &[
    // LLM Provider API Keys
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "GOOGLE_GENERATIVE_AI_API_KEY",
    // Claude Code specific
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "ANTHROPIC_BASE_URL",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
    "CLAUDE_CODE_MAX_OUTPUT_TOKENS",
    // AWS Bedrock
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_REGION",
    "AWS_PROFILE",
    // Google Vertex
    "GOOGLE_APPLICATION_CREDENTIALS",
    "GOOGLE_CLOUD_PROJECT",
    "CLOUD_ML_REGION",
];

pub fn build_passthrough_env_args(env: &HashMap<String, String>) -> Vec<String> {
    PASSTHROUGH_VARS
        .iter()
        .filter(|&&var| env.contains_key(var))
        .flat_map(|&var| ["-e".to_string(), var.to_string()])
        .collect()
}
```

---

### `crates/cast/src/dev/claudecode/config_dir.rs`

```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_config_dir(base: &Path) -> PathBuf {
    base.join(".claude")
}

pub fn ensure_config_dir(base: &Path) -> Result<PathBuf> {
    let config_dir = get_config_dir(base);
    fs::create_dir_all(&config_dir).with_context(|| {
        format!("Failed to create config directory at {}", config_dir.display())
    })?;
    Ok(config_dir)
}
```

---

## Files to Modify

### `crates/cast/src/dev/version/fetcher.rs`

Add `NpmRegistryFetcher` alongside the existing `GithubReleaseFetcher`:

```rust
pub struct NpmRegistryFetcher {
    pub package: &'static str,
}

impl VersionFetcher for NpmRegistryFetcher {
    fn fetch_latest_version(&self) -> Result<String> {
        #[derive(serde::Deserialize)]
        struct NpmLatest { version: String }
        let url = format!("https://registry.npmjs.org/{}/latest", self.package);
        let dist: NpmLatest = ureq::get(&url)
            .set("User-Agent", "cast")
            .call()
            .context("Failed to reach npm registry")?
            .into_json()
            .context("Failed to parse npm registry response")?;
        Ok(dist.version)
    }
}
```

### `crates/cast/src/dev/mod.rs`

Add:
```rust
pub mod claudecode;
```

### `crates/cast/src/commands/cli.rs`

1. Add import: `use crate::dev::claudecode::ClaudeCode;`

2. Add to `BuildAgent` enum:
```rust
/// Build the ClaudeCode agent's Docker image
Claudecode {
    #[arg(long)] base: bool,
    #[arg(short, long)] force: bool,
    #[arg(long)] no_cache: bool,
},
```

3. Add to `RunAgent` enum:
```rust
/// Start an interactive ClaudeCode session
#[command(alias = "c", disable_help_flag = true)]
Claudecode {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
    extra_args: Vec<String>,
},
```

4. Add to `ShellAgent` enum:
```rust
/// Drop into an interactive shell in the ClaudeCode container
Claudecode,
```

5. Add to `RunAgent::as_agent()`:
```rust
RunAgent::Claudecode { .. } => &ClaudeCode,
```

6. Add match arms in `run()`:
```rust
// Build
Some(Commands::Build { agent: BuildAgent::Claudecode { base, force, no_cache } }) => {
    let approved = verify_config(cfg)?;
    dev::build_agent(&ClaudeCode, &approved, base, force, no_cache)?;
    Ok(ExitCode::SUCCESS)
}
// Run  (extend extra_args match)
RunAgent::Claudecode { extra_args } => extra_args.clone(),
// Shell
Some(Commands::Shell { agent: ShellAgent::Claudecode }) => {
    let approved = verify_config(cfg)?;
    let status = dev::shell(&ClaudeCode, &approved)?;
    Ok(to_exit_code(status))
}
```

---

## Open Questions (resolved before implementation)

1. **ClaudeCode config path** — The plan uses `~/.claude` (home-relative, not
   `~/.config/claude`). This matches the known default on Linux/macOS. Confirm
   this is correct before wiring the bind mount.

2. **PASSTHROUGH_VARS completeness** — The list above covers the documented
   environment variables. Any project-specific or enterprise vars to add?

3. **npm version format** — npm registry returns bare semver (`"1.2.3"`) without
   a `v` prefix. `normalize_version()` already handles this correctly (it's a
   no-op for bare semver). No changes needed to the resolver.

4. **Node.js base image** — ✅ Resolved: `FROM node:lts-trixie-slim`. Tracks the
   active Node LTS line (currently 24.x). Official image, slim variant, ~78–80 MB
   compressed, multi-arch (amd64 + arm64).

---

## Testing Checklist (per new file)

- `claudecode/config_dir.rs` — unit tests: `get_config_dir`, `ensure_config_dir` (mirrors pi tests)
- `claudecode/env.rs` — unit tests: passthrough filters correctly (mirrors opencode/pi tests)
- `claudecode/mod.rs` — unit tests:
  - `test_extra_run_args_includes_claude_config_mount`
  - `test_extra_run_args_includes_data_volumes`
  - `test_extra_run_args_user_flake_absent`
  - `test_image_tag_format`
  - `test_dockerfile_has_correct_base_image`
- `version/fetcher.rs` — unit test for `NpmRegistryFetcher` (mock or integration)
- `cli_test.rs` — add `claudecode` to any CLI smoke tests that enumerate agents

---

## Summary of Touchpoints

| File | Action |
|---|---|
| `assets/Dockerfile.dev.claudecode` | **Create** |
| `src/dev/claudecode/mod.rs` | **Create** |
| `src/dev/claudecode/env.rs` | **Create** |
| `src/dev/claudecode/config_dir.rs` | **Create** |
| `src/dev/version/fetcher.rs` | **Modify** — add `NpmRegistryFetcher` |
| `src/dev/mod.rs` | **Modify** — add `pub mod claudecode` |
| `src/commands/cli.rs` | **Modify** — 3 enums + 3 match arms + 1 import |
