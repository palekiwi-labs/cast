# MCP

---

## Command timeout issue

- spawned commands must be killed when the client disconnects, currently they are left dangling.
- there must be a global timeout set on the server to kill the commands if exceeded

report: `.mem/feat-mcp/trace/1778485547-6060736/mcp-timeout-issue.md`

## Logging improvements

- stdout: record execution time
- stdout: record errors
