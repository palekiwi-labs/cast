# Goal: `cast config allow` Overwrite Behavior

Ensure that `cast config allow` maintains exactly one approved configuration version per workspace by overwriting existing approvals rather than appending to them.

## Context
Currently, the `ApprovalStore` uses configuration hashes as keys in a `BTreeMap`. When a user runs `cast config allow` on a modified configuration, a new entry is added, but the old entry for that same workspace remains. This could allow stale or previously approved configurations to remain valid indefinitely.

## Requirements
1.  `cast config allow` must remove any existing approved hashes for the current workspace before adding the new hash.
2.  `cast config deny` should continue to remove all approved hashes for the workspace.
3.  The implementation should prioritize security by ensuring no stale configurations are left in an "approved" state for a workspace.
