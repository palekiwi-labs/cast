---
title: Design repository-defined agent and project shells
status: complete
priority: high
tag: 0.2.0
kind: design
---

## Problem Statement

`cast` uses a nested devshell design: a **global shell** (typically defines the
agent harness) and a **project shell** (defines the project). Today the global
shell must live in the user's global flake and is selected by name via the
`global_shell` config field (defaulting to the agent name).

Organizations using `cast` want the option to define **both** shells inside
their repository so the setup is trivially shared with all team members: a
"default" shell for the project, and a separate "ai" shell for the agent
(e.g. including `llm-agents`, `gh`, etc.). There is currently no way to source
the global (agent) shell from the repository.

## Candidate Direction (to be examined, not settled)

The simplest idea so far: let `global_shell` in the config accept either:

- a shell **name** resolved against the global flake (current behavior), or
- a **full flake reference** (path/URI with optional `#shell` fragment) that
  can be passed directly to `nix develop`, e.g. `.#ai` to pick an "ai"
  devShell from the repository flake.

The design session must validate this direction against alternatives,
backward compatibility, and the existing shell-selection code path
(`crates/cast/src/dev/build_command.rs`, `crates/cast/src/config/schema.rs`).

## Scope

- How the agent (global) shell can be sourced from the repository flake.
- How the project shell reference is expressed and shared via the repo.
- Config schema, validation, and error behavior for the new forms.
- Documentation impact (`docs/nix/`, `docs/config/reference.md`).
- Backward compatibility with existing `global_shell` (name-only) configs.

Out of scope (unless the session redirects): implementation, downstream
build task cards.
