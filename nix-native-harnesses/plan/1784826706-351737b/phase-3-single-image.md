---
status: complete
refs: .cue/nix-native-harnesses/plan/index.md
---
# Phase 3 — Strip Docker-image assembly; single plain image

## Foreword

Executes Phase 3 of the nix-native-harnesses master plan. Collapses the
per-agent Docker images into a single harness-free dev image built from one
shared Dockerfile. Harnesses now come exclusively from the global Nix devShell,
so the image no longer bakes or version-pins any harness.

Decision beyond the master plan's literal "Dies" list: the version-resolution
machinery (`dev/version/`, per-agent `resolve_version`, GitHub/npm fetchers) is
removed. Its sole purpose was selecting a harness version to install in Docker;
with harness installs gone it is dead code and contradicts "versions pinned by
flake.lock". `config.version_cache_ttl_hours` stays in the schema (unused) to
avoid schema churn this phase.

## Steps

- [x] RED: `image::image_tag()` returns plain `localhost/cast:{CAST_VERSION}`
- [x] RED: shared `Dockerfile.dev` has accept-flake-config, union mkdir
  (.claude/.pi), and NO harness install URLs
- [x] GREEN: add `assets/Dockerfile.dev` (union, harness-free,
  accept-flake-config=true, union config mkdir)
- [x] GREEN: rewrite `dev/image.rs` — `image_tag()` no args; `ensure_dev_image`
  builds the shared Dockerfile (USERNAME/UID/GID/EXTRA_DIRS only)
- [x] GREEN: slim `Agent` trait — drop `dockerfile`, `dockerfile_snippet`,
  `resolve_version`, `image_tag`, `ensure_image`
- [x] GREEN: update `run.rs`, `exec.rs`, `build.rs` to call
  `image::image_tag()` / `image::ensure_dev_image()`
- [x] GREEN: delete `universal/image.rs`, `universal/dockerfile.rs`; update
  `universal/mod.rs`
- [x] GREEN: delete assets `Dockerfile.frag.*`, `Dockerfile.dev.{opencode,pi,claudecode}`
- [x] GREEN: strip per-agent `dockerfile`/`resolve_version` impls + obsolete tests
- [x] GREEN: delete `dev/version/` module + `pub mod version`
- [x] `cargo test -p cast`, `cargo clippy -p cast`, `cargo fmt` — clean
- [x] Commit + cue-log
