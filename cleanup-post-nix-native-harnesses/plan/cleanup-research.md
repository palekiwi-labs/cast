---
status: complete
---
# Research Plan: Cleanup post-nix-native-harnesses

Thorough scan of the codebase for dead code lingering after the pivot to Nix-provisioned agent harnesses.

## Phase 1: Broad Keyword Search
- [x] Grep for primary keywords: `agent_versions`, `universal_container`, `version_check`, `auto_update`, `autoupdate`, `self_update`, `content_hash`, `image_tag`, `harness_version`, `update_check`, `per-agent`, `ImageTag`.
- [x] Grep for secondary patterns related to Docker image building and per-agent logic.

## Phase 2: Analysis of Findings
- [x] Analyze search results for each keyword.
- [x] Identify structs, functions, and modules that appear orphaned.
- [x] Check for documentation references that are now incorrect.
- [x] Verify if any remaining references are truly dead or have a new purpose.

## Phase 3: Detailed Reporting
- [x] Document each finding with file path, line number, description of why it's dead, and confidence level.
- [x] Check for existing callers/references for each finding.
- [x] Compile a structured list for the final report.
