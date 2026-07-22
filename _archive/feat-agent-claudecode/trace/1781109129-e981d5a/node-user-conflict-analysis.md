# Trace: node:lts-trixie-slim User Conflict Analysis

**Date:** 2026-06-10
**Consulted:** Gemini Flash
**Context:** `Dockerfile.dev.claudecode` uses `node:lts-trixie-slim` as base image,
which ships a built-in `node` user at UID/GID 1000. This conflicts with cast's
contract of mirroring the host user (same username/UID/GID) into the container.

---

## Problem

`node:lts-trixie-slim` has a pre-existing `node` user at UID 1000. Our Dockerfile
user-creation block checks existence by username, not by UID:

```bash
if ! getent passwd ${USERNAME} >/dev/null 2>&1; then
    useradd -u ${UID} -g ${GID} --non-unique ...
fi
```

When host user is `alice` (UID 1000): `getent passwd alice` returns nothing →
`useradd -u 1000 alice` silently fails (`|| true`) because UID 1000 is already
`node` → `USER alice` has no entry → container runs as `node`.

---

## Options Evaluated

### Option A: Keep node:lts-trixie-slim, rename with usermod/groupmod

Replace user block with UID/GID-based lookup + `usermod -l` / `groupmod -n`.

**Critical edge cases (Flash):**
- Host UID=0 → tries `usermod -l alice root` → corrupts or fails catastrophically
- Host username=`node` but UID≠1000 → `getent passwd node` returns existing node
  user → elif condition fails → no user created for the actual host UID → bind
  mount ownership broken
- npm global install writes config files with hardcoded `/home/node` paths;
  moving homedir with `usermod -m` does not rewrite file contents → runtime errors
- Group name collision if a group named `${USERNAME}` already exists at a
  different GID → `groupmod` crashes

**Verdict:** Too many failure modes. Not safe.

### Option B: debian:trixie-slim + NodeSource curl-pipe-bash

```dockerfile
FROM debian:trixie-slim
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y nodejs
```

**Concerns (Flash):**
- Non-deterministic: `setup_lts.x` fetches whatever NodeSource calls "LTS" today
- Fails in offline/sandboxed/proxy environments (network required at build time)
- curl-pipe-bash runs as root; third-party script; supply-chain risk

**Verdict:** Reproducibility and security concerns outweigh the simplicity.

### Option C: debian:trixie-slim + COPY --from=node:lts-trixie-slim (CHOSEN)

```dockerfile
FROM debian:trixie-slim
COPY --from=node:lts-trixie-slim /usr/local /usr/local
RUN npm install -g @anthropic-ai/claude-code@${AGENT_VERSION}
```

**Why it works:**
- Both images are built on `debian:trixie-slim` → identical glibc/libstdc++ →
  100% ABI-compatible binary copy
- No user inherited from the node image → clean-slate, same user-creation block
  as opencode/pi
- Official Node.js binaries (same as node:lts-trixie-slim) without curl-pipe-bash
- npm global installs to `/usr/local/lib/node_modules`, symlinks to
  `/usr/local/bin/claude` → world-readable/executable → any runtime user works
- Docker layer cache handles the node image pull; subsequent builds can be offline

**Verdict:** Recommended. Cleanest solution.

---

## Decision

Implement Option C. Switch `FROM node:lts-trixie-slim` to `FROM debian:trixie-slim`
with `COPY --from=node:lts-trixie-slim /usr/local /usr/local`. User creation block
stays identical to opencode/pi Dockerfiles — no special handling needed.
