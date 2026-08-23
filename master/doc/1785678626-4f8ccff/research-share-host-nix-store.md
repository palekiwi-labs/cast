---
refs:
- /home/pl/code/palekiwi-labs/cast/.cue/master/task/share-host-nix-store.md
- /home/pl/code/palekiwi-labs/cast/.cue/master/task/nix-daemon-volume-version-skew.md
- /home/pl/code/palekiwi-labs/cast/.cue/master/task/harden-nix-security.md
- /home/pl/code/palekiwi-labs/cast/.cue/master/task/nix-host-store-substituter.md
---
# Sharing the host Nix store with cast containers

Investigation for task `share-host-nix-store`. Conclusions in section 8.

Revision note: sections 3C, 3D, 5 and 7 were revised after review. Two
claims in the first draft were overstated and are corrected in place, with
the correction marked. Do not cite the first draft.

## 1. Where cast is today

The current design is already a *shared-store, single-daemon* architecture.
It just happens that the store lives in a Docker volume rather than on the
host:

- `crates/cast/src/nix_daemon/daemon.rs:55` mounts `cast-nix:/nix:rw` into a
  dedicated `cast-nix-daemon` container built from
  `assets/Dockerfile.nix-daemon` (`FROM nixos/nix:2.34.6`).
- `crates/cast/src/dev/run.rs:437` mounts the same volume `cast-nix:/nix:ro`
  into every dev container.
- `assets/Dockerfile.dev:33` sets `NIX_REMOTE=daemon`, so dev containers
  reach the daemon through the socket that lives *inside* the shared volume
  at `/nix/var/nix/daemon-socket/socket`.
- Daemon nix.conf is injected out-of-band via `NIX_CONFIG`
  (`nix_daemon/config.rs`), and dev containers carry their own
  `/etc/nix/nix.conf` baked at `assets/Dockerfile.dev:23`.

This matters more than it first appears. The invariants cast already relies
on — one writer to the store, clients get the store read-only and mutate it
only through the daemon socket, client config is separate from daemon config
— are exactly the invariants any host-sharing design must preserve. Sharing
the host store is therefore not an architectural rewrite; it is a question of
*which store the daemon is pointed at and who owns the daemon*.

## 2. The two constraints that eliminate most designs

A Nix store is not a directory of files. It is a directory plus a SQLite
registry at `/nix/var/nix/db/db.sqlite` recording which paths are valid, their
NAR hashes, references, and signatures, plus lock files, `temproots`, and
`gcroots`. Two consequences:

- **Bind-mounting `/nix/store` alone is useless.** Without the matching db
  rows, the container's nix considers every host path invalid and will
  rebuild or re-substitute it anyway. You would have paid the mount cost for
  nothing.
- **A store must have exactly one writer.** Two nix daemons of possibly
  different versions writing one db and one `gc.lock` is a corruption and
  GC-race hazard, not a configuration choice.

A third constraint, less obvious and decisive for option C:

- **Clients read the store directly from the filesystem.** Only *mutations*
  go through the daemon socket. A nix client resolves store paths by reading
  `/nix/store` itself — which is why dev containers already mount the volume
  read-only rather than streaming bytes over the socket. Any design must
  therefore make the *complete* store view visible in the dev container's
  own mount namespace, not just in the daemon's.

Everything below follows from these three facts.

## 3. Candidate architectures

### Option A: bind-mount host `/nix` read-write, keep cast's daemon

Reject. This puts two daemons on one store. It also collides on the daemon
socket path: the container daemon binds
`/nix/var/nix/daemon-socket/socket`, unlinking and replacing the host's
socket and breaking nix on the host. Even setting the socket aside, a
container nix-daemon of a different version can migrate the shared SQLite
schema out from under the host's nix. This is the
`nix-daemon-volume-version-skew` hazard promoted from "latent" to
"guaranteed, and it takes the host down with it".

### Option B: bind-mount host `/nix` read-only, use the *host's* daemon

Mount host `/nix` at `/nix` read-only in both the dev containers and (if
retained at all) skip cast's daemon entirely. `NIX_REMOTE=daemon` then
resolves to the host daemon's socket, which is visible through the mount.

- Dedup: total. One store, one download, one build, one GC.
- Code delta: the smallest of any option. It is the current topology with the
  volume source swapped and `nix_daemon::ensure_running` short-circuited.
- Config separation: **partial, and this is the sticking point.** Nix config
  splits into client-side and daemon-side settings. Client-side settings
  (`experimental-features` for the CLI, flake registry, `NIX_PATH`, netrc,
  `~/.local/state/nix/profiles`) stay in the container and remain fully
  separated. Daemon-side settings — `substituters`, `trusted-public-keys`,
  `sandbox`, `cores`, `max-jobs`, `build-users-group` — become the host's,
  because they are enforced by whoever owns the daemon. cast's
  `nix_extra_substituters` config would silently stop being authoritative in
  this mode unless the container's uid is a host `trusted-user`, which is
  precisely what you must not do (see section 4).
