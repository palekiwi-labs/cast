# Command: mcp

---

## Context

Implement a built-in MCP (Model Context Protocol) server that a user can start with
a new subcommand `cast mcp start`. The main purpose of this command is to allow users
to configure specific whitelisted commands that sandboxed agents runnin inside the containers
could safely execute on the host. This feature must be throughly examined from the security
perspective as it could possibly introduce an attack vector or an exploit for the agents.

We can begin by exposing only one MCP tool: `exec` or something similarly named that allows
clients to specify what command they want to run, with what positional arguments, and with what flags.

So conceptually, it would be something like: 
`cast-mcp exec cmd="rspec" args=[spec/models/car_spec.rb] flags=[]`

NOTE: I am not yet sure how I we would want to specify the flags in a way that could easily be parsed and validated.

On the host side, the user of `cast` will need to first configure the MCP tool via the config.
We can place the MCP config under a separate key `mcp`.

### Example config excerpt

```json
{
  ...
  "mcp": {
    "port": 32123,
    "hostname": "0.0.0.0"
    "commands": {
      "rspec": {
        "client_cmds": [
          ["rspec"],
          ["bin/rspec"], 
          ["bundle", "exec", "rspec"]
        ],
        "host_cmd": ["docker", "compose", "exec", "test", "bundle", "exec", "rspec", "{flags}" ,"{args}"]
        "args": {
          "pattern": "^spec/.*_spec\\.rb$"
        },
        "flags": {
          "--format": ["json", "progress"]
        }
      }
    }
  }
}
```

`port` and `hostname` will be used for the MCP (HTTP) server to run on the specified port and hostname on the host.

This configuration allows clients to execute commands such as:
- `rspec spec/service/my_service.rb spec/model/my_model.rb`
- `bin/rspec --format json`
- `bundle exec rspec --format progress spec/service/my_service.rb`

Clients request command execution with `client_cmds` which then map to what the MCP server executes
on the host with a `host_cmd`. We need a flexible syntax to map the flags and args, so I wonder if
matching on placeholders like `{args}` and `{flags}` could be good enough.

## Security Considerations

`mcp start` command must use `ApproveConfig` to ensure the server is not accidentally run to allow
dangerous commands or that AI agents do no tamper with the config and affect runtime.
The user must always approve the config with `cast config allow` for `mcp start` command.

## References

For `rmcp` reference check: `/home/pl/code/palekiwi-labs/dev-notes/cast/rmcp/`
