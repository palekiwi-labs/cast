---
status: complete
priority: high
---
# `--agent <name>` must land in the cast-agent MVP

## Context

The Layer-2 opencode task drop-in in `cue-plugins`
(`src/opencode/task.ts`) is registered under tool id `"task"`. opencode's
`ToolRegistry.describeTask` (`registry.ts:252-265`, applied at
`registry.ts:296`) injects the available subagent list into ANY tool whose
`id === "task"`, including our override. The model therefore sees the correct
filtered list of personas (explore, plan, etc.) and emits a `subagent_type`
argument accordingly.

But our plugin's executor currently SILENTLY DROPS that argument
(`task.ts:41` TODO; spawn at `task.ts:42` does not pass `--agent`). The model
believes it delegated to a read-only `explore` persona; in reality it launched
the default primary agent with full `* : allow` tool access. That is a quiet
permission/scope regression versus the built-in task tool, not just a missing
feature.

## What needs to land in cast-agent

- A `--agent <name>` flag on `cast-agent run` that selects the harness-side
  persona config. For `--harness opencode` this maps 1:1 to `opencode run
  --agent <name>` (the child opencode loads the persona prompt + permission
  profile + model itself -- cast-agent just passes the name through).
- The flag was originally deferred to priority-3 ("practical MVP
  enhancements"). The Layer-2 drop-in's fidelity makes it priority-1: without
  it, the drop-in is not a faithful replacement.

## Acceptance

- `cast-agent run --harness opencode --agent explore --file <prompt>` launches
  opencode with the `explore` persona's prompt/permissions/model.
- Unknown agent name fails fast with a clear error (do not silently fall back
  to default).

## Source / verification

- Layer-2 design task: `palekiwi/palekiwi` workspace,
  `.cue/cast-agent-design/` (the `cast-agent-design` master task).
- Trace of opencode's current treatment of id "task" tools:
  `.cue/cast-agent-design/trace/<ts>-<hash>/opencode-task-id-treatment.md`
  (in the palekiwi/palekiwi workspace).
- Plugin-side counterpart todo: `cue-plugins` repo,
  `.cue/castagent-task-tool/todo/pass-agent-flag-to-cast-agent.md`.