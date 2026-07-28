---
title: Cleanup post nix-native harnesses
priority: normal
status: complete
refs:
  - /home/pl/code/palekiwi-labs/cast/.cue/master/task/nix-native-harnesses.md
branch:
  - chore/cleanup-post-nix-native-harnesses
---

# Cleanup post nix-native harnesses

Scan the code for any opportunities to clean up dead code resulting from the
pivot to nix native harnesses.

Example: auto-update by agent harness version check make no sense any more

## Source

- Trace: `.cue/cleanup-post-nix-native-harnesses/trace/1784916739-c6794d5/dead-code-scan.md`
- Plan: `.cue/cleanup-post-nix-native-harnesses/plan/1784916739-c6794d5/cleanup.md`

## Acceptance Criteria

1. **`ureq` dependency removed.**
   - Verify by: `grep ureq Cargo.toml` is empty.
   - Evidence: commit ab1d38e.

2. **Stale comments fixed.**
   - Verify by: manual inspection of `run.rs`, `daemon.rs`, `agent.rs`.
   - Evidence: commit ab1d38e.

3. **`cast build <agent>` collapsed to `cast build`.**
   - Verify by: `cast build --help` shows no agent subcommand.
   - Evidence: commit 4636b77.

4. **`--base` renamed to `--nix-daemon`.**
   - Verify by: `cast build --help` shows `--nix-daemon`.
   - Evidence: commit 67d208c.

5. **Docs and Changelog updated.**
   - Verify by: `CHANGELOG.md` and `reference.md` reflect changes.
   - Evidence: commit da775a4 and 67d208c.

6. **Tests pass.**
   - Verify by: `cargo test -p cast`.
   - Evidence: 251 passed; 0 failed.

7. **Manual QA passed.**
   - Verify by: human attestation
   - Evidence: attested by user 2026-07-27.

