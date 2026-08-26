---
status: open
priority: high
refs:
  - .cue/design-repo-defined-shells/spec/index.md
  - .cue/build-repo-defined-shells/plan/index.md
---

# Manual QA: repo-defined shells

Human verification checklist for branch `build-repo-defined-shells`.
Do not mark a step complete unless it was personally observed.

## Safety and setup

- [x] Confirm the checked-out branch is `build-repo-defined-shells` and the working tree is clean.
- [x] Build the branch's `cast` binary and confirm `cast --version` runs.
- [x] Confirm Docker and the cast Nix daemon are available.
- [x] Back up any existing `~/.config/cast/cast.json` and `~/.config/cast/nix/flake.nix` before testing initialization.
- [x] Prepare a disposable repository or temporary workspace with no important `cast.json`, `cast.local.json`, or flake files.

## Global initialization

- [x] With both global files absent, run `cast config init`.
- [x] Confirm it creates `~/.config/cast/cast.json` and `~/.config/cast/nix/flake.nix`.
- [x] Confirm the generated config contains `sandbox_shell: "~/.config/cast/nix#default"` and the expected Numtide cache settings.
- [x] Confirm the generated flake's default shell contains the supported agent harnesses.
- [x] Confirm the generated flake has no live `nixConfig` block.
- [x] Run `cast config init` again and confirm it reports both files as skipped without modifying their contents.
- [x] Remove only the generated flake, rerun `cast config init`, and confirm it preserves the config while recreating the flake.
- [x] Remove only the generated config, rerun `cast config init`, and confirm it preserves the flake while recreating the config.
- [x] Temporarily make the existing config invalid JSON, run `cast config init`, and confirm initialization still preserves it and creates a missing flake.

## Default sandbox shell

- [x] In the disposable workspace, approve the effective configuration with `cast config allow` if required.
- [x] Run a supported agent using the generated default configuration.
- [x] Confirm cast announces `Loading sandbox nix devshell...`.
- [x] Confirm the literal `~/.config/cast/nix#default` config value successfully resolves to `/home/<container-user>/.config/cast/nix#default` inside the container.
- [x] Confirm the selected agent binary exists and starts successfully inside the sandbox shell.
- [x] Verify the same default sandbox shell behavior through a non-raw `cast exec` command.
- [x] Verify a non-raw `cast shell` enters successfully for a running container.
- [x] Confirm `cast exec --raw` and `cast shell --raw` bypass the configured Nix shell wrapping.

## Repository-defined project shell

- [x] Add a simple project `flake.nix` whose devshell exports a unique marker variable or prints a unique shell-hook message.
- [x] Set `project_shell` to `.#default` in the repository's `cast.json`.
- [x] Approve the changed project configuration.
- [x] Run a non-raw command and confirm both layers load in order: sandbox outer, project inner.
- [x] Confirm the project-shell marker is visible to the agent or executed command.
- [x] Confirm the relative `.#default` ref resolves from the mounted workspace.
- [x] Set `project_shell` to a leading-tilde local ref and confirm it resolves against the container user's home.
- [x] Set `project_shell` to an invalid ref and confirm Nix reports the resolution failure without cast silently falling back to another shell.

## Layer switches and empty values

- [ ] Set `use_sandbox_shell` to `false` and confirm the sandbox layer and its announcement are both absent.
- [ ] Set `use_project_shell` to `false` and confirm only the sandbox layer loads.
- [ ] Disable the sandbox layer while leaving the project layer enabled and confirm only the project layer loads.
- [ ] Disable both layers and confirm the bare command is attempted without `nix develop` wrapping.
- [ ] Set `CAST_SANDBOX_SHELL` to an empty value and confirm cast does not invoke `nix develop ""` or announce a sandbox shell.
- [ ] Set `CAST_PROJECT_SHELL` to an empty value and confirm cast does not invoke `nix develop ""`.
- [ ] Repeat the empty-value checks with whitespace-only configuration values if practical.

## Configuration merge and approval

- [ ] Confirm global `sandbox_shell` applies when project configuration does not override it.
- [ ] Override `sandbox_shell` in project `cast.json` and confirm the project value wins only after approval.
- [ ] Override shell refs in `cast.local.json` and confirm local values take precedence over project values.
- [ ] Confirm `CAST_SANDBOX_SHELL`, `CAST_PROJECT_SHELL`, `CAST_USE_SANDBOX_SHELL`, and `CAST_USE_PROJECT_SHELL` override file configuration.
- [ ] Confirm changing a repository-defined shell ref causes the approval state to become changed/unapproved.
- [ ] Confirm legacy `global_shell`, `use_flake`, and `use_flake_path` keys are ignored rather than changing shell selection.

## Existing-file safety

- [x] Confirm `cast config init` does not overwrite user-modified global config or flake files.
- [x] On Linux or macOS, place a dangling symlink at the intended `cast.json` path, run `cast config init`, and confirm the symlink target is not created or overwritten.
- [x] Confirm initialization still creates the independent missing flake when the config path already exists as that symlink.

## Documentation and failure behavior

- [ ] Follow the documented setup from the getting-started and flake-integration guides without relying on undocumented steps.
- [ ] Confirm documentation consistently uses `sandbox_shell`, `project_shell`, `use_sandbox_shell`, and `use_project_shell`.
- [ ] Confirm migration notes mention removal of `global_shell`, `CAST_GLOBAL_SHELL`, `use_flake`, and `use_flake_path`.
- [ ] Confirm stale first-run auto-scaffolding and global-shell terminology are absent from user-facing guidance relevant to this feature.

## Cleanup and attestation

- [ ] Restore the backed-up global cast configuration and flake.
- [ ] Remove disposable containers, temporary workspace files, and test-only Nix artifacts as appropriate.
- [ ] Record the tested operating system, architecture, Docker version, Nix version, and branch commit below.
- [ ] Record any failed, skipped, or ambiguous checks below and create follow-up findings where needed.

## Test environment

- Operating system / architecture:
- Docker version:
- Nix version:
- Commit tested:
- Date tested:
- Tester:

## Notes and findings

- None yet.
