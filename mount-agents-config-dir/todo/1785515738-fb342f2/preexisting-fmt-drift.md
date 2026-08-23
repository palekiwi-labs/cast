---
status: open
priority: low
refs: undefined
---
# Pre-existing `cargo fmt` drift in cast crate

While validating the mount-agents-config-dir task, `cargo fmt -p cast -- --check`
reports 6 diffs that pre-exist on master (confirmed via `git stash` on
feat/mount-agents-config-dir). None were introduced by the .agents work.

Affected files / nature:
- crates/cast/src/commands/cli.rs — use-list ordering
- crates/cast/src/dev/build.rs — use ordering (BuildOptions)
- crates/cast/src/dev/image.rs — use ordering (BuildOptions)
- crates/cast/src/dev/run.rs — use-list ordering + two over-long `assert!(...)`
  lines (CAST_MCP_URL) that rustfmt wants wrapped

Out of scope for mount-agents-config-dir. A separate cleanup commit should run
`cargo fmt -p cast` to normalize these.
