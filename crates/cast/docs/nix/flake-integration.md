# Flake Integration

`cast` can automatically detect and use your project's Nix flake.

## Enabling Integration

Set `use_flake` to `true` in your `cast.json`:

```json
{
  "use_flake": true
}
```

## Global Flake

`cast` uses a global flake at `~/.config/cast/nix/flake.nix` as the outer layer
of the shell wrapping. This flake serves two roles: it provides tools you want
available in every agent session, and — in the nix-native model — it provides
the **agent harnesses themselves** through named devShells.

### Shell selection

`cast run <agent>` enters the global devShell named after the agent. For
`cast run opencode`, the outer wrapper becomes:

```
nix develop ~/.config/cast/nix#opencode -c ...
```

Set the `global_shell` config field to override which devShell is entered,
regardless of the agent. For example, to route every agent through a single
`universal` shell that exposes all harnesses:

```json
{
  "global_shell": "universal"
}
```

When `global_shell` is unset (the default), the agent name is used as the
shell fragment.

### Auto-scaffolding

The first `cast run <agent>` on a fresh host works with no manual setup: if
`~/.config/cast/nix/flake.nix` is absent, `cast` scaffolds it from a shipped
template (a loud notice is printed). An existing flake is never overwritten.
Scaffolding is best-effort — a failure warns and continues rather than aborting
the run.

You can also initialise the template explicitly with Nix:

```bash
nix flake init -t github:palekiwi-labs/cast#global
```

### Harness fetches and the numtide cache

Prebuilt harness packages (from `numtide/llm-agents.nix`) should be fetched
from the `cache.numtide.com` binary cache rather than built from source. How
that cache reaches the builder differs between environments.

Inside `cast`'s dev container the nix daemon runs with `trusted-users = root`
(a security hardening: the non-trusted dev user must not be able to override
substituters or import unsigned paths into the shared `/nix` store). Because
the dev user is non-trusted, flake-declared substituters/keys forwarded from
the client are **rejected** by the daemon. The dev image therefore sets
`accept-flake-config = false` in its system `/etc/nix/nix.conf`.

For that reason the shipped template declares **no `nixConfig` block**. One
would be actively harmful: with `accept-flake-config = false` nix prompts for
approval on every devshell entry, and the prompt cannot be usefully answered —
the daemon rejects the settings whichever way it is answered.

Instead, the cache is provisioned **daemon-side** from
`~/.config/cast/cast.json`: `cast` reads `nix_extra_substituters` /
`nix_extra_trusted_public_keys` and bakes them into the daemon's server-side
config via `generate_nix_conf`. The daemon (root, trusted) then performs
substitution using its own config, so caches work for the non-trusted dev user.

To avoid a silent regression to multi-minute source builds, the first
`cast run`/`cast exec` on a fresh host seeds a default
`~/.config/cast/cast.json` containing the numtide substituter and public key
(a loud notice is printed; an existing `cast.json` is never overwritten). Edit
this file to add or remove caches.

If you scaffolded your global flake before this change, it still contains a
`nixConfig` block — `cast` never overwrites an existing flake. Delete the
block by hand to stop the approval prompt. You may also need to remove a stale
`~/.local/share/nix/trusted-settings.json`, where nix records approval answers
it was given previously.

Harness versions are pinned by the global flake's `flake.lock`, not by
`cast.json`.

### Template shape

The shipped template defines a `default` shell (shared base tooling), one
devShell per harness (`opencode`, `pi`, `claudecode`), and a `universal` shell
exposing all harnesses. Shells are composed from a shared `baseInputs` list via
a small `mkShell` helper, so you can add tools every harness should see in one
place. Edit this flake to pin versions, add tools, or add shells.

### Keeping stdout clean

`cast` prints its own status messages to stderr. For `cast run --headless
--format json` to produce clean, pipeable JSON, any `shellHook` echoes in your
flakes must also write to stderr. Use `>&2`:

```bash
# ~/.config/cast/nix/flake.nix — shellHook snippet
shellHook = ''
  echo "Global environment loaded." >&2
'';
```

This applies to both the global flake and project-level flakes. Anything
written to stdout inside a `shellHook` will appear in `cast`'s stdout and
will corrupt a JSON pipeline.

## How it works

If enabled, `cast` constructs a "Russian Doll" of shell wrappers:

1. `nix develop <global_flake>#<shell> -c` (harness layer; `<shell>` is the
   agent name or `global_shell`)
2. `nix develop <project_flake> -c` (only when `use_flake` is enabled)
3. `<agent_binary>`

This wrapping also applies to `cast shell`, so an interactive shell starts
inside the devshell by default. Use `cast shell --raw <agent>` to bypass it.

## Example `flake.nix`

A typical project flake for use with `cast` might look like this:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.nodejs
            pkgs.python3
            pkgs.jq
          ];
        };
      }
    );
}
```

When you run `cast run opencode`, the agent will have `node`, `python3`, and
`jq` available in its `PATH`.
