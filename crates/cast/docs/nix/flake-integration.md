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

`cast` also supports a global flake that applies to all projects. If a
`flake.nix` is found in `~/.config/cast/nix/`, it will be used as the outer
layer of the shell wrapping. This is useful for tools you want available in
every agent session, regardless of the specific project.

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

1. `nix develop <global_flake> -c` (optional)
2. `nix develop <project_flake> -c`
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
