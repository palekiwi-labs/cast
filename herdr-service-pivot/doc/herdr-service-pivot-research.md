# Research Report: herdr-service-pivot

Analysis of current path discovery, Docker mount computation, and naming identity in `cast` to support a project-wide service-pivot UX.

## 1. Current State Analysis

### Path Discovery
- **File**: `crates/cast/src/dev/workspace.rs:23` (`get_workspace`)
- **Mechanism**: Uses `std::env::current_dir()` as the `workspace.root`.
- **Behavior**: Fragmented. Every subdirectory or Git worktree is treated as a distinct project.

### Docker Mount Computation
- **File**: `crates/cast/src/dev/run.rs:400`
- **Mechanism**: Bind-mounts the current `workspace.root` to its host path (or `/workspace/abs/path`) in the container.
- **Behavior**: Incomplete for worktrees. The main project `.git` common directory is NOT mounted when invoked from a linked worktree, breaking Git operations.

### Naming, Identity, and Ports
- **Files**: `crates/cast/src/dev/container_name.rs`, `crates/cast/src/dev/port.rs`.
- **Mechanism**:
  - **Name**: `cast-{agent}-{basename}-{port}`
  - **Port**: 32768 + CRC32(pwd_str + agent_name) % 32768.
- **Behavior**: Unstable. Moving between project root, subdirectories, or worktrees changes both the container name and the port.

## 2. Service-Pivot Feasibility

### Pivot Discovery
Using `git` CLI to discover the main project root is highly feasible:
- `git rev-parse --show-toplevel`: Current worktree root.
- `git rev-parse --git-common-dir`: Main `.git` directory path.
- **Project Root**: `dirname(git-common-dir)`.

### Mount Strategy
To ensure a single project container with correct Git access:
1. **Primary Mount**: Bind-mount the "main" project root.
2. **Worktree Mount**: If the current worktree is outside the main root, mount it as a second volume.
3. **CWD Pivot**: Set the container's `--workdir` to the mapped path of the host's `current_dir()`.

### Identity Stabilization
- Use the **Main Project Root** path as the seed for port hashing and container naming.
- This ensures `cast up` or `cast exec` anywhere in the project tree (root, worktree, or deep subdir) targets the same container and uses the same port.

## 3. Risks and Requirements

- **Zombie Processes**: Long-lived project containers are vulnerable to PID exhaustion if agents (e.g., `opencode`) leak children.
  - **Requirement**: Add `--init` (tini) to all `docker run` calls (R1/R2 in task trace).
- **Git Dependency**: PIVOT logic depends on `git` availability. Needs a graceful fallback to `current_dir()` for non-Git projects.
- **Orphaned Containers**: The name change will cause existing containers to be "lost" to `cast` commands (they will still exist in Docker but won't match the new naming scheme).

## 4. Proposed Architecture Options

### Option A: Fully Git-Integrated (Recommended)
- Modify `get_workspace` to perform Git discovery.
- Stabilize all identity (Name/Port) on the Git main root.
- Mount the main root and the current worktree.

### Option B: Config-Driven
- Add a `project_root` field to `cast.json`.
- Stable identity only for projects with an explicit config.
- Higher manual overhead for users.

## 5. Conclusion
The service-pivot UX is technically feasible and addresses current fragmentation. The primary effort is in the `get_workspace` refactor and ensuring Docker mounts cover the Git common directory.
