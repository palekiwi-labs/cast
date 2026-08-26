# Project Log

## [4f8ccff] Investigate reusing host Nix store for cast containers

Analysed four candidate architectures for sharing the host /nix with the cast nix-daemon and dev containers. Report at .cue/master/doc/1785678626-4f8ccff/research-share-host-nix-store.md, task card at .cue/master/task/share-host-nix-store.md. No code changed.

- **Found:** cast's current design is already a shared-store/single-daemon topology (cast-nix volume rw in daemon, ro in dev containers, socket inside the volume) - host sharing is a change of store source, not an architectural rewrite
- **Found:** A Nix store cannot be shared by mounting /nix/store alone: path validity lives in /nix/var/nix/db/db.sqlite, so without the db the container rebuilds everything anyway
- **Found:** A store must have exactly one writer; any design with two nix-daemons on one store is a corruption/GC-race hazard, not a config choice
- **Found:** Dockerfile.dev:61 creates the container user with the host uid (useradd -u ${UID}); if the host has trusted-users = @wheel or the username, mounting the host daemon socket silently grants every agent container trusted-user authority over the host store - full compromise path
- **Found:** Host here is nix 2.34.6 with trusted-users = root, socket mode 0666; local-overlay-store experimental feature is available in 2.34.6 (verified)
- **Found:** macOS: FODs (source tarballs, individual bundlerEnv gem fetches) are system-independent and would be shared, but no built output would be, and the podman VM cannot practically bind-mount the darwin /nix anyway
- **Found:** Option B (host daemon socket) cannot satisfy the config-separation requirement: substituters/trusted-keys/sandbox are daemon-side and become the host's, so nix_extra_substituters stops being authoritative
- **Decided:** Recommend sequencing: resolve nix-daemon-volume-version-skew first, then ship host-store-as-substituter (chroot store URI, off by default) as the cheap safe win
- **Decided:** Reject option A (host /nix mounted rw with cast's own daemon) as a dead end: two daemons on one db plus a daemon-socket path collision that breaks host nix
- **Decided:** Treat local-overlay-store as the only option delivering both full dedup and full config separation, but gate it behind an experimental flag due to CAP_SYS_ADMIN and host-GC dangling-reference risk
- **Decided:** Gate any sharing mode to Linux hosts; keep the volume-based store as the permanently supported default
- **Open:** Which cost dominates for the user: rebuild time, or disk footprint and double GC? This selects between substituter-only and the overlay approach
- **Open:** Is CAP_SYS_ADMIN on the cast-nix-daemon container acceptable?
- **Open:** Must cast keep enforcing nix_extra_substituters in shared mode? If yes, the host-daemon-socket option is disqualified
- **Open:** Desired behaviour when the host GCs a path a stopped container's store still references

## [c1980fe] GREEN: env_passthrough helper module committed

Added dev::env_passthrough with build_env_passthrough_args_from (pure) and
build_env_passthrough_args (std::env wrapper). 7 unit tests cover: prefix
stripping, non-prefixed ignored, empty suffix skipped, single-underscore
precision, sorted multi-var output, special chars preserved, empty input.
Fixed an unstable-feature use (str_as_str) by dropping a redundant
.as_str() call. Not yet wired into build_docker_run_flags.

- **Found:** Rust toolchain in this repo lacks the unstable str_as_str feature; avoid .as_str() on &str.
- **Found:** Config has no deny_unknown_fields and no env field, so figment silently ignores CAST_ENV__* during loading.
- **Decided:** Prefix is CAST_ENV__ (double underscore) per user example; output sorted by name for deterministic argv.
- **Decided:** Helper is pure over an iterator for testability; wrapper reads std::env and debug-logs keys only.

## [4f8ccff-dirty] Host Nix store sharing: investigation complete, substituter approach selected

Review of the first-draft report corrected two overstated claims and produced a third finding that reversed the recommendation on the local-overlay option. Report revised in place at .cue/master/doc/1785678626-4f8ccff/research-share-host-nix-store.md. Investigation task share-host-nix-store marked complete with evidence; follow-up implementation task nix-host-store-substituter created (open, blocked on the version-skew task). No code changed in this session.

- **Found:** Nix clients read the store directly from the filesystem; only mutations go via the daemon socket. This is why an overlay design cannot confine the OverlayFS mount to the agent-free daemon container - dev containers need the merged view in their own mount namespace, and mount namespaces are per-container
- **Found:** Consequence: local-overlay requires creating the mount host-side and bind-mounting the merged dir in, i.e. cast would need root on the host. The privilege escalates to the host rather than shrinking to the low-risk container
- **Found:** local-overlay is self-defeating against the stated motivation: OverlayFS requires the lower dir not change and nix requires the lower store to only grow, so it fixes double-build while forbidding free host GC - and double-GC was half the original complaint
- **Found:** Correction: the confidentiality risk of mounting host /nix was overstated. The mount lives only in the daemon container; agents run only in dev containers. Residual channel is an indirect path oracle - a client can ask the daemon to realise a specific path, but cannot enumerate (no listing, want-mass-query defaults false) and secret-bearing path hashes are not guessable
- **Found:** Correction: docker/podman on macOS CAN bind-mount host /nix (podman machine init -v, Docker Desktop file-sharing list). The real objection is solely the aarch64-darwin vs aarch64-linux system mismatch in derivation input hashes, plus virtiofs performance
- **Found:** Verified against nix 2.34.6 help-stores: local store accepts root, read-only and trusted params; trusted=true means 'usable as substitutes even if not signed by trusted-public-keys', which is what unlocks reuse of unsigned host-built paths
- **Found:** trusted-users is enforced by the daemon from the socket peer uid; a client cannot request reduced trust and the daemon cannot distinguish a containerised caller. cast has no in-container mitigation, only uid divergence via idmapped mounts
- **Decided:** Recommend and track only option D: host /nix bind-mounted read-only into the daemon container as a low-priority, trusted, read-only chroot-store substituter. Off by default, Linux-only, daemon container only
- **Decided:** Reject local-overlay-store (option C): needs host root, forbids free host GC, experimental. Withdrawn from the plan entirely rather than deferred
- **Decided:** Reject the reverse direction (host substituting from the cast store): inverts the sandbox trust boundary and is a host code execution path; a signing key for the cast daemon does not fix it since it attests provenance not trustworthiness
- **Decided:** Option B (host daemon socket) stays out of scope while configuration separation is a hard requirement, since substituters and trusted keys are daemon-side and would become the host's
- **Decided:** Disk duplication is explicitly out of scope; revisit only after measuring residual pain once redundant compilation is eliminated
- **Open:** Does the version-skew task need to land before the substituter work starts, or can they proceed in parallel with a graceful-degradation guard?
- **Open:** How should cast detect a VM-backed container engine in order to refuse the flag on macOS?
- **Open:** If disk duplication remains painful after this lands, what is the alternative - dedup pass, GC policy change, or accept it?

## [3a07c81] 0.2.0 scope decisions: herdr out to 0.3.0, local-json in, tracker refreshed

Coordination session with the operator (driven from the palekiwi workspace) settled release scoping decisions for cast 0.2.0.

- **Decided:** herdr-service-pivot deferred to 0.3.0; tagged tag: 0.3.0 on the card, objective 4 resolved
- **Decided:** cast-local-json-overrides confirmed in 0.2.0 scope (card already tagged); release-0.2.0 card and plan story tracker refreshed to current reality (env-var-passthrough merged via PR #64 and complete; design-repo-defined-shells complete with build child in-progress)
- **Decided:** cast 0.2.0 and cue 0.2.0 run in parallel; cue-agent is not gated on cast
- **Open:** Does improve-cast-docs (high, open) ride in 0.2.0? Filed in the release plan sequencing notes
- **Open:** Month-old PR #61 (feat/cast-agent-mvp) still needs a merge/park decision

