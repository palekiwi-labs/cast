# Prepare repo for a 0.1.0 release

## Documentation

### Top level: `docs/`

Add a top level `docs/` entrypoint with the following entries:

- `index.md`: table of contents
- `ARCHITECTURE.md`: explain the architecture of this project

### Crate level: `crates/<crate>/docs/`

This directory already exists for `cast` crate and plays a special
role: it is served by the built-in `cast mcp` server.

For both crates we can introd

- `index.md`: table of contents
- `ARCHITECTURE.md`: explain the architecture of this project
- `TESTING.md`
- other entries depending on codebase:
  - `mcp/configuration.md` (already exists)
  - `mcp/client.md`
  - nix
  - config
  - Agent trait supporting different harnesses
  

## Release boilerplate

- README.md
- LICENSE: GPL or MIT?
- CHANGELOG.md: for the first release we probably don't need it?
- Taskfile.yml
  - task: `prepare-release`
    * automates tagging releases on `master` branch
    * tags a commit with version
    * pushes the commit and the tag

