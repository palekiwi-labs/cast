# nix flake commands

## Purpose

Design and add utility commands that will let user create and manage the global environment
flake in `~/.config/cast/nix`

## Scope

Begin with a minimal set of operations, e.g.:
- creating a flake (`nix flake init`)
- updating the flake (`nix flake update`)

## Example usage

- `cast flake init`  (NOTE: or `cast nix flake init`?)
  - ensures parent dir is created in `~/.config/cast/nix`
  - wraps `nix flake init` calling this inside the dev container (NOTE: but which container?)
  - creates a default flake in `~/.config/cast/nix/flake.nix`
  - prints the path to the flake to stdout
  - if a flake already exists, exits with an error

- `cast flake init --template (-t) <path-or-url-to-a-template>`
  - passes the tmmplate to `nix flake init --t`
