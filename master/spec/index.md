# Project Overview: cast (coding agent sandbox tool)

`cast` is a Rust-based CLI utility designed to provide a secure environment for running coding agents.

## Core Functionality

- **Sandbox Orchestration**: Builds and manages Docker containers
- **Nix Integration**: Manages development environments, tools and dependencies via Nix
- **Workspace Isolation**: Preventing uncontrolled access to the host system.
- **Agent Abstraction**: Provides a generic `Agent` trait
