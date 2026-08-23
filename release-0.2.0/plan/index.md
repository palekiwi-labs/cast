---
status: open
refs:
  - .cue/master/task/env-var-passthrough.md
  - .cue/master/task/design-repo-defined-shells.md
---

# Master plan: ship 0.2.0

## Goal

Converge 0.2.0 scope, land its stories on master, and tag + push `v0.2.0`.

## Principles

- Breaking changes allowed; strictly no backward compatibility, no deprecation
  shims, no trace of superseded behaviour in the release.
- A story enters 0.2.0 only via `tag: 0.2.0` frontmatter on its task card on the
  master board. Adding or removing a story is a user decision made in this task's
  discussion.

## Story tracker
### env-var-passthrough (build, in-progress)

- [x] Pivot design (base + `extra_env_passthrough`, zero-trace removal
      of hardcoded agent forwarding) recorded on the task card;
      original master and executive plans closed as superseded
- [x] New executive plan drafted for the pivot (next agent)
      (`.cue/env-var-passthrough/plan/1787026202-d86c37f/`)
- [ ] Pivot commits appended to `feat/env-passthrough`
- [ ] Review pass on the appended commits
- [ ] Branch merged to master

### design-repo-defined-shells (design, in-progress)

- [ ] Design converged with the user (the story's own completion criterion:
  explicit mutual agreement, not just a spec artifact)
- [ ] Build task(s) spawned, tagged `0.2.0`, linked here
- [ ] Implementation merged to master

## Release gate

Run in order; all must pass before tagging.

- [ ] Scope frozen: every task tagged `0.2.0` is complete/closed; nothing else
  pulled in without user agreement
- [ ] Version audit: `crates/cast` is 0.2.0 (already); decide whether
  `cast-mcp-client` rides at 0.1.0 or bumps
- [ ] CHANGELOG.md: fold `[Unreleased]` into the existing `[0.2.0]` section (it
  was pre-written at the universal-container merge but never tagged); Keep a
  Changelog format. Must lead with breaking changes:
- env passthrough redesigned: `env_passthrough` + `extra_env_passthrough`;
  hardcoded agent var forwarding removed; manual migration: add provider API keys
  to global `cast.json` or containers lose agent auth
- repo-defined shells changes, per the converged design
- [ ] master green: full test suite, clippy; decide the deferred repo-wide
  formatting commit (pre-existing rustfmt mismatches in cli.rs, build.rs,
  image.rs, possibly loader.rs imports)
- [ ] `v0.2.0` tagged on master and pushed
- [ ] Post-tag: record outcome; reopen this task only for release hotfixes

## Sequencing notes

- The env-var-passthrough pivot can proceed immediately; the branch exists and
  the design is settled.
- design-repo-defined-shells gates its own build work; start its design session
  whenever the user is ready. Nothing in the passthrough pivot depends on it.
