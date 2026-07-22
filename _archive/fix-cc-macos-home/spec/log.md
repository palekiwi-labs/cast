# Project Log

## [2ce36c4] Root cause analysis: ClaudeCode on macOS fails with git safe.directory error

- **Found:** The error is 'libgit2 error code = 7' (GIT_ERROR_REPOSITORY_NOT_OWNED) — git refuses to read the workspace repo because it's not owned by the current user inside the container.
- **Found:** On macOS, Docker Desktop maps host paths like /Users/alex/... into the container. When Nix runs 'nix develop .' it uses libgit2 to resolve the flake, and git's safe.directory protection blocks it.
- **Found:** Dockerfile.nix-daemon already has 'RUN git config --global safe.directory "*"' with a comment explaining exactly why.
- **Found:** Dockerfile.dev.claudecode has NO git config at all — this is the bug.
- **Found:** Dockerfile.dev.opencode and Dockerfile.dev.pi have git config blocks but are ALSO missing safe.directory — same latent bug.
- **Found:** Existing tests in claudecode/mod.rs check Dockerfile content (FROM image, COPY node statement) — a test for safe.directory can follow the same pattern.
- **Decided:** Fix is in Dockerfile.dev.claudecode: add 'RUN git config --system safe.directory "*"'
- **Decided:** Apply same fix to opencode and pi Dockerfiles for consistency and to prevent same bug there
- **Decided:** Add a regression test to each agent's mod.rs Dockerfile content tests
- **Open:** Is the fix scoped to claudecode only (per the task title) or should it cover all three agents?

## [f386b14] Cycle 1 GREEN: claudecode Dockerfile safe.directory fix committed

Added `RUN git config --system safe.directory \"*\"` to Dockerfile.dev.claudecode and a regression test. Commit f386b14.

- **Found:** Test went RED before Dockerfile change, GREEN after — TDD cycle confirmed clean

## [be4a9e5] All three TDD cycles complete — safe.directory fix shipped

Three commits on fix/cc-macos-home. 205 tests passing, 0 failures, clippy clean.

- **Found:** All three agent Dockerfiles now have `RUN git config --system safe.directory "*"` and nothing else in the git config section
- **Found:** opencode and pi Dockerfiles lost 6 lines of unnecessary git config each (user.name, user.email, init.defaultBranch, commit.gpgsign, core.autocrlf, pull.rebase)
- **Found:** Regression tests added to all three agent mod.rs files asserting the Dockerfile content
- **Decided:** Used --system scope (writes to /etc/gitconfig) rather than --global (root home), consistent with how other config is done in dev containers and appropriate since containers run as USERNAME not root

