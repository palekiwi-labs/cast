# Flake Integration

`cast` can enter explicit sandbox and project Nix devshells around an agent
command. It does not detect flakes or choose shell fragments automatically.

## Enabling Integration

Set either or both shell refs in `cast.json`:

```json
{
  "sandbox_shell": ".#ai",
  "project_shell": ".#default"
}
```

Both values are full flake references and are passed verbatim to
`nix develop <ref> -c` inside the container. Supported forms include `.#shell`,
`/absolute/path#shell`, `~/.config/cast/nix#default`, and
`github:org/repo#shell`. Relative refs resolve from the mounted workspace.

`use_sandbox_shell` and `use_project_shell` default to `true`. Set the
corresponding switch to `false` to skip a configured layer silently. An unset
ref also means that layer is absent.

## Sandbox Flake

The outer shell serves two roles: it provides tools you want available in every
agent session and, in the nix-native model, provides the **agent harnesses
themselves**. Its location is entirely controlled by `sandbox_shell`.

### Shell selection

For this configuration:

```json
{
  "sandbox_shell": "~/.config/cast/nix#opencode"
}
```

the outer wrapper becomes:

```
nix develop ~/.config/cast/nix#opencode -c ...
```

There is no agent-name fallback or placeholder substitution. To use a minimal
per-harness shell, select its complete ref explicitly. To use one shell for all
agents, point `sandbox_shell` at a devshell containing all harness packages. The
template's `default` shell does this.

### Bootstrap

Initialize the shipped global config and flake explicitly:

```sh
cast config init
```

The command creates `~/.config/cast/cast.json` and
`~/.config/cast/nix/flake.nix`. It never overwrites either file: when one
already exists, it reports the skip and still creates the other if needed.
`cast run` and `cast exec` never scaffold these files automatically.

The `~/.config/cast/nix` directory is bind-mounted at the same path inside the
container whenever the directory exists, even if it has no `flake.nix` yet.
Flakes elsewhere must be under the mounted workspace or exposed through a
user-configured mount.

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

`cast config init` seeds `~/.config/cast/cast.json` with the numtide substituter
and public key. Edit this file to add or remove caches. Existing files are never
overwritten.

If you scaffolded your global flake before this change, it still contains a
`nixConfig` block — `cast` never overwrites an existing flake. Delete the
block by hand to stop the approval prompt. You may also need to remove a stale
`~/.local/share/nix/trusted-settings.json`, where nix records approval answers
it was given previously.

Harness versions are pinned by the selected flake's `flake.lock`, not by
`cast.json`.

### Template shape

The shipped template defines a `default` shell containing shared base tooling
and all supported harnesses, plus one minimal devshell per harness (`opencode`,
`pi`, `claudecode`). Shells are composed from a shared `baseInputs` list via a
small `mkShell` helper, so you can add tools every harness should see in one
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

For configured and enabled refs, `cast` constructs a "Russian Doll" of shell
wrappers:

1. `nix develop <sandbox_shell> -c` (outer harness layer)
2. `nix develop <project_shell> -c` (inner project layer)
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

With `"project_shell": ".#default"`, the agent will have `node`, `python3`,
and `jq` available in its `PATH`.
