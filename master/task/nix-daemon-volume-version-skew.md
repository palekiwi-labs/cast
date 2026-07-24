---
title: nix-daemon image / /nix volume version-skew migration
status: open
priority: normal
refs: /home/pl/code/palekiwi-labs/cast/.cue/master/task/nix-native-harnesses.md
---
# nix-daemon image / /nix volume version-skew migration

Address the latent coupling between the versioned nix-daemon image and the
persistent, unversioned `/nix` volume.

## Problem

- The daemon image tag is cast-version-coupled:
  `localhost/cast-nix-daemon:{CARGO_PKG_VERSION}`
  (`crates/cast/src/nix_daemon/image.rs:13`). Every cast version bump builds a
  new daemon image (`daemon.rs:27-31`).
- The `/nix` volume (`cast-nix`, `daemon.rs:55`) is persistent and unversioned.
  Docker only populates a named volume from the image's `VOLUME /nix`
  (`assets/Dockerfile.nix-daemon:7`) contents when the volume is **empty**
  (first creation). On upgrade the old volume is reused as-is.
- The daemon base is `FROM nixos/nix:2.34.6`
  (`assets/Dockerfile.nix-daemon:1`). As long as this tag is unchanged, a new
  daemon image contains the same nix, so reused-volume paths stay valid.
- **Hazard:** the first time the `nixos/nix` base tag is bumped while an old
  `/nix` volume exists, the new daemon expects store paths the old volume never
  received — e.g. the `bash` path linked in the nix profile — and things break.

## Scope

Investigate and implement a safe strategy, e.g. one of:

- Version/namespace-key the nix volume so a base-image change starts a fresh
  volume.
- Detect nix-version skew between image and volume on daemon startup and warn or
  repopulate.
- A migration/repopulation step when the base image changes.

Not required for the 0.2.0 nix-native pivot — that bump keeps
`nixos/nix:2.34.6` pinned and is therefore safe. This task unblocks *future*
nix base-image upgrades.

## Acceptance Criteria

1. **Base-image bump does not break running containers.**
   - Verify by: bump `nixos/nix` base tag, run `cast run <agent>` on a host that
     had a pre-existing `/nix` volume; container starts and core paths (bash,
     profile) resolve.
   - Evidence: (paste)

2. **Tests green.**
   - Verify by: `cargo test` in `crates/cast`.
   - Evidence: (paste exit code)
