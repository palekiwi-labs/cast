# The `Agent` Trait

**File:** `crates/cast/src/dev/agent.rs`

The `Agent` trait is the extension point for adding new coding-agent harnesses
to `cast`. Each harness (OpenCode, Pi, ClaudeCode, …) is a plain unit struct
that implements this trait. Everything else — image naming, build orchestration,
Nix wrapping, generic Docker security flags — is handled by shared
infrastructure that calls into the trait.

---

## Trait Definition

```rust
pub trait Agent {
    // --- Required ---

    /// Short identifier used in container names and CLI subcommands (e.g. `"opencode"`).
    fn name(&self) -> &'static str;

    /// Embedded Dockerfile content for this agent.
    fn dockerfile(&self) -> &'static str;

    /// Resolve the concrete version string from config (e.g. `"latest"` → `"1.4.7"`).
    fn resolve_version(&self, config: &Config) -> Result<String>;

    /// Agent-specific `docker run` arguments (env vars, bind mounts, named volumes, …)
    /// appended after the generic arguments assembled by `run::build_run_opts`.
    fn extra_run_args(
        &self,
        config: &Config,
        opts: &RunOpts,
        env: &HashMap<String, String>,
    ) -> Result<Vec<String>>;

    /// The binary name inside the container (e.g. `"opencode"`, `"pi"`, `"claude"`).
    fn base_command(&self) -> &'static str;

    // --- Provided (override when needed) ---

    /// Full Docker image tag: `localhost/cast:<cast-version>-<name>-<agent-version>`.
    fn image_tag(&self, version: &str) -> String { image::image_tag(self.name(), version) }

    /// Ensure the image exists locally, building it if necessary.
    fn ensure_image(&self, docker, config, user, version, opts) -> Result<()> {
        image::ensure_image(self.name(), self.dockerfile(), ...)
    }

    /// Build the full command vector passed to `docker run` (handles Nix flake wrapping).
    fn build_command(&self, config, opts, extra_args) -> Vec<String> {
        build_command::build_command(config, opts, self.base_command(), extra_args)
    }

    /// Host-side preparation before the container starts (e.g. create config dirs).
    /// Default is a no-op.
    fn prepare_host(&self, _config: &Config, _opts: &RunOpts) -> Result<()> { Ok(()) }
}
```

---

## Responsibilities by Method

| Method | Concern | Who calls it |
|---|---|---|
| `name()` | Container naming, CLI subcommand label, image tag | `run_agent`, `build_agent`, `resolve_port`, `image_tag` |
| `dockerfile()` | Image build source | `ensure_image` |
| `resolve_version()` | Version pin or latest-fetch + cache | `run_agent`, `build_agent` |
| `extra_run_args()` | Agent-specific env vars, bind mounts, named volumes | `run_agent` |
| `base_command()` | Binary entrypoint | `build_command` |
| `image_tag()` | Derived from `name()` + cast version + agent version | `run_agent`, `build_agent` |
| `ensure_image()` | Build-if-missing logic | `run_agent`, `build_agent` |
| `build_command()` | Nix flake wrapping + entrypoint assembly | `run_agent` |
| `prepare_host()` | Create host-side dirs before container starts | `run_agent` |

---

## Execution Flow

### `run_agent` (`dev/run.rs`)

1. `agent.resolve_version(config)` → concrete version string
2. `agent.image_tag(version)` → full image tag
3. `agent.ensure_image(...)` → build image if missing
4. `agent.prepare_host(config, &run_opts)` → host-side setup
5. `build_run_opts(config, &run_opts)` → generic Docker flags (security, network, workspace mount, Nix store, shadow mounts, …)
6. `agent.extra_run_args(config, &run_opts, &env)` → agent-specific flags appended
7. `agent.build_command(config, &run_opts, extra_args)` → full command vector (with optional Nix wrapping)
8. `docker.interactive_command(...)` → exec into container

### `build_agent` (`dev/build.rs`)

1. `agent.resolve_version(config)`
2. `agent.ensure_image(...)` (with `force`/`no_cache` flags)

---

## Version Resolution

All current agents delegate to `VersionResolver` (`dev/version/resolver.rs`),
which accepts either a concrete semver (`"1.4.7"`) from `config.agent_versions`
or `"latest"` (the default). For `"latest"` it checks a local TTL cache first,
then falls back to a `VersionFetcher` implementation:

| Agent | Fetcher | Source |
|---|---|---|
| `opencode` | `GithubReleaseFetcher { repo: "anomalyco/opencode" }` | GitHub Releases API |
| `pi` | `GithubReleaseFetcher { repo: "badlogic/pi-mono" }` | GitHub Releases API |
| `claudecode` (planned) | `NpmRegistryFetcher { package: "@anthropic-ai/claude-code" }` | npm registry JSON API |

Version cache files live at:
`~/.cache/cast/versions/<agent-name>-version-cache.json`

