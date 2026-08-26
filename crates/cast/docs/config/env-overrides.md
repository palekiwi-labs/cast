# Environment Overrides

Every configuration field in `cast` can be overridden by environment variables.

## Naming Convention

- Prefix: `CAST_`
- Case: ALL_CAPS
- Nesting: Double underscore `__`

## Examples

- `memory` becomes `CAST_MEMORY`.
- `cpus` becomes `CAST_CPUS`.
- `mcp.port` becomes `CAST_MCP__PORT`.
- `mcp.hostname` becomes `CAST_MCP__HOSTNAME`.
- `global_shell` becomes `CAST_GLOBAL_SHELL`.
- `project_shell` becomes `CAST_PROJECT_SHELL`.
- `use_global_flake` becomes `CAST_USE_GLOBAL_FLAKE`.
- `use_project_flake` becomes `CAST_USE_PROJECT_FLAKE`.
- `extra_data_volumes.cargo.target` becomes
  `CAST_EXTRA_DATA_VOLUMES__CARGO__TARGET`.

The removed `CAST_USE_FLAKE` and `CAST_USE_FLAKE_PATH` overrides have no effect.

## Passing Host Variables Into the Sandbox (`env_passthrough` and `extra_env_passthrough`)

The overrides above configure `cast` itself. To make a variable from your host
shell visible *inside* the container — such as provider API keys or a token
you do not want written to `cast.env` on disk — list its **name** in
`env_passthrough` or `extra_env_passthrough`:

In your global configuration (`~/.config/cast/cast.json`):

```json
{
  "env_passthrough": ["ANTHROPIC_API_KEY", "OPENAI_API_KEY"]
}
```

In your project configuration (`./cast.json`):

```json
{
  "extra_env_passthrough": ["GH_TOKEN"]
}
```

`cast` forwards no user-configurable host variables by default. Adding provider
API keys to global `env_passthrough` is the standard way to provide credentials
to agents inside sandboxes.

For each listed name in the effective set (`env_passthrough` ++
`extra_env_passthrough`) that is set (and non-empty) in `cast`'s host
environment, `cast` emits `docker run -e NAME` in the valueless form. Docker
reads the value from its own inherited environment, so the secret never appears
in `cast`'s argv and is not visible to other host users via `ps`.

Behaviour:

- Only names are stored. The value is read from the host environment at run
  time and is never written to `cast.json`, hashed into the approval record,
  or logged.
- A name that is unset or empty on the host is silently skipped.
- Names must match `[A-Za-z_][A-Za-z0-9_]*`; anything else is ignored.
- `PATH`, `HOME`, and `NIX_REMOTE` are reserved and always dropped, with a
  warning on stderr: they come from the image or the container user, and a
  host value for them breaks the sandbox.
- Duplicates across and within lists collapse, and the emitted arguments are
  sorted by name.
- Passthrough values take precedence over entries in `cast.env` (Docker
  applies `--env` after `--env-file`) and over the image's own `ENV` defaults.
  They do not override the variables `cast` sets itself (`USER`, `TERM`,
  `CAST_MCP_URL`, ...), which are emitted later in argv.

### Precedence and Replacement

`env_passthrough` and `extra_env_passthrough` are lists, and lists **replace**
rather than merge across config files.

- A project `cast.json` that sets `env_passthrough` replaces the global base
  list entirely.
- A project `cast.json` that sets `extra_env_passthrough` replaces the global
  extra list entirely.
- A `cast.local.json` value replaces the same key from project and global
  configuration.
- The two keys replace independently: a project that sets only
  `extra_env_passthrough` leaves the global `env_passthrough` intact.

Note that `extra_env_passthrough` is additive to `env_passthrough` within the
effective configuration, but it does *not* union global extra entries across
projects. Each key replaces wholesale at the file level. This is intentional:
the failure mode is a missing variable rather than a silently inherited one, and
the effective allowlist remains fully auditable.

The allowlists are config fields, so `CAST_ENV_PASSTHROUGH` and
`CAST_EXTRA_ENV_PASSTHROUGH` in `cast`'s own environment outrank all
configuration files. Being lists, they require the bracketed form:

```sh
export CAST_ENV_PASSTHROUGH='[ANTHROPIC_API_KEY, OPENAI_API_KEY]'
export CAST_EXTRA_ENV_PASSTHROUGH='[GH_TOKEN, NPM_TOKEN]'
```

An unbracketed value (e.g. `CAST_ENV_PASSTHROUGH=GH_TOKEN`) fails to parse and
causes every `cast` invocation to error until the variable is fixed or unset.

Auditing the effective configuration means reading the files *and* checking the
environment — use `cast config show`, which prints the merged result (`cast config
diff` prints only changes against the approved snapshot, so it is silent once the
config is approved).

### Approval

Both `env_passthrough` and `extra_env_passthrough` are part of `Config`, so they
are covered by the approval hash. Adding or changing a name in either field
causes `cast` to report the configuration as changed and requires
`cast config allow` before the next run. See [Approval](approval.md).

**Trust boundary.** Config allowlists are the sole channel for forwarding
variables from cast's own environment into the sandbox (`cast.env` files are the
separate, file-based channel). Approval gates *which names* cross into the
container, not what your shell put in them. If something later overwrites an
already-approved variable (such as `GH_TOKEN` modified by an `.envrc` or a
sourced script), `cast` will pass the new value through without prompting.
Anyone able to run arbitrary code in your shell already has host code execution
and can exfiltrate data without involving `cast`; approval here is a control
over the channel, not a sanitiser of its contents.

## Specialized Env Vars

- `CAST_LOG_DIR`: Directory where daily rolling logs are stored.
- `CAST_DATA_DIR`: Directory where `approved_configs.json` and other state are
  stored.
