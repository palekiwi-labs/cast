# Proposed Documentation Tree

cast/
│
├── README.md                                ← Replace "TODO": pitch + install + link to docs/
│
├── docs/                                    ← Minimal project-level entry point
│   ├── README.md                            ← Pure navigation: what each crate is, links to crate docs
│   └── quick-start.md                       ← Install → first `cast run opencode` → 5-min path
│
├── crates/
│   ├── cast/
│   │   └── docs/                            ← Full documentation suite
│   │       ├── README.md                    ← TOC for the cast crate
│   │       ├── getting-started.md           ← Prerequisites, install, first run
│   │       ├── concepts.md                  ← Mental model: sandbox, Docker, agents, Nix
│   │       │
│   │       ├── commands/
│   │       │   └── reference.md             ← All subcommands, flags, env vars, exit codes
│   │       │
│   │       ├── agents.md                    ← Agent trait contract, 6-step lifecycle, port calc
│   │       │
│   │       ├── config/
│   │       │   ├── overview.md              ← File locations, loading precedence
│   │       │   ├── reference.md             ← Every Config field with defaults
│   │       │   ├── approval.md              ← Hash model, allow/deny/diff workflow
│   │       │   └── env-overrides.md         ← CAST_* env var → field mapping
│   │       │
│   │       ├── nix/
│   │       │   ├── overview.md              ← Two modes: flake wrapping vs daemon volume
│   │       │   ├── daemon.md                ← Container lifecycle, shared /nix, Unix socket
│   │       │   └── flake-integration.md     ← use_flake, nix develop wrapping, include an example flake
│   │       │
│   │       └── mcp/
│   │           ├── overview.md              ← MCP server: Axum, tool dispatch, embedded docs
│   │           ├── client.md
│   │           └── configuration.md         ← ✅ EXISTS — keep path unchanged
│   │       
│   │
│   └── cast-mcp-client/
│       └── docs/                            ← Lighter-touch (supporting tool)
│           ├── README.md                    ← What it is, when to use it vs cast directly
│           ├── usage.md                     ← All 5 subcommands with examples
│           ├── config.md                    ← ClientConfig, {env:VAR}, CAST_MCP_URL override
│           └── script-generation.md         ← generate command, Bash wrapper anatomy