- GC: host `nix-collect-garbage` will not see the container's roots. Live
  sessions are protected by the daemon's `temproots` for the duration of the
  connection, but a container profile or a `./result` symlink is registered as
  an indirect root pointing at a *container* path, which the host cannot
  resolve, so it is treated as dangling and ignored. Anything the container
  depends on but is not actively holding open is collectable.

Disqualified by the configuration-separation requirement.

### Option C: local-overlay store — host store as read-only lower layer

**Rejected after review. The first draft recommended prototyping this; that
recommendation is withdrawn.**

Nix has a first-class construct that appears to fit: the `local-overlay-store`
(experimental feature `local-overlay-store`, present in 2.34.6, verified
locally). It composes a read-only `lower-store` with a writable `upper-layer`
into one logical store via an OverlayFS mount, with validity determined by
the union of the two dbs and new builds landing in the upper layer. Nix never
modifies the lower store, so the single-writer constraint is respected.

Store URI shape, from the manual's worked example:

```
local-overlay://?root=/mnt/merged&lower-store=/mnt/store-a&upper-layer=/mnt/store-b
```

On paper this is the only option delivering both full dedup and full
configuration separation. Three findings sink it:

1. **The privilege escalates to the host rather than shrinking.** Nix does
   not create the OverlayFS mount; it only verifies it (`check-mount`). By
   constraint 3 in section 2, the *merged* view must exist in the dev
   container's mount namespace, not just the daemon's — otherwise every path
   inherited from the lower layer is invisible to the agent. Mount namespaces
   are per-container, so a mount made inside the daemon container does not
   propagate to siblings without `rshared` plumbing. The workable version is
   to create the overlay mount on the host and bind-mount the merged
   directory into both containers, which requires cast to hold **root on the
   host** to call `mount -t overlay`. cast currently requires no host
   privileges at all; this would change what cast is. Confining
   `CAP_SYS_ADMIN` to the agent-free daemon container — the obvious
   mitigation — does not work.
2. **It is self-defeating against the stated motivation.** OverlayFS requires
   the lower directory not to change while mounted, and Nix requires the
   lower store to only grow. The motivation for host sharing was to stop
   rebuilding *and garbage collecting* everything twice. This design solves
   the rebuild half while forbidding free GC of the host store for as long as
   any container store is composed against it. Host GC of a lower path that
   an upper path references leaves dangling references requiring
   `nix-store --verify --repair`.
3. **Experimental status.** Interface subject to change, on a core code path.
   Also requires the upper layer to support `trusted.overlay.*` xattrs, which
   makes overlay-on-overlay storage-driver configurations a live risk.

If disk duplication later proves to be the binding constraint, the answer is
more likely a dedup or GC-policy change than an overlay composition.

### Option D: host store as a substituter (chroot store URI) — RECOMMENDED

A local Nix store can be rooted anywhere: `root=R` places the store dir at
`R/nix/store` and the db at `R/nix/var/nix/db`. So bind-mount host `/nix`
read-only at `/host-nix/nix` in the **daemon container only**, and add to the
generated `NIX_CONFIG` in `nix_daemon/config.rs`:

```
extra-substituters = local?root=/host-nix&read-only=true&trusted=true&priority=10
extra-trusted-substituters = local?root=/host-nix&read-only=true&trusted=true&priority=10
```

Three parameters carry the weight (all verified against `nix help-stores` on
2.34.6):

- `read-only=true` — Nix opens the store db read-write even for queries and
  would otherwise fail on a read-only mount. This is what makes a ro-mounted
  store usable at all.
- `trusted=true` — "paths from this store can be used as substitutes even if
  they are not signed by a key listed in `trusted-public-keys`". Locally
  built host paths carry no signature, so **all compile-time savings live
  behind this flag**. Without it you get only paths that retained their
  `cache.nixos.org` signature, which are fetchable from the cache anyway.
- `priority=10` — lower priority than real binary caches (default 0), so the
  host store acts as the fallback that avoids compilation rather than
  displacing the cache.

Properties:

- Dedup: **rebuild and re-download dedup only, not disk dedup.**
  Substitution between two local stores is a NAR serialise/deserialise, so
  paths are physically copied into the cast store. Disk usage and double GC
  remain. This addresses the expensive half (compilation) and not the other.
- Config separation: complete. cast keeps its own daemon and its entire
  nix.conf including `nix_extra_substituters`. It is just another substituter
  entry.
- Security: no socket sharing, no capabilities, no host privileges, no host
  writes. The container cannot affect the host store at all.
