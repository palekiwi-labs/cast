---
status: open
refs: undefined
---
# Research Plan: Cleanup post-nix-native-harnesses

Thorough scan of the codebase for dead code lingering after the pivot to Nix-provisioned agent harnesses.

## Phase 1: Broad Keyword Search
- [ ] Grep for primary keywords: `agent_versions`, `universal_container`, `version_check`, `auto_update`, `autoupdate`, `self_update`, `content_hash`, `image_tag`, `harness_version`, `update_check`, `per-agent`, `ImageTag`.
- [ ] Grep for secondary patterns related to Docker image building and per-agent logic.

## Phase 2: Analysis of Findings
- [ ] Analyze search results for each keyword.
- [ ] Identify structs, functions, and modules that appear orphaned.
- [ ] Check for documentation references that are now incorrect.
- [ ] Verify if any remaining references are truly dead or have a new purpose.

## Phase 3: Detailed Reporting
- [ ] Document each finding with file path, line number, description of why it's dead, and confidence level.
- [ ] Check for existing callers/references for each finding.
- [ ] Compile a structured list for the final report.
