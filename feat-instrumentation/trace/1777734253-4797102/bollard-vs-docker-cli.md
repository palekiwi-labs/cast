# Architectural Decision: docker CLI vs bollard

## Context
While designing the instrumentation and logging system for `cast`, we evaluated replacing our `std::process::Command` calls to the `docker` CLI with the `bollard` Rust crate for direct Docker socket communication. The primary motivation was to obtain typed errors and richer execution metadata for instrumentation.

Consultations were held with architecture agents (Gemini and Sonnet) to analyze the tradeoffs regarding interactive TTYs, user environment compatibility, and maintenance burden.

## Findings & Analysis

### 1. Interactive TTY Sessions
`cast` heavily relies on passing interactive shell control to the user via `run` and `shell`. 
- **Current Approach**: `cast` uses `execvp` (or `spawn().wait()` with `Stdio::inherit()`). This allows the kernel and the Docker CLI to fully handle the pseudo-terminal, `SIGWINCH` (window resizing), and raw mode transparently.
- **Bollard Approach**: Would require building a custom TTY multiplexer in Rust, handling ANSI sequences, manual terminal resizing, and async byte pumping. This is notoriously fragile and complex.

### 2. User Environment Compatibility
Developer environments are complex and vary greatly (Docker Desktop, Colima, OrbStack, Linux natively).
- **Current Approach**: The `docker` CLI natively respects `~/.docker/config.json`, seamlessly executing credential helpers (e.g., `docker-credential-desktop`) to pull private images, and understands `docker context`.
- **Bollard Approach**: Does not execute credential helpers or respect Docker contexts. Pulling private images or connecting to non-standard local endpoints would break unless we manually re-implemented these protocols.

### 3. Dependencies and Maintenance
- **Current Approach**: The project is lean and synchronous. The `docker` CLI is a highly stable, officially maintained abstraction layer.
- **Bollard Approach**: Would require introducing `tokio` (and switching the architecture to async), `hyper`, and other heavy HTTP dependencies. This drastically inflates binary size, compile times, and maintenance overhead to track Docker API changes.

### 4. Instrumentation Goals
While `bollard` provides structured API errors (e.g., HTTP 404s), the existing architecture can achieve the required visibility through a "Hybrid/Instrumentation Approach":
- Retaining `Command` to shell out.
- Transitioning from `execvp` to `spawn().wait()` for interactive paths to capture exit codes.
- Using the `tracing` ecosystem to wrap execution calls, capturing the exact CLI arguments, duration, and output in structured JSON format.

## Decision
**REJECTED `bollard` integration.**

`cast` will continue to use the `docker` CLI via `std::process::Command`. The instrumentation needs will be met by implementing `spawn().wait()` for exit-code capture and using the `tracing` ecosystem for structured logging, maintaining a zero-async, high-compatibility architecture.