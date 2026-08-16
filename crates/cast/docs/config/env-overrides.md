# Environment Overrides

Every configuration field in `cast` can be overridden by environment variables.

## Naming Convention

- Prefix: `CAST_`
- Case: ALL_CAPS
- Nesting: Double underscore `__`

## Examples

| Config Field   | Env Variable         |
| -------------- | -------------------- |
| `memory`       | `CAST_MEMORY`        |
| `cpus`         | `CAST_CPUS`          |
| `mcp.port`     | `CAST_MCP__PORT`     |
| `mcp.hostname` | `CAST_MCP__HOSTNAME` |
| `use_flake`    | `CAST_USE_FLAKE`     |
| `extra_data_volumes.cargo.target` | `CAST_EXTRA_DATA_VOLUMES__CARGO__TARGET` |

## Passing Host Variables Into the Sandbox (`env_passthrough`)

The overrides above configure `cast` itself. To make a variable from your host
shell visible *inside* the container — a token you do not want written to
`cast.env` on disk — list its **name** in `env_passthrough`:

```json
{
  "env_passthrough": ["GH_TOKEN", "ANTHROPIC_API_KEY"]
}
```

For each listed name that is set (and non-empty) in `cast`'s own environment,
`cast` emits `docker run -e NAME` in the valueless form. Docker reads the value
from its own inherited environment, so the secret never appears in `cast`'s
argv and is not visible to other host users via `ps`.

Behaviour:

- Only names are stored. The value is read from the host environment at run
  time and is never written to `cast.json`, hashed into the approval record,
  or logged.
- A name that is unset or empty on the host is silently skipped.
- Names must match `[A-Za-z_][A-Za-z0-9_]*`; anything else is ignored.
- Duplicates collapse, and the emitted arguments are sorted by name.
- Passthrough values take precedence over entries in `cast.env`, because Docker
  applies `--env` after `--env-file`. They do not override `cast`'s own
  infrastructure variables.

### Precedence

`env_passthrough` is a list, and lists **replace** rather than merge across
config files. A project `cast.json` that sets `env_passthrough` overrides the
global list entirely; the global list applies only when the project config is
silent about it. This is intentional: the effective allowlist is always
auditable by reading a single file, and the failure mode is a missing variable
rather than a silently inherited one.

### Approval

`env_passthrough` is part of `Config`, so it is covered by the approval hash.
Adding a name causes `cast` to report the configuration as changed and requires
`cast config allow` before the next run. See [Approval](approval.md).

**Trust boundary.** Approval gates *which names* cross into the container, not
what your shell put in them. If something later overwrites an
already-approved `GH_TOKEN` — an `.envrc`, a sourced script — `cast` will pass
the new value through without prompting. Anyone able to run arbitrary code in
your shell already has host code execution and can exfiltrate data without
involving `cast`; approval here is a control over the channel, not a
sanitiser of its contents.

## Specialized Env Vars

- `CAST_LOG_DIR`: Directory where daily rolling logs are stored.
- `CAST_DATA_DIR`: Directory where `approved_configs.json` and other state are
  stored.
