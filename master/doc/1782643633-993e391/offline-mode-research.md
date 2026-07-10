# Research: Offline Mode for `cast` (Nix Network Reduction)

This report answers the two-phase research task in
`.cue/master/task/1782643633-993e391/research-offline-mode.md`:
(1) map every place `cast` uses nix and the conditions under which nix attempts
network calls/downloads, and (2) analyze the feasibility of an optional
flag-enabled "offline" mode that reduces or eliminates nix-initiated network
calls.

It builds on — but is distinct from — the earlier
`.cue/feat-optional-nix/doc/1780761606-73dff8d/nix-daemon-disabling-complexity.md`,
which analyzed *fully removing* nix (`"nix": false`). Offline mode keeps nix
running but suppresses network activity.

## Research Questions
1. Where does `cast` interact with nix, and which interactions trigger network
   calls / large downloads, and under what conditions?
2. How feasible is an optional `--offline` flag that reduces or eliminates
   nix-initiated network calls?

## Executive Summary
- `cast`'s nix surface is concentrated in ~6 files. The dominant source of
  "large downloads at launch" is **`nix develop <flake> -c`** wrapping
  (`crates/cast/src/dev/build_command.rs`), which triggers flake-input fetches
  and full devshell-closure downloads through the **nix daemon**
  (`crates/cast/src/nix_daemon/`).
- The nix daemon (`cast-nix-daemon` container) is the **central download
  engine**: all agent containers mount its store read-only and talk to it via
  `NIX_REMOTE=daemon`. Its substituters default to `https://cache.nixos.org`
  (`generate_nix_conf`).
- Docker provisioning (`ensure_image` in `crates/cast/src/dev/image.rs`) is the
  other major download source: base-image pulls (`debian:trixie-slim`,
  `nixos/nix:2.34.6`), `apt-get`, and `curl`'d agent binaries during `docker build`.
- `cast` **never issues `docker pull`** explicitly; it relies on Docker's
  implicit pull-on-build. There is **no existing `--offline` / `--no-net` flag**
  anywhere in the codebase.
- The version resolver (`resolve`) **already degrades gracefully** to a stale
  cache on network failure — the one subsystem that is effectively offline-ready
  today.
- **Feasibility: MODERATE.** There are 5 clean injection points and the changes
  are localized "if-gating." The main subtlety is that nix needs **two
  complementary mechanisms** to be fully offline: empty `substituters` in
  `nix.conf` (blocks binary-cache downloads) AND `--offline` on `nix develop`
  (blocks flake-input fetches). Neither alone is sufficient.

## Phase 1: Complete Map of Nix Usage & Network Conditions

### 1.1 Integration Points

| # | File | Symbol | Operation | Network Risk | Trigger Condition |
|---|------|--------|-----------|--------------|-------------------|
| 1 | `crates/cast/src/dev/build_command.rs` | `build_command` | `nix develop <flake> -c` (project flake) | **High** | `config.use_flake == true` + flake ref resolves |
| 2 | `crates/cast/src/dev/build_command.rs` | `build_command` | `nix develop <global_flake> -c` (user/global) | **High** | `opts.user_flake_present` (file exists) — **NOT gated by `use_flake`** |
| 3 | `crates/cast/src/nix_daemon/daemon.rs` | `ensure_running` | `docker run -d cast-nix-daemon` (image `nixos/nix:2.34.6`) | **Medium** (image pull if missing) | Every `cast run` (unconditional) |
| 4 | `crates/cast/src/nix_daemon/config.rs` | `generate_nix_conf` | Emits `substituters = https://cache.nixos.org ...` | **High (indirect)** — defines the download endpoint | Daemon startup |
| 5 | `crates/cast/src/dev/run.rs` | `build_docker_run_flags` | Mounts `config.nix_volume_name:/nix:ro` | None (local bind) | Every agent container start |
| 6 | `crates/cast/src/dev/image.rs` | `ensure_image` | `docker build` (base images, apt, curl) | **Medium-High** | Image not present locally (or `--force`) |
| 7 | `crates/cast/src/dev/version/resolver.rs` | `resolve` | GitHub/npm API fetch | **Low** (<10KB; has cache fallback) | Version is `"latest"` + 24h cache expired |
| 8 | `crates/cast/src/commands/nix_daemon.rs` | `handle_nix_daemon` | Manual `Build`/`Shell`/`Start`/`Stop` | Variable | Explicit user invocation of `cast nix-daemon` |
| 9 | `crates/cast/assets/Dockerfile.dev.*` | `ENV` | `NIX_REMOTE=daemon`, `PATH=/nix/...` | None (configures client) | Image build time |

### 1.2 Resolved Discrepancy: `use_flake` Default
- `config.use_flake` defaults to **`false`** (`Config::Default` in
  `crates/cast/src/config/schema.rs`).
