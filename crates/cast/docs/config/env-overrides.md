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
- `PATH`, `HOME`, and `NIX_REMOTE` are reserved and always dropped, with a
  warning on stderr: they come from the image or the container user, and a
  host value for them breaks the sandbox.
- Duplicates collapse, and the emitted arguments are sorted by name.
- Passthrough values take precedence over entries in `cast.env` (Docker
  applies `--env` after `--env-file`) and over the image's own `ENV` defaults.
  They do not override the variables `cast` sets itself (`USER`, `TERM`,
  `CAST_MCP_URL`, ...), which are emitted later in argv.

### Precedence

`env_passthrough` is a list, and lists **replace** rather than merge across
config files. A project `cast.json` that sets `env_passthrough` overrides the
global list entirely; the global list applies only when the project config is
silent about it. This is intentional: the failure mode is a missing variable
rather than a silently inherited one.

The allowlist is itself a config field, so `CAST_ENV_PASSTHROUGH` in `cast`'s
own environment outranks both `cast.json` files. Being a list, it needs the
bracketed form — `export CAST_ENV_PASSTHROUGH='[GH_TOKEN, NPM_TOKEN]'`; an
unbracketed value fails to parse and every `cast` invocation errors until
the variable is fixed or unset. Auditing the effective list therefore means
reading the project file *and* checking the environment — use
`cast config show`, which prints the merged result (`cast config diff`
prints only changes against the approved snapshot, so it is silent once the
config is approved). An allowlist extended this way (for example by a
`direnv` `.envrc`) still changes the config hash and still requires
`cast config allow`.

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
