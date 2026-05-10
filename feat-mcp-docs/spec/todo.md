# Todo: MCP Documentation Serving

## Slices (TDD & Commit Milestones)

- [x] **Slice 1: Setup Content** <!-- priority: high -->
  - Create `docs/mcp/configuration.md` with schema documentation.
  - *Commit*: `docs: add initial MCP configuration documentation`

- [x] **Slice 2: Enable Resources Capability (TDD Cycle)** <!-- priority: high -->
  - **RED**: Write a test for `McpHandler::get_info` verifying `capabilities.resources` is enabled.
  - **GREEN**: Update `ServerCapabilities` builder to enable resources.
  - *Commit*: `feat: enable resources capability in MCP server handler`

- [x] **Slice 3: Implement `list_resources` (TDD Cycle)** <!-- priority: high -->
  - **RED**: Write a test for `McpHandler::list_resources` verifying it returns the `cast://docs/mcp/configuration` resource.
  - **GREEN**: Define `EmbeddedDoc` struct, create `DOCS` registry with `include_str!`, and implement `list_resources` logic.
  - *Commit*: `feat: implement resource listing for embedded documentation`

- [x] **Slice 4: Implement `read_resource` (TDD Cycle)** <!-- priority: high -->
  - **RED**: Write a test for `McpHandler::read_resource` verifying a successful read (matches embedded text) and a not-found error.
  - **GREEN**: Implement `read_resource` to lookup URI in `DOCS` registry and return `TextResourceContents`.
  - *Commit*: `feat: implement reading documentation resources`

- [x] **Slice 5: Manual Validation** <!-- priority: medium -->
  - Build the binary (`cargo build`).
  - Run the `cast mcp start` server and manually verify the resources via an MCP inspector/client.

