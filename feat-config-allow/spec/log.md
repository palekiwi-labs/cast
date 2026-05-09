# Project Log

## [8e4aef7] Research complete: cast config allow/deny

- **Found:** Config fields in src/config/schema.rs identified for hashing
- **Found:** Interception point for approval check found in src/dev/run.rs#run_agent
- **Found:** Proposed persistence in ~/.local/share/cast/approved_configs.json

## [58e35e1] Implementation plan complete: cast config allow/deny

- **Decided:** Hash includes workspace root path (per-project approval)
- **Decided:** All Config fields are hashed (full snapshot)
- **Decided:** Approval store at ~/.local/share/cast/approved_configs.json
- **Decided:** Canonicalize HashMap serialization by sorting JSON object keys recursively
- **Decided:** Interception point in run_agent before nix_daemon::ensure_running
- **Decided:** New module: src/config/approval.rs

## [58e35e1] Plan updated with Gemini review findings

- **Found:** as_encoded_bytes() does not exist — must use OsStrExt::as_bytes() on Unix
- **Found:** workspace path must be canonicalized with std::fs::canonicalize before hashing
- **Found:** ApprovalStore.entries must use BTreeMap not HashMap for deterministic JSON
- **Found:** load_approval_store must distinguish NotFound from other IO errors
- **Decided:** File permissions: parent dir 0o700, store file 0o600
- **Decided:** cast config show enhanced to display hash and approval status
- **Decided:** Error message in run_agent includes short hash and env-var override note
- **Decided:** cast config deny removes only the current hash (not all workspace approvals)

