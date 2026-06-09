---
status: open
---

# Redesign the client API

current client API `list`, `call`, `describe` tries to solve a problem
that is should not: "how to identify server + tool combination"? and
offers an arbitrary convention ("server/tool-name") that breaks apart fast.

This is a result of poor API design.

I propose the following redesign:

```bash
# remove `--server` flag and replace with positional argument
# return nested data keyed by server name with tools retaining original names
cast-mcp-client list [servers] # returns { "server_a": [], "server_b: []" } or {} if none

# accept separate required positional args for server and tool
cast-mcp-client describe <server> <tool>

# accept separate required positional args for server and tool
cast-mcp-client call <server> <tool> --data [json-data]
```


