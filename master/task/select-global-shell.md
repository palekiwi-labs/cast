---
priority: high
status: closed
title: Select Global Shell
superseded_by: .cue/master/task/nix-native-harnesses.md
---
> Superseded by `nix-native-harnesses.md`, which merges global-shell selection
> with the nix-native harness pivot. Global-shell selection is now the primary
> harness-selection mechanism there.

## Context

`cast` supports running an agent from inside a global nix devshell.

## Allow config to optionally specify **which** global devshell to use

The flake should be able to define any number of named shells that the user can
pick by setting the config value of a new config field in any of the supported
ways. If no devshell is specified, default to the "default" devshell.

## Extended idea

A nix flake [GH repository][1] exists that provides packages for all the major
agents. Could we pivot to shipping our agent harness packages via the nix flake
rather than pre-baking them inside the images?

In combination with multiple global shells, that could mean that for example a
command `cast run opencode` would default to runnning a global shell with the
name `opencode` which provides the `opencode` pkg. The user could of course
override that with a config setting, e.g. they may want to run `opencode-slim`
or `opencode-extended` devshells that provide different packages.

## Extra considerations

As part of this story, we should also explore "shell composition", as in having
different variations of the same shell in the flake that share some
configurations, e.g. packages but differ in some ways, e.g. environment
variables, extra packages or settings customized to a specific shell.

---

[1]: https://github.com/numtide/llm-agents.nix
