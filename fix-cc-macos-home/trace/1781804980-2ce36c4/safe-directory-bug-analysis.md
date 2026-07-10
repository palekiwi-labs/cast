# Trace: ClaudeCode macOS Bug — safe.directory Analysis

## Bug Report

User ran `cast run claudecode` on macOS and got:

```
error:
       … while fetching the input 'git+file:///home/alex/Documents/ygt/sales'

       error: opening Git repository "/home/alex/Documents/ygt/sales": repository path
       '/home/alex/Documents/ygt/sales' is not owned by current user (libgit2 error code = 7)
```

## Key Facts Established

### The path is container-side, not macOS host-side

On macOS the host path `/Users/alex/Documents/ygt/sales` is mapped via `map_container_path`:

```rust
fn map_container_path(root: &Path, home_dir: Option<&Path>, username: &str) -> PathBuf {
    if let Some(home) = home_dir
        && let Ok(rel) = root.strip_prefix(home)
    {
        PathBuf::from("/home").join(username).join(rel)
    } else { ... }
}
```

`/Users/alex` → strip prefix → `Documents/ygt/sales` → `/home/alex/Documents/ygt/sales`.
This exactly matches the error. The error happens **inside the dev container**.

### The nix-daemon is not involved

The nix-daemon is started with only `-v {nix_volume}:/nix:rw` — no workspace mount.
The error cannot originate there.

### The trigger: `nix develop .` uses libgit2

When `use_flake: true` and `flake.nix` is present, `build_command` wraps the agent command:

```
nix develop . -c claude
```

`nix develop .` uses libgit2 internally to resolve the flake from the current git repo.
libgit2 applies the `safe.directory` ownership check (CVE-2022-24765, Git 2.35.2+).

### UID flow

- Image is built with host UID/GID baked in via `--build-arg UID=... GID=...`
- No `--user` flag at runtime — container runs as the `USER` from the Dockerfile
- On Linux (standard setup): bind-mount UIDs pass through directly → owner matches → no error
- On macOS/Docker Desktop: virtualization layer can cause the mounted directory to appear
  owned by a different UID than the container process → libgit2 error code 7

### The fix already exists in the codebase

`Dockerfile.nix-daemon` already has:

```dockerfile
# Allow building git repositories owned by different users
# This is necessary because the daemon builds repos on behalf of multiple dev containers
RUN git config --global safe.directory "*"
```

**`Dockerfile.dev.claudecode` has no git config at all — this is the bug.**

### Config scope: --system vs --global

- nix-daemon uses `--global` (root's `~/.gitconfig`) — runs as root
- opencode/pi Dockerfiles use `--system` (`/etc/gitconfig`) for other git settings
- dev containers run as `${USERNAME}`, not root
- `--system` writes to `/etc/gitconfig`, readable by all users regardless of home dir
- `--system` is the correct scope for the dev containers (consistent with existing git config)

### libgit2 reads /etc/gitconfig

libgit2 reads `/etc/gitconfig` (system scope) unless `GIT_CONFIG_NOSYSTEM` is set.
cast sets no such environment variable. The fix is sound.

## Why opencode didn't reproduce for this user

**NOT because of image caching.** Both images bake the same host UID.

The correct explanation: `nix develop .` is only invoked when `use_flake: true` AND
`flake.nix` is present in the workspace. If the opencode run used a different workspace
or had `use_flake` disabled, libgit2 was never invoked. The opencode Dockerfile also
lacks `safe.directory "*"` and is equally vulnerable when those conditions are met.

## Is this macOS-specific?

**No.** The same bug can trigger on Linux with:
- Docker `--userns-remap` enabled (shifts all UIDs)
- A workspace owned by a non-matching UID
- A cached image built by a different user

It surfaces on macOS first because Docker Desktop's virtualization layer introduces
UID presentation differences more readily than native Linux Docker.

## Resolution

Add `RUN git config --system safe.directory "*"` to all three agent Dockerfiles.
Remove all other git config from opencode and pi Dockerfiles — those settings
(`user.name`, `user.email`, `init.defaultBranch`, etc.) are not needed and were
never in the claudecode Dockerfile. Standardize all three to be minimal.

## Opus Consultation

Consulted Claude Opus to verify the diagnosis. Key takeaways:
- Factual substrate confirmed correct (workspace mapping, UID baking, no --user flag, etc.)
- Challenged the claim that "VirtioFS causes foreign UID" — the exact macOS mechanism
  is not precisely established, but the fix is correct regardless
- Confirmed: opencode non-reproduction is about `use_flake` code path, not image caching
- Confirmed: `safe.directory "*"` is safe for single-tenant ephemeral containers
- Confirmed: not macOS-specific
- Flagged: use `--system` (consistent with other dev container git config, not `--global`)
- Flagged: verify libgit2 reads /etc/gitconfig — confirmed it does unless GIT_CONFIG_NOSYSTEM
