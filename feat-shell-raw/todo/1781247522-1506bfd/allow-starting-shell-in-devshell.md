---
status: complete
priority: 5
---

## Context

Pretty much every time I run `cast shell <agent>` I have to immediately
follow it inside the shell with `nix develop ~/.config/cast/nix`.

The reason the shell does not immedialy start inside a nix devshell
is to prevent the user from locking themselves out from shell access
when nix fails for any reason. 

## Scope

Consider the following options:

1. start in the nix global devshell by default if present and add
  a `--raw` (or base, or something better named) flag that runs without
  the shell (current default behavior)

2. add a flag to `cast shell <agent> ` that automatically enters
  the global flake located in `/home/pl/.config/cast/nix/`.
