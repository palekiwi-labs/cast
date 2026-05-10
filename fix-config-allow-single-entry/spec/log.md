# Project Log

## [28f4486] Research complete: cast config allow overwrite

- **Found:** ApprovalStore uses BTreeMap with hash as key, leading to multiple entries per workspace
- **Found:** deny_workspace_config already removes all workspace entries

## [1285330] Implemented config allow overwrite

- **Found:** ApprovalStore now enforces one entry per workspace in add_entry
- **Decided:** Applied overwrite logic at the store level (add_entry) to ensure the invariant regardless of caller

## [692b62b] Feature complete: cast config allow overwrite

- **Found:** Verified that allow overwrites and deny removes all entries
- **Decided:** Maintained standard deny behavior for safety and consistency

## [e8d3ffc] Fixed path canonicalization mismatch

- **Found:** Found that inconsistent path strings allowed multiple approvals via symlinks
- **Decided:** Canonicalize all paths at the API boundary to ensure consistent string matching in the store

## [e8d3ffc] Research complete: Drop opencode_config special handling

- **Found:** Special handling in src/dev/opencode/mod.rs can be replaced by extra_data_volumes in cast.json and environment variables in cast.env
- **Found:** extra_data_volumes already supports bind mounts and tilde expansion
- **Found:** Dropping this simplifies the codebase and increases transparency
- **Decided:** The special handling can be safely removed in favor of explicit user configuration

