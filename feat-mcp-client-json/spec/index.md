# cast-mcp-client JSON output

## Context

Currently all `cast-mcp-client` commands return unstructured text. This tool is intended mostly
for programmatic use in scriting as well as direct use by AI agents in bash.

A CLI that acts as an MCP client provides a number of benefits with composability (piping) being
the major one. We need to redesign the output of our commands to prioritize composable and
ergonomic use for scriting and programmatic tool calling.

## Purpose

Ensure all our client commands return structured JSON as output.
