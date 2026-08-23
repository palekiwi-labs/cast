---
title: Investigate sharing the host Nix store with cast containers
status: complete
priority: normal
refs:
- /home/pl/code/palekiwi-labs/cast/.cue/master/task/nix-daemon-volume-version-skew.md
- /home/pl/code/palekiwi-labs/cast/.cue/master/task/harden-nix-security.md
- /home/pl/code/palekiwi-labs/cast/.cue/master/doc/1785678626-4f8ccff/research-share-host-nix-store.md
- /home/pl/code/palekiwi-labs/cast/.cue/master/task/nix-host-store-substituter.md
---
> Note: `refs` uses absolute paths matching the existing convention in this repo.

# Investigate sharing the host Nix store with cast containers

Determine whether cast can optionally reuse the host's `/nix` store for the
nix-daemon container and the development containers, eliminating the current
duplication of store contents (double download, double build, double GC)
between host and the `cast-nix` volume.

The feature, if adopted, must be **opt-in and off by default**, and must
preserve **configuration separation** between host and container environments:
only store *contents* are shared, not nix configuration, profiles, registries
or user state.

## Source

- Existing hazard this compounds:
  `.cue/master/task/nix-daemon-volume-version-skew.md`
- Related: `.cue/master/task/harden-nix-security.md`
- Historical context: an early cast prototype bind-mounted the host `/nix`;
  it was abandoned over security uncertainty and the isolation benefits of a
  dedicated volume (relocatable, size-constrainable, predictable).

## Scope of investigation

- Enumerate viable sharing architectures and their failure modes.
- Establish what "configuration separation" can and cannot mean for each.
- Assess the macOS story (store-path/system divergence, podman machine VM,
  fixed-output derivations).
- Assess interaction with nix version skew and garbage collection.
- Produce a recommendation and, if positive, a phased implementation plan.

## Acceptance Criteria

1. **A written analysis exists covering all candidate architectures.**
   - Covers, for each: dedup achieved, config separation achievable,
     security exposure, GC semantics, version-skew behaviour, platform support.
   - Verify by: research report artifact under `doc/`.
   - Evidence: `.cue/master/doc/1785678626-4f8ccff/research-share-host-nix-store.md`
     — six architectures assessed (A, B, C, D, D-reverse, E), each against all
     listed dimensions. Store semantics verified against `nix help-stores` on
     nix 2.34.6.

2. **A recommendation with a phased plan is recorded and accepted.**
   - Verify by: human attestation from the user.
   - Evidence: user (pl), 2026-08-13, in session — accepted the recommendation
     of option D (host store as read-only substituter, off by default),
     confirmed rejection of the local-overlay approach as a dealbreaker, and
     directed creation of the follow-up implementation task.

3. **Security exposure of any recommended mode is explicitly characterised.**
   - Verify by: dedicated section in the report.
   - Evidence: report section 4 (host-daemon trust model, `trusted-users` uid
     regimes, applies to options A and B only), section 3D confidentiality
     note (indirect path-oracle channel, no enumeration), and section
     3D-reverse (why the reverse substituter direction is a host code
     execution path). The recommended mode grants the container no host
     daemon access, no capabilities, no host privileges and no host writes.

## Outcome

Recommendation: implement option D only — bind-mount host `/nix` read-only
into the daemon container and register it as a low-priority, trusted,
read-only chroot-store substituter. Off by default, Linux-only.

Rejected with reasons recorded in the report: option A (two daemons on one
db, plus daemon-socket collision), option C local-overlay (needs host root
because dev containers must see the merged view, and forbids free host GC —
half the stated motivation), option D-reverse (inverts the sandbox trust
direction). Option B is out of scope while configuration separation is a
requirement.

Follow-up: `.cue/master/task/nix-host-store-substituter.md`
