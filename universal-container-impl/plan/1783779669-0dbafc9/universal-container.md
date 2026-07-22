---
status: open
refs: master/task/1783779669-0dbafc9/universal-container.md
---

# Universal Container — Implementation Plan

## Design Decisions (agreed)

- **Config flag**: `universal_container: bool` in `cast.json` (default `false`).
- **CLI unchanged**: `cast run opencode` still works; only the image and mounts change.
- **Agent inclusion**: only agents with a pinned version in `agent_versions` are included.
- **Image tag**: content hash — `localhost/cast:{cast_ver}-universal-{sha256_12}` where the
  hash input is the sorted, serialised agent→version map. Human-readable label added via
  Docker `--label`.
- **Volumes**: universal mode uses shared `{namespace}-universal-{cache,local}` named
  volumes (respects `volumes_namespace` config) plus the union of all included agents'
  config dir bind mounts (distinct paths — no collision).
- **Container sharing**: NOT in scope. Each `cast run <agent>` still starts its own container.
- **CMD**: universal Dockerfile uses `CMD ["bash"]` (dead code for `cast run`, ergonomic for bare docker run).

---

## Phases

### Phase 1 — Config Schema
**File**: `crates/cast/src/config/schema.rs`

- Add `#[serde(default)] pub universal_container: bool` to `Config`.
- Add `Default::default()` impl coverage (already exists, just add the field = `false`).
- Add validation helper: if `universal_container && agent_versions.is_empty()` → error.

**Tests** (unit, in `schema.rs`):
- Deserialise with `universal_container: true` → field set.
- Deserialise without field → `false` (backward compat).
- Validate: error when enabled with empty `agent_versions`.

---

### Phase 2 — Dockerfile Fragment Assembly (pure function, no Docker I/O)
**New module**: `crates/cast/src/dev/universal/`
**New assets**: `crates/cast/assets/Dockerfile.frag.{opencode,claudecode,pi}`

#### 2a — Extract installation fragments from existing Dockerfiles

Each fragment contains only the agent-specific installation steps (the `RUN curl …` /
`COPY --from=node … && npm install` blocks). Everything else (FROM, apt, Nix config,
ENV, user creation) moves to the shared preamble/postamble.

```
Dockerfile.frag.opencode   → ARG OPENCODE_VERSION + RUN curl/tar install block
Dockerfile.frag.claudecode → COPY --from=node:lts-trixie-slim /usr/local /usr/local
                             ARG CLAUDECODE_VERSION + RUN npm install block
Dockerfile.frag.pi         → ARG PI_VERSION + RUN curl/tar install block
```

Note: `claudecode`'s fragment must keep `COPY --from=node` immediately before
`npm install` — ordering is preserved because both live in the same fragment string.

#### 2b — Add `dockerfile_snippet()` to the `Agent` trait

```rust
// agent.rs
fn dockerfile_snippet(&self) -> &'static str { "" }
```

Each harness overrides with `include_str!("../../../assets/Dockerfile.frag.<agent>")`.

#### 2c — Assembly function

```rust
// dev/universal/dockerfile.rs
pub fn assemble(agents: &[&dyn Agent]) -> String
```

Structure of the assembled file:
```
[PREAMBLE]
  FROM debian:trixie-slim
  ARG TARGETARCH          ← required by opencode/pi arch detection
  RUN apt-get install ca-certificates curl git ssh   ← union of all agents' deps
  [Nix config block]
  ENV PATH=…  GC_NPROCS=1  NIX_REMOTE=daemon
[FRAGMENTS — one per included agent, in sorted order by name]
[POSTAMBLE]
  USER root
  RUN git config --system safe.directory "*"
  ENV PATH="/usr/local/bin:${PATH}"
  ARG USERNAME UID GID EXTRA_DIRS
  RUN [user + union mkdir creation]   ← mkdir includes .claude, .pi, .config/opencode
  USER ${USERNAME}
  WORKDIR /workspace
  CMD ["bash"]
```

**Tests** (unit, pure string assertions):
- `assemble(&[opencode, pi])` contains opencode fragment, pi fragment; does NOT contain
  `npm install` or `COPY --from=node`.
