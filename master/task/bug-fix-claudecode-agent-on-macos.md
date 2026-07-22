---
title: "Bug: Fix ClaudeCode Agent on MacOS"
priority: critical
status: complete
---
A macOS user got the following error when running `cast run claudecode` in a project
with `use_flake: true` and a `flake.nix` present:

```
error:
       … while fetching the input 'git+file:///home/alex/Documents/ygt/sales'

       error: opening Git repository "/home/alex/Documents/ygt/sales": repository path
       '/home/alex/Documents/ygt/sales' is not owned by current user (libgit2 error code = 7)
```

## Root Cause

The error occurs **inside the dev container**, not on the macOS host. The path
`/home/alex/Documents/ygt/sales` is the container-side mapping of the macOS host path
`/Users/alex/Documents/ygt/sales` (produced by `map_container_path`).

When `use_flake: true` and `flake.nix` is present, the command run inside the container is:

```
nix develop . -c claude
```

`nix develop .` uses libgit2 to resolve the flake from the current git repository.
libgit2 applies the `safe.directory` ownership check (CVE-2022-24765, Git 2.35.2+),
which fires when the bind-mounted workspace directory appears owned by a different UID
than the container process. On macOS, Docker Desktop's virtualization layer can cause
this UID mismatch.

**This is not macOS-specific.** The same error occurs on Linux with Docker
`--userns-remap` enabled, non-standard UIDs, or a cached image built by a different user.

The same user did not hit this with `cast run opencode`. This is because the opencode
run likely did not have `use_flake: true` with a `flake.nix` present — not because
opencode is immune. The opencode Dockerfile has the same gap.

## Fix

`Dockerfile.nix-daemon` already solves this with:

```dockerfile
RUN git config --global safe.directory "*"
```

All three agent Dockerfiles (`claudecode`, `opencode`, `pi`) are missing this.
Additionally, `opencode` and `pi` Dockerfiles contain unnecessary git config settings
(`user.name`, `user.email`, `init.defaultBranch`, etc.) that `claudecode` never had.

The fix standardizes all three Dockerfiles to be minimal: only
`RUN git config --system safe.directory "*"` is needed.

See trace: `trace/1781804980-2ce36c4/safe-directory-bug-analysis.md`
