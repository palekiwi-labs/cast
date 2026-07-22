# Plan: Prepare repo for 0.1.0 release

## Philosophy
- **Nix-only** for install/usage docs at this stage
- **Guide towards source code** — don't duplicate it
- **Project-level docs are minimal** — link to crate docs
- **Don't over-document** — get something in place, not something exhaustive

---

## Documentation Structure

See `.mem/feat-pre-release/spec/proposed-doc-tree.md` for the agreed file tree.

---

## Files to Create

### Repo root

| File | Content guidance |
|------|-----------------|
| `README.md` | What cast is (1 paragraph). Nix install: `nix run github:palekiwi-labs/cast` and `nix profile install`. Link to `docs/`. |

### `docs/`

| File | Content guidance |
|------|-----------------|
| `docs/README.md` | Navigation only. Two crates, one-liner each, links to `crates/cast/docs/README.md` and `crates/cast-mcp-client/docs/README.md`. |
| `docs/quick-start.md` | Nix-only: `nix profile install` then `cast run opencode`. Prerequisites (Docker). Link to `crates/cast/docs/` for depth. |

### `crates/cast/docs/`

| File | Content guidance |
|------|-----------------|
| `README.md` | TOC — section list with one-liner per entry, all links. |
| `getting-started.md` | Prerequisites (Docker, Nix). Install via `nix profile install`. First run: `cast config allow` → `cast run opencode`. Link to config/ and agents.md. |
| `concepts.md` | Mental model: what a sandbox is, why Docker, how agents map to images, where Nix fits. Pointers to deeper sections, not exhaustive. |
| `agents.md` | Name the 3 supported agents (OpenCode, ClaudeCode, Pi). Explain `Agent` trait makes it extensible. Show trait method signatures (no impl detail). Point to `src/dev/agent.rs` and harness files in source. |
| `commands/reference.md` | List all subcommands with one-line descriptions and key flags. Point to source (`src/commands/`) for full detail. |
| `config/overview.md` | Where files live, loading precedence table (`env > cast-mcp.json > cast.json > global`). Link to reference.md. |
| `config/reference.md` | All `Config` fields with types and defaults. Point to `src/config/schema.rs`. |
| `config/approval.md` | The approval model: what gets hashed, where store lives, `allow/deny/diff` workflow. Point to `src/config/approval.rs`. |
| `config/env-overrides.md` | Table of `CAST_*` env vars → config fields. |
| `nix/overview.md` | Two modes: flake dev-shell wrapping and Nix daemon volume. When to use each. Links to daemon.md and flake-integration.md. |
| `nix/daemon.md` | Daemon container lifecycle, shared `/nix` volume, Unix socket handoff. `cast nix-daemon` commands. Point to `src/nix_daemon/`. |
| `nix/flake-integration.md` | `use_flake` / `use_flake_path` fields, how `nix develop` wrapping works. Include a minimal example `flake.nix` snippet showing a typical project setup. |
| `mcp/overview.md` | What the built-in MCP server is, how tools are defined and dispatched, how embedded `docs/` are served to agents. Link to configuration.md and client.md. |
| `mcp/client.md` | For agents running inside cast containers. How to use `cast-mcp-client` from within a container: `list`, `describe`, `call`. `CAST_MCP_URL` env var. Link to `crates/cast-mcp-client/docs/` for full reference. |
| `mcp/configuration.md` | **EXISTS** — keep path unchanged. Update if needed. |

### `crates/cast-mcp-client/docs/`

| File | Content guidance |
|------|-----------------|
| `README.md` | What it is, relationship to `cast mcp` server. Link to usage.md. |
| `usage.md` | All 5 subcommands (`list`, `describe`, `call`, `status`, `generate`) with brief examples. |
| `config.md` | `cast-mcp-client.json` schema, `{env:VAR}` substitution, `CAST_MCP_URL` override. Point to `src/config/schema.rs`. |
| `script-generation.md` | `generate` command, what a generated Bash wrapper looks like, `jq` dependency. |

---

## Release Boilerplate

| File | Notes |
|------|-------|
| `LICENSE` | Use MIT. No GPL deps found; MIT consistent with full dep tree. Add `license = "MIT"` to both `Cargo.toml` files. |
| `CHANGELOG.md` | Minimal for 0.1.0 — single entry marking the initial release. Use Keep a Changelog format. |
| `Taskfile.yml` | One task: `prepare-release`. Steps: assert on `master` branch, tag commit with version, push commit + tag. Use `go-task` conventions. |

---

## Research Artifacts (already saved)

All research is in `.mem/feat-pre-release/ref/1781190455-4ea3fb2/`:
- `cast-crate-architecture.md`
- `cast-mcp-client-architecture.md`
- `testing-patterns.md`
- `config-system.md`
- `nix-integration.md`
- `existing-docs-and-license.md`
- `agent-trait-harnesses.md`

---

## Out of Scope for 0.1.0
- `development/` section (architecture, adding-an-agent, testing) — deferred
- Per-agent docs (opencode.md, claudecode.md, pi.md) — covered by agents.md
- Non-Nix install paths
