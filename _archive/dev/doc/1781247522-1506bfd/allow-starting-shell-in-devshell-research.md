# Research Report: Allow Starting Shell in Devshell

## Research Question
How can `cast shell <agent>` be modified to automatically start inside a Nix devshell, and what are the implications for current implementation?

## Summary
`cast` already possesses the logic for wrapping commands in `nix develop` layers (global flake and project flake), but this logic is currently only applied during `cast run`. `cast shell` is implemented as a direct `docker exec` into `/bin/bash`. Implementing the requested feature involves exposing this wrapping logic to the `shell` command and adding appropriate CLI flags to control the behavior.

## Findings

### 1. Current Shell Implementation
The `shell` command is hardcoded to execute `/bin/bash` in the target container.

**File:** `crates/cast/src/dev/shell.rs`
```rust
31:     let exec_args = vec![
32:         "exec".to_string(),
33:         "-it".to_string(),
34:         container_name,
35:         "/bin/bash".to_string(),
36:     ];
37: 
38:     docker.interactive_command(exec_args)
```

### 2. Nix Wrapping Logic
`cast` uses a "Russian Doll" nesting approach for Nix flakes, implemented in `build_command.rs`. It can wrap any command in global and/or project-level `nix develop` calls.

**File:** `crates/cast/src/dev/build_command.rs`
```rust
25:     if opts.user_flake_present {
26:         let global_flake = format!("/home/{}/.config/cast/nix", opts.user.username);
27:         cmd.extend([
28:             "nix".to_string(),
29:             "develop".to_string(),
30:             global_flake,
31:             "-c".to_string(),
32:         ]);
33:     }
```

### 3. Global Flake Detection
The global flake (located at `~/.config/cast/nix/` on the host) is already detected and mounted into agent containers.

**Detection (in `crates/cast/src/dev/run.rs`):**
```rust
81:     let user_flake_present = host_home_dir
82:         .as_ref()
83:         .filter(|h| h.join(".config/cast/nix/flake.nix").exists())
84:         .is_some();
```

**Mounting (example from `crates/cast/src/dev/opencode/mod.rs`):**
```rust
77:                     "{}:/home/{}/.config/cast/nix:rw",
```

### 4. CLI Configuration
The `shell` command variants are defined in `cli.rs`. They currently do not accept any arguments or flags.

**File:** `crates/cast/src/commands/cli.rs`
```rust
203: #[derive(Subcommand)]
204: #[command(subcommand_required = true)]
205: pub enum ShellAgent {
206:     /// Drop into an interactive shell in the OpenCode container
207:     Opencode,
208:     /// Drop into an interactive shell in the Pi container
209:     Pi,
210:     /// Drop into an interactive shell in the ClaudeCode container
211:     Claudecode,
212: }
```

## Implementation Considerations

1.  **Shared Logic**: The `RunOpts` resolution logic in `crates/cast/src/dev/run.rs` should be shared or replicated in `shell.rs` to correctly detect the presence of flakes.
2.  **Wrapping the Shell**: Instead of passing `"/bin/bash"` directly to `docker exec`, `dev::shell` should use `build_command::build_command` with `"bash"` (or the user's preferred shell) as the base command.
3.  **Default Behavior**: The user suggested starting in the global devshell by default if present. This would change the current behavior where `cast shell` always drops into a "raw" bash shell. A `--raw` flag would then be needed to bypass the devshell.
4.  **Error Handling**: As noted in the request, starting in a devshell carries the risk of locking the user out if Nix fails. If `nix develop` fails, the `docker exec` command will likely return a non-zero exit code and terminate the session.

## Unanswered Questions
- Should the `project` flake also be applied by default if `use_flake` is true in `cast.json`? Currently, `cast run` applies both layers if configured.
- Should we support specifying a different shell than `/bin/bash`?
