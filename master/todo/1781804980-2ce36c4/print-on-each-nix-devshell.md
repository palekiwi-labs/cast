---
priority: normal
title: Print on each nix devshell
status: closed
---
Print messages to the terminal when

1. `cast` loads the global nix devshell
2. `cast` loads the project devshell

`1.` should be easy to add, but is it possible to add `2`
given that we are wrapping the sessions russian doll style?

Having this information would improve transparency and allow
users to know what is happening.

An example of what we print right now is:

```bash
opencode dev image already exists: localhost/cast:0.1.0-opencode-1.16.2 # `cast` prints this to stdout
CAST Global Nix Environment Loaded # comes from the shell hook in `~/.config/cast/nix/nix.flake`
Rust development environment ready! # comes from the project devshell hook
Rust version: rustc 1.94.1 (e408947bf 2026-03-25) # comes from the project devshell hook
```


## Scope

We need to ensure that:

- **IMPORTANT** we print all `cast` messages, including the existing ones to stderr
  so that we can use `cast` programmatically with the `--headless` flag
- print new "marker" statements when `cast` runs `nix develop`
