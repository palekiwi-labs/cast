# Configuration Approval

For security, `cast` will not run an agent sandbox unless the project
configuration has been approved.

## Why Approval is Required

Agents run inside sandboxes, but `cast` itself runs on your host. If `cast`
automatically loaded a malicious `cast.json` from a project you just
downloaded, it could compromise your host (e.g., by mounting sensitive
directories).

## How it works

1. `cast` calculates a SHA256 hash of:
   - The canonical path of the workspace.
   - The serialized content of the merged configuration.
2. It checks if this hash is present in the approval store
   (`~/.local/share/cast/approved_configs.json`).
3. If the hash matches, the session proceeds. If not, `cast` errors and asks you
   to run `cast config allow`.

## Managing Approvals

- `cast config allow`: Approve the current state.
- `cast config deny`: Revoke approval.
- `cast config diff`: See exactly what changed since your last approval.

For implementation details, see
[crates/cast/src/config/approval.rs](../../src/config/approval.rs).
