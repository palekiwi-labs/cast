# Project Log

## [b49c527] Slice 1: Setup Content

- **Found:** Created initial documentation for MCP configuration in docs/mcp/configuration.md.
- **Decided:** Using markdown for documentation as it is easily readable by both humans and AI agents.

## [f5a3091] Slice 2: Enable Resources Capability

- **Found:** Enabled the resources capability in the MCP server handler's get_info method.
- **Decided:** Following the TDD cycle to ensure capabilities are correctly reported to clients.

## [807b208] Slice 3: Implement list_resources

- **Found:** Implemented the list_resources MCP method by mapping a registry of embedded documentation entries.
- **Decided:** Using RawResource and AnnotateAble to construct the resource metadata correctly for rmcp 1.6.0.

## [7872149] Slice 4: Implement read_resource

- **Found:** Implemented the read_resource MCP method by looking up URIs in the embedded documentation registry.
- **Decided:** Returning TextResourceContents for documentation entries and handling unknown URIs with InvalidParams error.

