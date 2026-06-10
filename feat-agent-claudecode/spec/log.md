# Project Log

## [73dff8d] Research: Agent trait + ClaudeCode harness design

Researched the full Agent extension model in cast and analysed installation options for ClaudeCode (npm vs Debian). See findings below for the complete design recommendation.

- **Found:** Agent trait lives in crates/cast/src/dev/agent.rs (82 lines). Five required methods: name, dockerfile, resolve_version, extra_run_args, base_command. Two defaulted: image_tag (delegates to image::image_tag), ensure_image (delegates to image::ensure_image), build_command (delegates to build_command::build_command), prepare_host (no-op).
- **Found:** Both existing agents (OpenCode, Pi) resolve versions via GithubReleaseFetcher hitting the GitHub releases API. ClaudeCode has no GitHub repo so a new NpmFetcher would be needed.
- **Found:** Dockerfiles receive a single AGENT_VERSION build-arg and download a binary. ClaudeCode via npm would instead run 'npm install -g @anthropic-ai/claude-code@VERSION'.
- **Found:** CLI wiring requires additions in three enum/match sites in commands/cli.rs: BuildAgent, RunAgent, ShellAgent, plus the run/shell/build match arms.
- **Found:** npm allows pinning (npm install -g pkg@1.2.3) and querying latest (npm view pkg version). Debian APT does NOT expose version numbers via API without running apt on the host.
- **Found:** ClaudeCode config lives at ~/.claude – needs a config_dir module analogous to opencode/config_dir.rs and pi/config_dir.rs.
- **Found:** Passthrough env vars for ClaudeCode: ANTHROPIC_API_KEY plus CLAUDE_* prefix vars.
- **Decided:** npm is the better installation method: version pinning via @VERSION tag, latest version query via npm registry API (registry.npmjs.org), clean uninstall. Debian APT has no programmatic version API and requires adding external apt sources to the image.
- **Open:** What Claude Code config dir path is used on Linux (~/.claude, ~/.config/claude, or other)?
- **Open:** Should ANTHROPIC_API_KEY be shared with other agents' passthrough lists or declared per-agent (current pattern is per-agent)?

## [15d5baa] Phase 1: NpmRegistryFetcher committed

Added NpmRegistryFetcher to crates/cast/src/dev/version/fetcher.rs. Hits registry.npmjs.org/<pkg>/latest, deserializes the `version` field. Also picked up a cargo fmt trailing-comma fix in pi/mod.rs.

- **Decided:** Include pi/mod.rs trailing-comma fmt fix in the same commit (style, not logic)

## [e600dcc] Phase 2: Dockerfile.dev.claudecode committed

Created assets/Dockerfile.dev.claudecode. Uses node:lts-trixie-slim as base (Node + npm pre-installed). Installs @anthropic-ai/claude-code@${AGENT_VERSION} via npm. Creates ~/.claude, ~/.config, ~/.cache, ~/.local directories. Sets git config with claudecode identity. CMD ["claude"].

## [b3c12f9] Phase 3: claudecode agent module committed

Created crates/cast/src/dev/claudecode/{config_dir.rs,env.rs,mod.rs}. Added pub mod claudecode to dev/mod.rs. 10 unit tests all pass. config_dir uses ~/.claude (home-relative). env passthrough covers Anthropic direct, Bedrock, Vertex. Agent impl follows pi pattern exactly.

## [e981d5a] Phase 4: CLI wiring committed — implementation complete

Wired ClaudeCode into commands/cli.rs: import, BuildAgent::Claudecode, RunAgent::Claudecode (alias "c"), ShellAgent::Claudecode, all match arms, and RunAgent::as_agent(). Updated cli_test.rs with 2 new tests (build claudecode help, shell claudecode help) and expanded port test to cover all three agents. All 14 CLI tests + 10 unit tests pass.

- **Decided:** Use alias 'c' for claudecode run subcommand (matches plan)

