---
status: open
refs: master/task/herdr-service-pivot.md
---
# Worktree-compatible service UX

## Conclusion

Two viable service boundaries exist. The earlier recommendation of one service
per repository is no longer clearly preferable: **one service per Git
worktree** preserves the existing branch-local configuration and approval
model, while still making Git work when the service mounts the common Git
directory. It is likely the better default when concurrent worktrees are used
as independent streams of work.

In either model, the process cwd must remain the invoking worktree/directory.
Resolving every invocation to the primary checkout as its cwd would make
worktree use incorrect.

In the common layout where linked worktrees live at
`<primary>/worktrees/<branch>`, mounting the primary checkout exposes its
`.git` common directory and every worktree. This is convenient, but it grants
each worktree service access to sibling checkouts. A narrower and preferable
topology mounts only the selected worktree plus the primary checkout's `.git`
common directory.

## Repository-service model

Resolve these values independently at startup:

- `invocation_dir`: canonical host cwd; determines container workdir and where
  `cast exec` runs.
- `worktree_root`: `git rev-parse --show-toplevel`; the checkout containing
  `invocation_dir`.
- `git_common_dir`: absolute result of
  `git rev-parse --path-format=absolute --git-common-dir`.
- `repository_root`: the primary checkout when the common Git directory is
  its `.git` directory. This is the stable seed for the service name and any
  service-scoped port.

`cast up` uses `repository_root` for service identity. `cast exec` repeats
resolution, finds that service, and invokes Docker exec with the container
workdir mapped from `invocation_dir`. Thus `cast exec --mux opencode` in
`feature-a` opens a pane in `feature-a`, without starting a second container.

Do not use a branch name, the immediate cwd, or the selected harness in this
model's service identity. They all change independently of the repository
service.

This model gives one mux and one persistent service state for every checkout
family, but requires repository-scoped service configuration and approval.

## Worktree-service model

Use the resolved `worktree_root` as the service identity and configuration
root. `cast up` in the primary checkout creates the primary service; `cast up`
in `feature-a` creates a second service for feature-a. `cast exec` targets the
service for the invoking worktree. It should retain Docker-like semantics and
fail clearly when that service is absent rather than silently create it; the
user explicitly chooses creation with `cast up`.

This changes the proposed norm from **one container per repository** to **one
container per checkout**, but gives a direct and desirable mapping:

- each branch reads its own `cast.json`, `cast.local.json`, and project flake;
- each configuration is approved against its own checkout as it is today;
- resource settings, mounts, server command, and service lifecycle cannot
  conflict across worktrees;
- each mux has its own sessions and agent process tree, so a bad session has a
  smaller blast radius.

The trade-off is deliberate duplication: feature-a and feature-b each run a
mux/service container and each needs its own long-lived state. They may still
share the Nix daemon and Nix store, but must not share mux state.

## Mount contract

The container needs path-complete Git metadata, not merely the worktree's
`.git` pointer file.

1. Bind mount the invoked worktree read-write.
2. Bind mount `git_common_dir` read-write at a compatible container path.
   Git needs this directory for refs, objects, and linked-worktree metadata.
   The primary checkout's files are not otherwise required.
3. Ensure `git_common_dir` remains reachable at the exact absolute path
   recorded in a linked worktree's `.git` file. The robust way is to expose
   source paths at their host absolute container paths, or otherwise add an
   exact-path bind mount of the common Git directory. Rewriting the `.git`
   files is not acceptable because it modifies user working trees.
4. Map `invocation_dir` to the corresponding mounted path and pass it as
   Docker's `--workdir` for every exec.

The current home-relative mapping happens to preserve paths under `/home`, but
it changes paths outside the home directory to `/workspace/...`. That breaks
linked-worktree `.git` files that contain host-absolute `gitdir:` locations.
The implementation must test both mappings, including an external worktree.

Mounting the entire primary checkout is a simpler alternate implementation and
works immediately for the stated `./worktrees/` layout, but unnecessarily
exposes sibling branch files to a feature-specific agent. It is not needed in
the worktree-service model.

The narrow topology removes the repository-service model's external-worktree
mount problem: every service has a fixed selected-worktree mount from startup.

## Case behavior

### Case 1

1. From the primary checkout, `cast up` creates the primary-checkout service
   and mounts that checkout plus its common Git directory.
2. `cast exec --mux opencode` runs in the primary checkout.
3. From `worktrees/feature-a`, `cast up` creates the independent feature-a
   service, then `cast exec --mux opencode` runs in feature-a.

Root and feature-a have separate service identities and, if host ports are
ever published, separate port allocation. Their worktree configurations do not
conflict.

### Case 2

From `worktrees/feature-a`, `cast up` creates the feature-a service, mounting
that worktree and its common Git directory. A later `cast up` in root or
feature-b creates a distinct service for that checkout. `cast exec` always
targets only the service for the current worktree.

## Integration points and migration impact

Current `ResolvedWorkspace` is only `{ root, container_path }` and uses the
literal cwd. It is used for config approval, config-relative files, mounts,
shadow mounts, container names, ports, and workdir. The redesign should not
silently repurpose this one field: introduce explicit service-repository and
invocation-worktree values and migrate consumers intentionally.

The configuration loader currently loads `cast.json`, `cast.local.json`, and
`cast-mcp.json` only from cwd. The design must choose one policy:

- repository-level service configuration, loaded from `repository_root`, is
  coherent with one service and one approval; or
- branch-local configuration is permitted, but commands must reject settings
  that conflict with the already-running service.

The worktree-service model retains current configuration and approval scope;
this is its strongest advantage. It needs only to resolve configuration from
the enclosing `worktree_root` rather than arbitrary subdirectories. The
repository-service model instead requires repository-level configuration and
approval, or explicit conflict rejection for a branch whose configuration
differs from the running service.

Per-worktree services need separate multiplexer state. Existing shared
`~/.local` volumes cannot blindly hold herdr state: two services sharing its
socket/state path can collide or restore each other's sessions. Give each
service a path/volume keyed by the worktree identity (using herdr's XDG state,
config, and data homes), while preserving intentionally shared caches.

A long-lived service must have a reaping init (`docker --init` or equivalent),
as documented by the existing zombie-resilience trace. This is independent of
worktree support but required before multiplexed long-lived agents are safe.

## Non-Git and unusual repositories

Cast should reject non-Git directories with a clear error. Fail explicitly,
rather than guess, when Git reports a bare repository or a common directory
that cannot be mapped to a checkout. Submodules are independent Git
repositories and should receive independent services unless a later explicit
workspace-group feature says otherwise.

## Alternatives considered

- **One container per worktree:** recommended alternative. It deliberately
  changes the stated one-project-service goal, but eliminates service-config
  conflicts and permits genuinely branch-specific environments. It duplicates
  mux state and requires explicitly isolated state volumes.
- **Mount only the invoked worktree:** fails because linked-worktree Git needs
  the primary `.git` common directory.
- **Mount the primary checkout but make it every command's cwd:** Git works,
  but commands from feature branches modify/run the wrong checkout.
- **A config `project_root`:** useful only as a future override for unusual
  layouts; it should not be required for normal Git worktrees.

## Decisions needed before implementation

1. Choose repository service versus worktree service as the default identity
   boundary. The worktree alternative is the recommended response to
   branch-specific configuration.
2. Confirm whether worktree services use the narrow mount topology (worktree
   plus common Git directory; recommended) or mount the whole primary tree.
3. Define service-state isolation from currently shared cache/local volumes.
4. Confirm whether service identity should be a digest of canonical worktree
   path (worktree model) or primary path (repository model), independent of
   optional ports.