---

## Generic vs Agent-Specific Docker Arguments

`build_run_opts` (`dev/run.rs`) assembles the common flags that every agent shares:

- Security hardening: `--security-opt no-new-privileges`, `--cap-drop ALL`
- Resource limits: `--memory`, `--cpus`, `--pids-limit`
- Network: `--network`, optional `--add-host host.docker.internal:host-gateway`
- Port publishing
- Env files (`.env` from workspace root and host home dir)
- Identity env vars: `USER`, `TERM`, `COLORTERM`, `FORCE_COLOR`
- MCP URL injection: `CAST_MCP_URL`
- Nix store volume: `<nix-volume>:/nix:ro`
- Timezone: `/etc/localtime:/etc/localtime:ro`
- Workspace bind mount
- Extra data volumes (`config.extra_data_volumes`)
- Shadow mounts (empty tmpfs over `config.forbidden_paths`)
- Working directory (`--workdir`)

Each agent's `extra_run_args` then appends on top:

| Agent | Extra args |
|---|---|
| `opencode` | `ANTHROPIC_API_KEY` + `OPENCODE_*` passthrough; user flake mount; `~/.config/opencode` bind mount; `cast-opencode-cache` + `cast-opencode-local` named volumes |
| `pi` | `ANTHROPIC_API_KEY` + `PI_*` + AWS passthrough; `~/.pi` bind mount; user flake mount; `cast-pi-cache` + `cast-pi-local` named volumes |
| `claudecode` (planned) | `ANTHROPIC_API_KEY` + `CLAUDE_CODE_*` + AWS/GCP passthrough; `~/.claude` bind mount; user flake mount; `cast-claudecode-cache` + `cast-claudecode-local` named volumes |

---

## Adding a New Agent — Checklist

1. **`assets/Dockerfile.dev.<name>`** — build instructions; receives `AGENT_VERSION`, `USERNAME`, `UID`, `GID`, `EXTRA_DIRS` build-args
2. **`src/dev/<name>/mod.rs`** — unit struct + `Agent` impl; `resolve_version` free function
3. **`src/dev/<name>/env.rs`** — `PASSTHROUGH_VARS` list + `build_passthrough_env_args`
4. **`src/dev/<name>/config_dir.rs`** — `get_config_dir` / `ensure_config_dir` for the agent's host config directory
5. **`src/dev/version/fetcher.rs`** — new `VersionFetcher` impl if not already covered (e.g. `NpmRegistryFetcher`)
6. **`src/dev/mod.rs`** — `pub mod <name>;`
7. **`src/commands/cli.rs`** — add variant to `BuildAgent`, `RunAgent`, `ShellAgent`; add match arms in `run()`; add `as_agent()` arm

---

## Existing Implementations

### `OpenCode` — `src/dev/opencode/mod.rs`

```rust
pub struct OpenCode;
impl Agent for OpenCode {
    fn name(&self) -> &'static str { "opencode" }
    fn dockerfile(&self) -> &'static str { include_str!("../../../assets/Dockerfile.dev.opencode") }
    fn base_command(&self) -> &'static str { "opencode" }
    // resolve_version: GithubReleaseFetcher { repo: "anomalyco/opencode" }
    // prepare_host: ensures ~/.config/opencode exists
    // extra_run_args: OPENCODE_* env passthrough, ~/.config/opencode bind mount,
    //                 user flake mount, cast-opencode-{cache,local} volumes
}
```

### `Pi` — `src/dev/pi/mod.rs`

```rust
pub struct Pi;
impl Agent for Pi {
    fn name(&self) -> &'static str { "pi" }
    fn dockerfile(&self) -> &'static str { include_str!("../../../assets/Dockerfile.dev.pi") }
    fn base_command(&self) -> &'static str { "pi" }
    // resolve_version: GithubReleaseFetcher { repo: "badlogic/pi-mono" }
    // prepare_host: ensures ~/.pi exists
    // extra_run_args: PI_* + AWS env passthrough, ~/.pi bind mount,
    //                 user flake mount, cast-pi-{cache,local} volumes
}
```

### `ClaudeCode` — `src/dev/claudecode/mod.rs` _(planned)_

```rust
pub struct ClaudeCode;
impl Agent for ClaudeCode {
    fn name(&self) -> &'static str { "claudecode" }
    fn dockerfile(&self) -> &'static str { include_str!("../../../assets/Dockerfile.dev.claudecode") }
    fn base_command(&self) -> &'static str { "claude" }
    // resolve_version: NpmRegistryFetcher { package: "@anthropic-ai/claude-code" }
    // Base image: node:lts-trixie-slim (no arch detection needed)
    // prepare_host: ensures ~/.claude exists
    // extra_run_args: CLAUDE_CODE_* + AWS/GCP env passthrough, ~/.claude bind mount,
    //                 user flake mount, cast-claudecode-{cache,local} volumes
}
```
