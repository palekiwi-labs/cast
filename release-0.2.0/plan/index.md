---
status: open
refs:
  - .cue/master/task/env-var-passthrough.md
  - .cue/master/task/build-repo-defined-shells.md
  - .cue/master/task/cast-local-json-overrides.md
  - .cue/master/task/setup-renovate.md
  - .cue/master/task/update-dependencies-0.2.0.md
  - .cue/master/task/audit-documentation-0.2.0.md
---

# Master plan: ship 0.2.0

## Goal

Converge 0.2.0 scope, land its stories on master, and tag + push `v0.2.0`.

## Scope

- [x] `env-var-passthrough` merged in PR #64.
- [x] `cast-local-json-overrides` merged in PR #65.
- [x] `build-repo-defined-shells` merged to master.
- [x] `setup-renovate` landed on master.
- [x] `update-dependencies-0.2.0` landed on master.
- [ ] `audit-documentation-0.2.0` audited and updated the docs on master.

## Release gate

- [ ] Confirm every task tagged `0.2.0` is complete or closed.
- [ ] Audit crate versions and decide the `cast-mcp-client` version.
- [ ] Finalize the `0.2.0` changelog and migration notes.
- [ ] Confirm documentation audited and fully updated
      (`audit-documentation-0.2.0`).
- [ ] Run the full test suite, formatting checks, and clippy on master.
- [ ] Tag `v0.2.0` on master and push the tag.
- [ ] Record the release outcome and complete this task.