- Code delta: one extra mount plus one config line.

**Confidentiality note (corrected).** The first draft claimed this exposes the
host store to agents. That was overstated: the host mount exists only in the
daemon container, and agents run only in dev containers, which never see it.
The residual channel is indirect — an untrusted client may ask the daemon to
realise a *specific* store path, which the daemon will substitute from the
host store and place in the cast store, where the dev container can read it.
There is no enumeration: no directory listing is available and
`want-mass-query` defaults to `false`, so the client needs the exact path
hash. For a path whose content is secret, that hash is derived from inputs
including the secret and is not guessable. The realistic residual is an
oracle for paths whose hashes an agent already knows. Low severity; document
it, do not design around it.

### Option D-reverse: host uses the cast store as a substituter — REJECT

Mechanically possible: the volume is readable on the host under
`/var/lib/docker/volumes/cast-nix/_data` (or the rootless podman equivalent),
so the host could add a `local?root=...` substituter pointing at it.

Reject. It inverts the trust boundary the sandbox exists to create. The cast
store's contents are produced under agent control. With `trusted=true`, any
path an agent placed there can be substituted into the host store *under a
store path the host would otherwise have built itself*, and the host will
then execute it. An agent able to `nix store add` or build an arbitrary
derivation can pre-position content at a path it predicts the host will want.
That is host code execution, gated only on eventual use of that package.

Without `trusted=true` it is safe and useless: the host would accept only
paths still signed by keys in its `trusted-public-keys`, i.e. things it could
fetch from `cache.nixos.org` directly. No compile savings, since agent-built
paths are unsigned.

Giving the cast daemon its own signing key and trusting that key host-side
does not rescue it. The signature would attest "the cast daemon built this",
not "this is trustworthy", and the cast daemon builds whatever the agent
asked for. Same exposure with more ceremony.

The asymmetry is inherent and correct: the sandbox is downstream of the host
in trust, and substitution should only flow downhill. Moving a specific
expensive artifact out of a container should be a deliberate, reviewed
`nix copy` of a named path, not a standing trust relationship.

### Option E: expose the host daemon socket only, keep a private store

Not viable in a useful form. If the container has a private store, paths
realised by the host daemon are not in it; if it shares the host store, this
degenerates to option B.

## 4. Security analysis

The exposure is entirely about **who the container is, as seen by the host
nix daemon**, and applies to options A and B (socket sharing) only. Options C
and D never contact the host daemon, so none of this section applies to the
recommended design.

The daemon socket is world-writable (`0666`, verified on this host). Nix
authorises a client by the **peer uid of the socket connection**, checked
against `trusted-users`. There is no protocol-level way for a client to
request reduced trust, and no daemon-side way to distinguish a containerised
caller from a native one. cast cannot override this from inside the
container. Two regimes:

**Untrusted client (the safe case).** Cannot add substituters or trusted
keys, cannot import unsigned NARs, cannot set `builders` or override sandbox
settings. It can still ask the daemon to *build* arbitrary derivations, which
run on the host inside the nix build sandbox as a `nixbld` user. The residual
risk is resource exhaustion, store growth, and whatever the host's sandbox
configuration permits (note `sandbox` is off by default on darwin and
`sandbox-paths` on this host already pokes a hole for `/bin/sh`).

**Trusted client (the dangerous case).** A trusted client can set arbitrary
`substituters` and `trusted-public-keys` for its own requests, and import
unsigned NARs directly, i.e. it can place attacker-chosen content into the
host store under any store path it likes. From there the host may execute it.
This is a full host compromise path.

The critical detail for cast: `assets/Dockerfile.dev:61` creates the
container user with `useradd -u ${UID}`, deliberately matching the host uid.
So a dev container process presents the *host user's own uid* to the host
daemon. On this machine `trusted-users = root` and uid 1000 is untrusted, so
the safe regime holds. But `trusted-users = @wheel` or an explicit username
is an extremely common host configuration (and is effectively standard on
multi-user darwin installs), and on such a host enabling socket sharing
silently hands every agent container trusted-user authority over the host
store. Agent containers run untrusted model-generated code; this is not a
theoretical concern.

Therefore, if option B is ever shipped:

- cast must read the host's effective `trusted-users` at startup and **refuse
  to enable the mode** if the container uid, its groups, or `*` appear there,
  unless the user passes an explicit override acknowledging it.
- The daemon container must never be rootful-with-uid-0 against the host
  socket, since root is trusted by definition.
- The alternative mitigation is uid divergence: run the container process as
  a host uid that is not trusted, and use an idmapped bind mount
  (`-v src:dst:idmap`, or `--uidmap`) so workspace ownership still resolves.
  That decouples the kernel-visible uid from the in-container uid, which
  `useradd -u ${UID}` currently welds together. Viable, but it is extra
  machinery in service of the weakest option.
