# Project Log

## [ffc6b29] Project Overview Created

Created a high-level project summary for 'cast' (coding agent sandbox tool). The summary covers the project's purpose (secure agent sandboxing), its use of Docker and Nix, and its internal Rust structure.

- **Found:** 'cast' stands for coding agent sandbox tool.
- **Found:** It uses Docker for isolation and Nix for reproducible environments.
- **Found:** The project structure separates CLI commands, agent abstractions (in src/dev), and Docker interactions.

## [852f18c] Research complete: extra_dirs source field optionality

Researched the optionality of the `source` field in `VolumeConfig` and its implications for `extra_dirs.rs`. Found that it defaults based on volume type (bind vs volume) and is ignored in the directory resolution logic of `extra_dirs.rs`.

- **Found:** source defaults to target for bind mounts
- **Found:** source defaults to namespaced ID for named volumes
- **Found:** extra_dirs.rs only utilizes the target field for path resolution

## [f768db4] Research complete: Rust CLI patterns in cast project

- **Found:** Thin command handlers in src/commands/ delegating to domain logic.
- **Found:** ApprovedConfig Newtype pattern for security/validation enforcement.
- **Found:** Pure function extraction for testability without mocks.
- **Found:** Progressive discovery documentation structure (top-level vs per-crate).
- **Found:** Integration tests isolated using temp directory environment variables.

## [f768db4] Drafted general rust-cli skill content

- **Decided:** Distilled Rust CLI research into a minimal, general-purpose skill draft.
- **Decided:** Emphasized 'Pure Logic' and 'Thin Handlers' as core refactoring blueprints.

## [2ce36c4] Research complete: Cast Run Options

- **Found:** 'cast run' implementation is located in 'crates/cast/src/commands/cli.rs' and 'crates/cast/src/dev/run.rs'
- **Found:** 'build_run_opts' currently hardcodes '-it' and '-p' flags, which needs to be made conditional for headless/JSON mode
- **Found:** 'DockerClient' lacks a non-interactive execution method for clean stdout capture
- **Found:** Agent trait and 'build_command' need to be updated to pass new flags to the inner agent process

## [993e391] Research complete: offline mode for cast (nix network reduction)

- **Found:** Nix surface is concentrated in ~6 files; dominant download source is `nix develop <flake> -c` wrapping in build_command.rs, which pulls flake inputs + full devshell closure through the nix daemon.
- **Found:** The nix daemon container (cast-nix-daemon, image nixos/nix:2.34.6) is the central download engine; all agents mount its store read-only via NIX_REMOTE=daemon. Its substituters default to https://cache.nixos.org (generate_nix_conf).
- **Found:** Resolved discrepancy: use_flake defaults to false, BUT the daemon starts unconditionally on every `cast run` AND the global/user flake (~/.config/cast/nix) wrapping is gated only by file presence, not by use_flake.
- **Found:** cast NEVER issues `docker pull`; it relies on Docker pull-on-build. No --offline/--no-net/skip-build flag exists anywhere in the codebase.
- **Found:** Version resolver (resolve) is the ONE offline-resilient subsystem today — it silently falls back to a stale cache on network failure.
- **Found:** Feasibility rated MODERATE: 5 clean injection points (Cli global flag, build_command nix develop, generate_nix_conf substituters, ensure_image build gating, resolve). Changes are localized if-gating with no structural refactor.
- **Found:** KEY design constraint surfaced: nix needs TWO complementary mechanisms for full offline — empty `substituters` in nix.conf (blocks .nar binary-cache downloads) AND `--offline` on `nix develop` (blocks flake-input fetches). Neither alone is sufficient; `--offline` is a global nix option that must precede the subcommand.
- **Found:** Offline mode is fundamentally a warm-cache-only mode: it can suppress network but cannot manufacture missing closures/images; on a cold machine it must fail fast with clear errors rather than stall.
- **Found:** Softer lower-risk alternatives exist at the same injection points: emit connect-timeout in nix.conf, or apply --offline only to nix develop.
- **Open:** Whether empty `substituters =` in nix.conf also suppresses github/tarball flake fetchers or only .nar downloads depends on nix's own semantics (not cast) and was not conclusively determined from cast source.
- **Open:** Docker's pull-on-build policy for present-but-stale images depends on Docker daemon settings, not cast.
- **Open:** No explicit 'skip build, require image present' flag exists on BuildAgent today (only --force/--no-cache); offline mode effectively needs one.

