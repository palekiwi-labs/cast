---
status: open
priority: normal
---
# Port vs unique identifier in service container naming

## Problem

The service container name formula is
`cast-{basename}-{workspace-port}` (default) and
`cast-{basename}-{workspace-port}-{name}` (named service, via
`cast up --name <n>`).

The number comes from `dev/port.rs::calculate_port`, currently
CRC32 of `"{pwd}:{agent_name}"`, mapped into 32768-65535. The service
model drops the agent from the seed, making it CRC32 of `"{pwd}"`.

That works as a *unique identifier per workspace*, but the number still
means a **port**. If two containers run for the same project (default
service plus `cast up --name heavy`), both derive the same number from
the same `pwd`. As an identifier that is fine only because `--name`
disambiguates the container name — but as a *port* it is a conflict:
two containers cannot publish the same host port.

## Options (not evaluated yet)

1. Include the service name in the port seed:
   CRC32 of `"{pwd}:{name}"`. Keeps generation implicit and
   deterministic; every named service gets its own port.
2. Pass the port explicitly to `cast up` (e.g. `cast up --port 8080`).
   Now that an explicit lifecycle verb exists, explicit assignment is
   natural — generation-by-hash was a workaround for having no place to
   put the decision. Operator leans this direction ("might be a better
   idea").
3. Separate the two roles entirely: derive the container identity from
   the workspace path (hash or path-derived slug) and allocate a port
   only when publishing is actually requested. Most containers may need
   no published port at all.
4. Allocate on demand: no port in the name; publish dynamically and
   report the assigned port via `cast status`.

## Notes

- Option 3/4 depend on a spike observation: do service containers
  actually need published host ports, and for what (agent web UIs such
  as opencode's server)? Do not guess.
- `cast port` currently takes an agent argument; whatever is chosen
  should simplify it to a workspace/service-level query.

## Status

Deferred deliberately — must not block the spike. Revisit when spike
evidence shows whether published ports are needed at all.