- `assemble(&[claudecode])` contains `COPY --from=node` BEFORE `npm install`.
- `assemble(&[opencode, claudecode, pi])` contains all three fragments.
- All assembled Dockerfiles contain `ARG TARGETARCH`.
- All assembled Dockerfiles contain `git config --system safe.directory`.

---

### Phase 3 — Universal Image Tag and Build
**New file**: `crates/cast/src/dev/universal/image.rs`

#### 3a — Tag function

```rust
pub fn universal_image_tag(agent_versions: &BTreeMap<String, String>) -> String
```

Algorithm:
1. Iterate `agent_versions` in sorted key order.
2. Build string `"agent1=ver1;agent2=ver2;…"`.
3. `sha256` of that string → take first 12 hex chars.
4. Return `format!("localhost/cast:{}-universal-{}", CAST_VERSION, short_hash)`.

#### 3b — Build function

```rust
pub fn ensure_universal_image(
    agents_with_versions: &[(&dyn Agent, &str)],
    docker: &DockerClient,
    config: &Config,
    user: &ResolvedUser,
    opts: BuildOptions,
) -> Result<()>
```

- Derives `image_tag` from the full `agent_versions` map (same hash, regardless of
  which subset of agents we received — both must be consistent, so pass the full map).
- Builds `Vec<(&str, &str)>` build args: `("{AGENT}_VERSION", ver)` per included agent
  + `USERNAME`, `UID`, `GID`, `EXTRA_DIRS` (existing pattern from `image.rs`).
- Adds `--label cast.universal.agents=<human-readable composition>`.
- Writes assembled Dockerfile to `TempDir`, calls `docker.stream_command(build_args)`.

**Tests**:
- Tag is stable for the same input map.
- Tag changes when any version changes.
- Tag changes when an agent is added or removed from the map.
- Tag is independent of insertion order.

---

### Phase 4 — Universal Volume / Mount Strategy
**New file**: `crates/cast/src/dev/universal/volumes.rs`
**Refactor**: `dev/opencode/mod.rs`, `dev/claudecode/mod.rs`, `dev/pi/mod.rs`

#### 4a — Shared cache/local volumes

```rust
pub fn build_universal_data_volume_args(config: &Config, user: &ResolvedUser) -> Vec<String>
```

Returns:
```
-v {namespace}-universal-cache:/home/{username}/.cache:rw
-v {namespace}-universal-local:/home/{username}/.local:rw
```

`namespace` = `config.volumes_namespace` (default `"cast"`).

#### 4b — Extract `config_mount_args` from each agent

Add new method to the `Agent` trait (with default no-op):

```rust
fn config_mount_args(&self, config: &Config, opts: &RunOpts) -> Result<Vec<String>> {
    Ok(vec![])
}
```

Each harness overrides to return only its config-dir bind mounts:
- `opencode` → `~/.config/opencode`
- `claudecode` → `~/.claude` + `~/.claude.json`
- `pi` → `~/.pi`

The existing `extra_run_args` on each harness calls `self.config_mount_args()` internally
(behaviour-preserving refactor for non-universal mode).

#### 4c — Universal mount composition

```rust
// dev/universal/volumes.rs
pub fn build_universal_run_args(
    included_agents: &[&dyn Agent],
    launched_agent: &dyn Agent,
    config: &Config,
    opts: &RunOpts,
    env: &HashMap<String, String>,
) -> Result<Vec<String>>
```

Returns:
1. `build_universal_data_volume_args` (shared cache/local — once).
2. Union of `config_mount_args` for every included agent.
3. Env passthrough from the launched agent (its existing `env::build_passthrough_env_args`).
4. User flake mount if present.

**Tests**:
- No two `-v` args share the same target path (`:rw` segment).
- All three config dirs present when all three agents included.
- Only opencode + pi config dirs when claudecode excluded.
- Exactly one `~/.cache` mount.
- Volume names use `volumes_namespace` from config.

---

### Phase 5 — Wire `run_agent` and `build_agent`
**Files**: `crates/cast/src/dev/run.rs`, `crates/cast/src/dev/build.rs`

