# Flake Integration

`cast` can automatically detect and use your project's Nix flake.

## Enabling Integration

Set `use_flake` to `true` in your `cast.json`:

```json
{
  "use_flake": true
}
```

## How it works

If enabled, `cast` constructs a "Russian Doll" of shell wrappers:
1. `nix develop <global_flake> -c` (optional)
2. `nix develop <project_flake> -c`
3. `<agent_binary>`

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

When you run `cast run opencode`, the agent will have `node`, `python3`, and `jq` available in its `PATH`.
