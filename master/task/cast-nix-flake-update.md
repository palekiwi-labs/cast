---
status: open
priority: normal
---
## Context

The global shell (`~/.config/cast/nix/flake.nix`) has grown from an optional
feature to a fundamental component of `cast` as it now also defines all the
agent harnesses via `llm-agents`. Moreover, cast now initializes the flake for
every user which means that every installation has a flake.

Up till now I have had to run `nix flake update` manually but other users may
need a dedicated command, such as `cast nix flake update`.