- This interacts directly with `harden-nix-security`.

## 5. macOS

**Corrected.** The first draft claimed containers on macOS cannot bind-mount
the host `/nix`. That is wrong. Podman shares host paths at machine-init time
(`podman machine init -v /nix:/nix`) and Docker Desktop via its file-sharing
list — which defaults to `/Users`, `/Volumes`, `/private`, `/tmp`, so `/nix`
must be added manually, but it is permitted. The accurate statement is that
it requires explicit VM configuration and performs poorly over virtiofs for a
stat-heavy tree of hundreds of thousands of entries.

The decisive objection stands independently: **wrong system.** Store paths are
`/nix/store` on darwin too, but `system` is an input to the hash of every
non-fixed-output derivation, so an `aarch64-darwin` host store and an
`aarch64-linux` container store compute different paths for the same package.
Nothing collides, so nothing is reused.

Fixed-output derivations are the real exception. FOD hashes are
content-derived and system-independent, so under option D specifically,
source tarballs, `fetchurl` results, and the individual gem fetches
underneath `bundlerEnv` *would* hit. But every derived output — the gem
environment, native extensions, the interpreter — would not. That is a
network saving, not a compile saving, and compile time is the stated pain.

Note also that macOS users already get container-to-container dedup, since
all cast containers share the volume inside the Linux VM. The missing axis is
host-to-container, and that is precisely the axis the system mismatch
destroys.

Recommendation: gate the feature on Linux hosts and document the macOS
position explicitly so it is not re-litigated. The team split described is an
argument for keeping the volume-based path as the default and fully supported
mode indefinitely, not as a legacy fallback.

## 6. Interaction with version skew

Sharing amplifies `nix-daemon-volume-version-skew` rather than resolving it,
and changes its character per option:

- Option B removes the skew problem entirely by removing the second nix: the
  host's nix is the only nix. Instead it introduces a *coupling*: cast's dev
  container CLI must speak a worker protocol the host daemon understands.
  Nix's protocol is versioned and backward-compatible in practice, but cast
  would inherit a dependency on the host's nix version that it currently does
  not have, and would need a minimum-version check at enable time.
- Option C keeps two nix versions but gives each its own db, which is the
  correct shape. The lower store's db is read by the container's nix, so the
  container nix must be at least as new as the host's schema.
- Option D is the most tolerant: substitution is a NAR transfer, so version
  coupling is limited to reading the chroot store's db. Still non-zero — a
  host nix newer than the daemon image's nix could present a db schema the
  container cannot read — so the daemon should degrade gracefully (warn and
  disable the substituter) rather than fail the session.

The existing skew task should be resolved first. Adding a second
store-provenance dimension on top of an unresolved volume/image versioning
story will make both problems harder to reason about.

## 7. Assessment

The pain is twofold: redundant *work* (rebuilds, re-downloads) and redundant
*state* (disk, two GC domains). Compilation is the expensive half.

- Option A: reject. Two daemons on one db, plus a socket-path collision that
  breaks nix on the host.
- Option B: disqualified by the configuration-separation requirement, and its
  security posture depends on a host setting cast does not control. Could
  exist as an explicitly-labelled "trusted host" mode for solo use, with the
  mandatory `trusted-users` guard. Not a default.
- Option C: rejected. Requires host root, and forbids free host GC — which is
  half of what the exercise was meant to fix.
- Option D-reverse: reject. Inverts the sandbox's trust direction.
- **Option D: recommended.** Small diff, no privileges, no socket, complete
  configuration separation, and it eliminates duplicated compilation and
  downloads. It does not address disk duplication.

The original decision to isolate the store was not wrong: relocatability,
size constraints and predictability are real benefits, and they are lost
under B and C but fully retained under D. What changed is the cost side, and
D captures the expensive part of the benefit without surrendering any of
those properties. That asymmetry is the main finding of this investigation.

## 8. Conclusions

1. Resolve `nix-daemon-volume-version-skew` first. It is a prerequisite.
2. Implement option D as an off-by-default config flag. Tracked as
   `nix-host-store-substituter`.
3. Do not pursue options A, C, or D-reverse. Reasons recorded above.
4. Treat option B as out of scope unless the configuration-separation
   requirement is explicitly relaxed.
5. Gate to Linux hosts; keep the volume-based store as the permanent default.
6. Revisit disk duplication only after measuring how much pain remains once
   redundant compilation is gone.

Resolved during review: `CAP_SYS_ADMIN` in the daemon container is moot
because option C needs host root regardless; the host-GC dangling-reference
behaviour is a dealbreaker rather than a design input; cast must keep
enforcing `nix_extra_substituters`, which disqualifies option B.