#### 5a — `run_agent` branch

```rust
if config.universal_container {
    // 1. Validate: requested agent must be in agent_versions.
    if !config.agent_versions.contains_key(agent.name()) {
        anyhow::bail!(
            "{} is not included in your universal container — \
             add it to agent_versions in cast.json to include it",
            agent.name()
        );
    }
    // 2. Collect all included agents.
    let included = resolve_included_agents(config);   // agents whose name is in agent_versions
    // 3. Resolve versions for all included agents.
    let versions: Vec<(&dyn Agent, String)> = ...;
    // 4. Build/ensure the universal image.
    let tag = universal::image::universal_image_tag(&config.agent_versions);
    universal::image::ensure_universal_image(&versions, &docker, config, &user, BuildOptions::default())?;
    // 5. Call prepare_host for ALL included agents.
    for (ag, _) in &versions { ag.prepare_host(config, &run_opts)?; }
    // 6. Run container with universal mounts.
    let extra_args = universal::volumes::build_universal_run_args(...)?;
    run_in_container(&docker, agent, config, &run_opts, &container_name, &tag, cmd)
} else {
    // existing path — unchanged
}
```

#### 5b — `build_agent` branch

In universal mode, `cast build <agent>` builds the universal image:
```
eprintln!("Universal mode: building combined image ({})", human_readable_composition);
universal::image::ensure_universal_image(...)
```

**Tests**:
- In non-universal mode all existing tests stay green.
- In universal mode, error returned for agent not in `agent_versions`.
- In universal mode, universal tag used (not agent-specific tag).
- `prepare_host` called for all included agents.

---

### Phase 6 — Documentation
**Files**: `crates/cast/docs/config/overview.md`, `crates/cast/docs/agents.md`

- Document `universal_container` field.
- Document universal volume naming (`{namespace}-universal-{cache,local}`).
- Document subset selection via `agent_versions`.
- Document subprocess use case (primary motivation).

---

## File Change Summary

| File | Change |
|------|--------|
| `config/schema.rs` | Add `universal_container: bool` |
| `dev/agent.rs` | Add `dockerfile_snippet()` and `config_mount_args()` trait methods |
| `dev/run.rs` | Branch on `universal_container` in `run_agent` |
| `dev/build.rs` | Branch on `universal_container` in `build_agent` |
| `dev/opencode/mod.rs` | Extract `config_mount_args`; add `dockerfile_snippet` |
| `dev/claudecode/mod.rs` | Extract `config_mount_args`; add `dockerfile_snippet` |
| `dev/pi/mod.rs` | Extract `config_mount_args`; add `dockerfile_snippet` |
| `dev/universal/mod.rs` | New module |
| `dev/universal/dockerfile.rs` | `assemble()` + tests |
| `dev/universal/image.rs` | `universal_image_tag()`, `ensure_universal_image()` + tests |
| `dev/universal/volumes.rs` | `build_universal_data_volume_args()`, `build_universal_run_args()` + tests |
| `assets/Dockerfile.frag.opencode` | Extracted installation fragment |
| `assets/Dockerfile.frag.claudecode` | Extracted installation fragment |
| `assets/Dockerfile.frag.pi` | Extracted installation fragment |
| `docs/config/overview.md` | Document new field |
| `docs/agents.md` | Document universal container section |

## Files NOT Changed

`container_name.rs`, `port.rs`, `build_command.rs`, `args.rs`,
`build_docker_build_args` — all untouched.

---

## TDD Order

Follow the red→green→commit cycle strictly:

1. Phase 1: config field deserialisation tests → impl
2. Phase 2: `assemble()` string tests → impl (no Docker, pure functions)
3. Phase 3: tag stability/hash tests → impl; build arg construction tests → impl
4. Phase 4: volume/mount arg tests → impl; `config_mount_args` refactor tests → impl
5. Phase 5: `run_agent` branching tests (mocked docker) → impl
6. Phase 6: docs (no tests needed)

Commit at each GREEN milestone per the git-commit skill protocol.
