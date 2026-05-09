# OCX Specification: Nix-Native Development Environments

## 1. Project Overview
OCX is a secure, high-performance CLI tool that manages isolated development environments using Docker and Nix. It orchestrates a persistent Nix daemon and ephemeral development containers to provide a seamless "Nix-inside-Docker" experience.

### 1.1 Goals
- **Isolation**: Every project gets a dedicated environment based on its directory path.
- **Security**: Strict capability dropping, resource limits, and path-masking (Shadow Mounts).
- **Persistence**: Shared Nix store across projects via a dedicated daemon container.
- **Reproducibility**: Environment defined by `flake.nix`.

---

## 2. System Architecture

OCX operates using a **Dual-Container Model**:

1.  **Nix-Daemon Container (`ocx-nix-daemon`)**:
    *   **Purpose**: A long-running background container that hosts the `/nix` store and the `nix-daemon` service.
    *   **Persistence**: Uses a Docker volume (typically named `ocx-nix-store`) to persist the store across restarts.
    *   **Communication**: Exposes the Nix daemon via a Unix socket or network interface shared with client containers.

2.  **Dev Container (Ephemeral)**:
    *   **Purpose**: The actual environment where the developer works.
    *   **Mounts**:
        *   The `/nix` store from the `ocx-nix-daemon`.
        *   The current project workspace from the host.
    *   **Lifecycle**: Created on-demand (`ocx opencode`) and removed after use (or persisted for specific tasks).
    *   **Entrypoint**: Typically runs `nix develop --command $SHELL`.

---

## 3. Configuration Specification

OCX uses a hierarchical configuration system.

### 3.1 Sources (In order of precedence)
1.  **Environment Variables**: Prefixed with `OCX_` (e.g., `OCX_MEMORY_LIMIT=4g`).
2.  **Project Config**: `./ocx.json` in the current working directory.
3.  **Global Config**: `~/.config/ocx/ocx.json`.
4.  **Defaults**: Hardcoded in the Rust binary.

### 3.2 Schema (Key Fields)
- `image`: The base Docker image to use for the dev container.
- `memory`: Memory limit (e.g., `2g`).
- `cpus`: CPU limit (e.g., `0.5`).
- `pids_limit`: Maximum number of processes.
- `ports`: Array of port mappings or deterministic auto-mapping toggle.
- `env`: Map of environment variables to inject.
- `shadow_mounts`: List of host paths to mask/block.
- `nix`:
    - `daemon_image`: Image for the nix-daemon.
    - `extra_substituters`: List of Nix binary caches.
    - `extra_trusted_public_keys`: Cache public keys.

---

## 4. CLI Specification

The CLI should be implemented using `clap` with the following primary commands:

### 4.1 `ocx opencode` (Alias: `o`)
Starts an interactive development session.
- **Logic**:
    1. Ensure `ocx-nix-daemon` is running.
    2. Build the `docker run` command for the dev container.
    3. Inherit STDIN/STDOUT for TTY.
    4. Execute `nix develop` inside the container.

### 4.2 `ocx run [command]`
Executes a specific command in the project environment headlessly.

### 4.3 `ocx nix`
Subcommands for managing the Nix infrastructure:
- `start`: Manually start the daemon container.
- `stop`: Stop the daemon container.
- `prune`: Clean up unused Nix store paths (executes `nix-store --gc` in the daemon).

---

## 5. Docker Orchestration Logic

### 5.1 Security Hardening (The "OCX Shield")
The generated `docker run` command **must** include:
- `--cap-drop ALL`: Remove all Linux capabilities.
- `--security-opt no-new-privileges`: Prevent privilege escalation.
- `--user [uid]:[gid]`: Run as the host user's UID/GID.
- **Shadow Mounts**:
    - For sensitive directories (e.g., `~/.ssh`): `--tmpfs [path]`.
    - For sensitive files: `-v /dev/null:[path]`.

### 5.2 Path Mapping (Workspace)
- The host path `$CWD` is mapped to a predictable path inside the container.
- If the path is under the user's `$HOME`, the structure is mirrored. Otherwise, it is mounted to `/workspace`.

### 5.3 Deterministic Port Mapping
- If enabled, OCX calculates a hash of the project directory.
- Maps this hash to a port range (e.g., `32768-65535`).
- Allows multiple projects to run their own web servers without port conflicts on the host.

---

## 6. Technical Implementation (Rust)

### 6.1 Recommended Crate Stack
- `clap`: CLI argument parsing (v4+).
- `figment`: Hierarchical configuration loading.
- `serde` / `serde_json`: Serialization.
- `std::process::Command`: Docker CLI interaction.
- `thiserror`: Internal module error types.
- `anyhow`: Top-level error context and reporting.
- `tracing`: Structured logging and diagnostic output.
- `whoami`: For retrieving user and host information.

### 6.2 Module Structure
```text
src/
├── main.rs          # CLI entrypoint and error formatting
├── config/          # Hierarchical config logic
├── docker/          # Type-safe Docker Command builders
│   ├── mod.rs
│   ├── daemon.rs    # Nix-daemon lifecycle
│   └── dev_env.rs   # Dev container orchestration
├── security/        # Shadow mounts and cap-drop logic
├── workspace/       # Path mapping and directory hashing
└── utils/           # Port generation, shell escaping
```

---

## 7. Execution Flow (The "Happy Path")

1.  **Initialization**:
    - Load config from files and environment.
    - Resolve the project directory and its hash.
2.  **Daemon Check**:
    - Run `docker ps` to find `ocx-nix-daemon`.
    - If missing: `docker run` the daemon image with the `ocx-nix-store` volume.
3.  **Command Assembly**:
    - Gather volumes (Workspace, Nix store, Shadow mounts).
    - Gather environment variables (Nix daemon address, User env).
    - Set resource limits and security opts.
4.  **Execution**:
    - Use `Command::new("docker").args(...).spawn()`.
    - Wait for the process to exit and return the status code.
