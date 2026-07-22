---
title: Implement cast exec command
status: complete
priority: high
---
## Summary

Add a `cast exec <agent> <cmd> [args...]` subcommand that starts a fresh
container from an agent's image and runs an arbitrary command inside it,
bypassing the agent binary.

## Motivation

- Explore a container environment without starting an agent (`cast run`)
- Debug issues inside a sandboxed container
- Run headless scripts (e.g. a harness loop) without restarting the container

## API

```
cast exec [--headless] [--name <name>] [--publish [<port>]] [--raw] <agent> <cmd> [args...]
```

| Flag | Required | Meaning |
|---|---|---|
| `--headless` | No | No TTY; fire-and-forget |
| `--name` | No | Override container name |
| `--publish`/`-p` | No | Publish port to host (`--publish` = auto, `--publish 8080` = fixed) |
| `--raw` | No | Skip Nix devshell wrapping |
| `<agent>` | Yes | opencode / pi / claudecode (as clap subcommand) |
| `<cmd> [args...]` | Yes | Command + trailing var args, hyphen-values allowed |

## Scope

This task also covers two related changes agreed during design:

### 1. Port publishing redesign

- Remove `config.publish_port: bool` (currently defaults `true`).
- Replace with `--publish`/`-p` CLI flag on `cast run` and `cast exec`.
  - Flag absent → no publish (new default)
  - `--publish` alone → publish the calculated/config port
  - `--publish 8080` → publish host:8080 → container:80
- Keep `config.port: Option<u16>` untouched (used for container naming by `cast shell`/`cast port`).
- `--publish` works in both interactive and headless modes.

### 2. Container naming unification (Option C)

Drop the `-headless-` literal from ephemeral container names.
New invariant:

- **Exactly one stable name**: interactive `cast run` → `cast-{agent}-{basename}-{port}`
- **Everything else is token-suffixed**: `cast-{agent}-{basename}-{port}-{token}`
  - headless `run` token: the invocation ID
  - interactive `exec` token: `exec-{invocation_id}`
  - headless `exec` token: the invocation ID

`resolve_container_name` signature stays token-driven (no `is_exec` param);
the caller supplies the appropriate token string.

## Acceptance Criteria

| # | Criterion |
|---|---|
| AC-1 | `cast exec opencode /bin/bash` starts a fresh interactive container |
| AC-2 | `cast exec --headless opencode ./script.sh` runs headlessly |
| AC-3 | `cast exec --raw opencode /bin/bash -c "echo hi"` skips Nix wrapping |
| AC-4 | `cast exec --publish opencode /bin/bash` publishes the calculated port |
| AC-5 | `cast exec --publish 8080 opencode /bin/bash` publishes host:8080 |
| AC-6 | `cast exec opencode` (no cmd) is rejected by clap with a helpful error |
| AC-7 | `cast run` no longer publishes by default; `cast run --publish opencode` does |
| AC-8 | headless `cast run` names drop `-headless-`: `cast-opencode-<base>-<port>-<token>` |
| AC-9 | interactive `cast exec` name: `cast-opencode-<base>-<port>-exec-<token>` |
| AC-10 | `cast shell` still resolves the stable interactive-run name correctly |
| AC-11 | All existing tests pass; new unit tests cover the above |