- Therefore, by default `cast` does **not** wrap project commands in `nix develop`.
- **However**, two nix-touching things happen regardless of `use_flake`:
  1. The nix daemon container is started **unconditionally** on every `cast run`
     (`ensure_running`).
  2. The **global/user flake** (`~/.config/cast/nix`) wrapping is gated **only by
     file presence** (`opts.user_flake_present`), not by `use_flake`. If a user
     has placed a global flake, every command is double-wrapped in `nix develop`.

  Practical implication: a user on a bad network who has *not* opted into flakes
  and has *no* global flake still pays: (a) the daemon-container image pull if
  uncached, and (b) any docker image build. The massive closure downloads only
  occur once `use_flake` is enabled or a global/project flake is present.

### 1.3 Conditions Under Which Nix Attempts Network Calls
1. **Cold nix store** — any path requested by `nix develop` not present in the
   shared `/nix` volume → daemon fetches it from `substituters`.
2. **Uncached/unlocked flake inputs** — flake refs (e.g. `github:...`,
   `path:...`, tarballs) not present in the local flake registry → nix fetcher
   downloads them. (Note: flake-input fetch is governed by nix's `--offline`/
   `--refresh`, which is **separate** from substituters.)
3. **Daemon image missing** — `nixos/nix:2.34.6` (and the derived
   `cast-nix-daemon` image) not present → Docker pulls during `ensure_running`.
4. **`docker build` triggered** — base image (`debian`, `nixos/nix`) absent, or
   `--force`/`--no-cache` used → Docker's pull-on-build plus `apt-get`/`curl`
   steps inside the Dockerfile.
5. **`"latest"` agent version + expired 24h cache** — resolver hits
   `api.github.com` / `registry.npmjs.org`.

### 1.4 Network-Heavy Operations (Ranked)
1. **Nix devshell closure** via `nix develop` — hundreds of MB to multiple GB.
   This is the headline "large download at launch" complaint.
2. **Docker base images** (`debian:trixie-slim`, `nixos/nix:2.34.6`) — ~50–100MB
   each.
3. **Agent binary downloads** (OpenCode etc. via `curl` in Dockerfile) — ~20–50MB.
4. **`apt-get install`** in Dockerfiles — ~10–30MB.
5. **Version-metadata fetches** — <10KB, and already cache-backed.

## Phase 2: Offline Mode Feasibility Analysis

### 2.1 There Is No Existing Offline Support
Confirmed: no `--offline`, `--no-net`, empty-substituter, or skip-build path
exists anywhere in the source. The only offline-resilient subsystem is the
version resolver, which silently falls back to a stale cache
(`crates/cast/src/dev/version/resolver.rs` `resolve`).

### 2.2 Two Complementary Nix Mechanisms Are Required
This is the single most important design constraint. Nix's network activity
splits across two independent channels:

| Channel | What it downloads | Controlled by | Where in `cast` |
|---------|-------------------|---------------|-----------------|
| **Substituters** (binary caches) | Built store paths (.nar) | `substituters` in `nix.conf` | `generate_nix_conf` (`nix_daemon/config.rs`) |
| **Flake fetchers** | Flake inputs (`github:...`, tarballs) | `--offline` / `--refresh` global flags on the `nix` invocation | `build_command` (`dev/build_command.rs`) |

**Emptying `substituters` alone does NOT prevent flake-input fetches**, and
**passing `--offline` alone does NOT by itself empty substituters** (though
`--offline` does instruct nix to avoid the registry). A robust offline mode must
address both. Treating this as a single switch is the most common pitfall.

> Note on `--offline` placement: it is a nix **global** option that must appear
> **before** the subcommand: `nix --offline develop <flake> -c ...`. The current
> `cmd.extend(["nix", "develop", ...])` pattern in `build_command` would need the
> flag inserted between `nix` and `develop`.

### 2.3 Injection Points (Where Logic Would Branch)
For a hypothetical `offline: bool` resolved from a CLI flag, the precise branch
locations are:

| Subsystem | File | Function | Behavior When `offline` |
|-----------|------|----------|-------------------------|
| CLI | `crates/cast/src/commands/cli.rs` | `Cli` struct | Add a `global = true` `--offline` clap arg (the top-level `Cli` only currently holds `command: Option<Commands>`, so a global arg is the natural fit) |
| Nix wrapping | `crates/cast/src/dev/build_command.rs` | `build_command` | Emit `nix --offline develop ...` for **both** the global-flake path (line ~25, ungated by `use_flake`) and the project-flake path (line ~36, gated by `use_flake`) |
| Nix daemon config | `crates/cast/src/nix_daemon/config.rs` | `generate_nix_conf` | Initialize `substituters` as empty (and optionally emit `connect-timeout`); currently hardcodes `["https://cache.nixos.org"]` then extends with `nix_extra_substituters` |
| Docker build | `crates/cast/src/dev/image.rs` | `ensure_image` | The skip-check `if !opts.force && docker.image_exists(...)` already exists; in offline mode, if the image is missing, **fail fast with a clear "offline: image not cached" error** rather than attempting a build that will pull |
| Version resolver | `crates/cast/src/dev/version/resolver.rs` | `resolve` | Skip the network `fetch_latest_version()` and go straight to the existing stale-cache fallback branch (already implemented and proven) |

