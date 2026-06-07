# Research Report: AI Agent Commit Signing in `cast`

## Overview
AI agents running inside `cast` operate within Docker-isolated environments and Nix devshells. Currently, these agents often commit code using the host user's identity (due to repository-local git configurations) but are unable to sign their commits. The goal is to provide agents with a separate, dedicated signing key while ensuring they do not have access to the user's primary private keys.

## Current State Analysis

### 1. Agent Environment and Identity
- **User Mapping**: Agents run with the same UID, GID, and username as the host user (`crates/cast/src/user/mod.rs`).
- **Container Isolation**: Agents run in Docker containers. Git is pre-installed and configured at the system level.
- **Default Identity**: 
  - `pi` agent: `pi <pi@local>`
  - `opencode` agent: `opencode <opencode@local>`
  - System-wide config: `commit.gpgsign false` (set in `crates/cast/assets/Dockerfile.dev.*`).

### 2. Git Configuration Resolution
- **Workspace Mounting**: The workspace is bind-mounted into the container as `rw` (`crates/cast/src/dev/run.rs:185`).
- **Identity Leakage**: Git resolves identity by checking:
  1. Repository-local `.git/config` (Inherited via workspace mount)
  2. Global `~/.gitconfig` (Not mounted by default)
  3. System `/etc/gitconfig` (Set in Docker image)
- **Problem**: If the user has configured their name/email locally in the repository, the agent uses that identity but lacks the corresponding signing key.

### 3. Commit Signing
- **Disabled by Default**: Both agent images explicitly disable GPG signing to prevent agents from hanging on passphrase prompts.
- **Key Access**: No GPG or SSH keys are mounted into the container by default.
- **Configuration**: The `Config` schema (`crates/cast/src/config/schema.rs`) lacks fields for git identity or signing keys.

## Key Findings

- **Passthrough Limitations**: The current environment variable passthrough (`PASSTHROUGH_VARS`) does not include any `GIT_*` or `GPG_*` variables.
- **Volume Extensibility**: While `extra_data_volumes` allows mounting host directories, mounting `~/.ssh` or `~/.gnupg` would expose the user's primary keys, which is explicitly forbidden by the requirements.
- **Nix Devshell Context**: Agents use `nix develop` to enter their environment. This environment is clean and does not inherit host git configurations unless they are part of the workspace.

## Proposed Strategy for Context Construction

To solve the problem, we need to address:
1. **Key Generation**: A mechanism to generate a "bot" signing key (GPG or SSH) that lives in the `cast` configuration directory (e.g., `~/.config/cast/keys/`).
2. **Environment Injection**:
   - Provide the public/private key pair to the container via a dedicated mount.
   - Inject `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `GIT_COMMITTER_NAME`, and `GIT_COMMITTER_EMAIL` into the agent's environment to override repository-local settings.
3. **Git Configuration**:
   - Set `user.signingkey` to the bot key.
   - Set `commit.gpgsign = true`.
   - Configure a non-interactive GPG/SSH signing agent if necessary.

## Sourced Findings

### Default Git Identity (OpenCode)
**File**: `crates/cast/assets/Dockerfile.dev.opencode`
```dockerfile
# Configure git defaults (system-level, immutable)
RUN git config --system user.name "opencode" && \
    git config --system user.email "opencode@local" && \
    git config --system init.defaultBranch "main" && \
    git config --system commit.gpgsign false && \
    git config --system core.autocrlf input && \
    git config --system pull.rebase false;
```

### Environment Passthrough
**File**: `crates/cast/src/dev/opencode/env.rs`
```rust
pub const PASSTHROUGH_VARS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    // ...
];
```

### Workspace Mount
**File**: `crates/cast/src/dev/run.rs`
```rust
run_args.extend([
    "-v".to_string(),
    format!(
        "{}:{}:rw",
        opts.workspace.root.display(),
        opts.workspace.container_path.display()
    ),
]);
```