### 2.4 What Offline Mode Can and Cannot Achieve
- **Can suppress**: substituter (binary-cache) downloads; flake-input fetches;
  agent-version metadata fetches; attempts to build/pull a missing docker image.
- **Cannot manufacture** missing data. Offline mode is fundamentally a
  **"warm-cache-only"** mode: it succeeds only when (a) the nix store already
  holds the devshell closure, (b) flake inputs are already fetched/locked, (c)
  the agent docker image is already built, and (d) a version is pinned or a
  cache entry exists. On a truly cold machine, offline mode will **fail fast**
  rather than hang on a congested network — which is arguably the desired UX
  (clear error vs. multi-minute stall).
- **Daemon image bootstrap**: if `nixos/nix:2.34.6` / `cast-nix-daemon` is not
  cached, `ensure_running` cannot proceed offline. Offline mode must detect this
  and report it distinctly.

### 2.5 Flag-Flow / Coupling Considerations
- `build_command` takes `&Config` + `&RunOpts`; `generate_nix_conf` takes
  `&Config`; `ensure_image` takes `&Config` + `&BuildOpts` (via `opts`). A CLI
  `--offline` flag is a **runtime** concern (per-invocation), not a persisted
  config value. The natural plumbing is to resolve the flag at the top-level
  command handler and thread it through the existing `*Opts` structs (or fold it
  into `Config` for the duration of the run). No structural refactors are
  required — only additional parameter passing.
- The global-flake wrapping (ungated by `use_flake`) means offline handling in
  `build_command` must cover **both** branches, not just the project-flake one.

### 2.6 Softer Alternatives (Lower-Effort, Partial Relief)
Even without a full offline flag, the following are independently useful on
low-bandwidth networks and live in the same injection points:
- Emitting `connect-timeout = <n>` in `generate_nix_conf` → fast-fail instead of
  long stalls on unreachable caches.
- Adding `--offline` only to `nix develop` (without touching substituters) →
  blocks flake-input refetches while still allowing cached-path substitution.
- These are strictly weaker than full offline mode but have lower risk of
  breaking a warm-but-not-fully-cached machine.

## Open Questions / Lower-Confidence Areas
1. **Flake fetcher behavior under empty substituters** — whether `substituters =`
   (empty) in `nix.conf` also suppresses the github/tarball flake fetchers, or
   only the `.nar` binary-cache downloads, was not conclusively determinable
   from `cast`'s source alone (it depends on nix's own semantics, not cast).
   Section 2.2 assumes the conservative answer (two mechanisms needed).
2. **Docker pull-on-build determinism** — `cast` issues no explicit `docker
   pull`; it relies on Docker pulling base images during `build` if absent. Whether
   Docker re-checks the registry for a *present but stale* image (and thus makes
   a network call) depends on Docker daemon settings (`--pull` policy), not cast.
3. **Daemon container as substituter-only engine** — confirming that *all* agent
   nix traffic flows through the daemon (via `NIX_REMOTE=daemon`) and that no
   agent directly contacts substituters would require inspecting the full
   Dockerfile ENV set; the sampled Dockerfiles (`Dockerfile.dev.pi`,
   `Dockerfile.dev.opencode`) both set `NIX_REMOTE=daemon`, supporting this.
4. **`--no-build` / image-skip semantics** — only `--force` and `--no-cache`
   exist on `BuildAgent` today; there is no "skip build, require image present"
   flag, which offline mode effectively needs (section 2.3).

## Sourced Files
- `crates/cast/src/dev/build_command.rs` — `build_command` (nix wrapping)
- `crates/cast/src/dev/run.rs` — `build_docker_run_flags` (volume mounts, run flags)
- `crates/cast/src/dev/image.rs` — `ensure_image` (docker build gating)
- `crates/cast/src/dev/version/resolver.rs` — `resolve` (cache-fallback version resolution)
- `crates/cast/src/nix_daemon/daemon.rs` — `ensure_running` (daemon container lifecycle)
- `crates/cast/src/nix_daemon/config.rs` — `generate_nix_conf` (substituters config)
- `crates/cast/src/commands/nix_daemon.rs` — `handle_nix_daemon` (manual nix ops)
- `crates/cast/src/commands/cli.rs` — `Cli`, `BuildAgent` (clap definitions)
- `crates/cast/src/config/schema.rs` — `Config` / `Default` (nix-related config fields)
- `crates/cast/assets/Dockerfile.dev.*` — `ENV NIX_REMOTE=daemon`, PATH injection
- `crates/cast/src/docker/args.rs` — `build_docker_build_args`, `build_run_args`

## Prior Work (Git History)
- Branch `feat-optional-nix` (commit `bdc6e44` and related) — documentation-only
  exploration of fully disabling nix (`"nix": false`), not offline mode.
- Commit `b2dd83f` — "test(pi): remove resolve_version test for network isolation,"
  indicating prior awareness of network dependencies in the test/CI environment.
