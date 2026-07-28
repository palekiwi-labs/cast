---
refs: .cue/cast-agent-mvp/doc/cast-agent-architecture.md
---
# opencode headless `run` contract (JSON-lines)

The interface `cast-agent` drives for the opencode MVP:
`opencode run --format json`. Injected from the `cast-agent-design` context in
`palekiwi/palekiwi`.

Source: `packages/opencode/src/cli/cmd/run.ts`
(`/home/pl/code/anomalyco/opencode`). Verified directly against source.

## Command

```
opencode run [message..]
```

- Entrypoint / command definition: `run.ts:126` (`RunCommand`, `run [message..]`).
- The positional `message` (plus anything after `--`) becomes the prompt
  (`run.ts:288-290`). Prompt may also be piped via stdin.

## Flags that matter for cast-agent

Verified at `run.ts:135-262`:

- `--format <default|json>` (`run.ts:174-179`) - **`json` = raw JSON-lines event
  stream**. Default is human-formatted. This is the flag cast-agent depends on.
- `--model, -m <provider/model>` (`run.ts:165`).
- `--agent <name>` (`run.ts:170`) - selects the agent persona/config. This is the
  hook for injecting a synthesized config (system prompt + permissions), since
  there is no ad-hoc `--system-prompt` flag.
- `--dir <path>` (`run.ts:204`) - working directory.
- `--variant <effort>` (`run.ts:212`) - provider reasoning effort (high/max/etc).
- `--thinking` (`run.ts:216`) - show reasoning blocks.
- `--session, -s <id>` (`run.ts:152`), `--continue, -c` (`run.ts:147`),
  `--fork` (`run.ts:157`) - session resume / fork controls.
- `--attach <url>` (`run.ts:190`) - connect to a running opencode server instead
  of spawning in-process. (Deferred for MVP: not portable across harnesses, and
  parent-exit kills the cast container.)
- `--auto` / `--yolo` / `--dangerously-skip-permissions` (`run.ts:242-256`) -
  auto-approve permissions not explicitly denied.

### Corrections to first-pass notes

- There is **NO `--print` / `-p` print flag**. `-p` is `--password` (basic-auth
  password for `--attach`, `run.ts:194-198`). JSON output is strictly
  `--format json`.
- There is **no `--system-prompt` flag** and **no ad-hoc `--allow`/`--deny`
  permission flags**. System prompt comes from the message/stdin; permissions and
  tool access come from the resolved agent config (`--agent`).

## JSON output: shape and events

JSON mode emits **JSON-lines** (one JSON object per line). Emitter at
`run.ts:678-691`:

```js
JSON.stringify({ type, timestamp: Date.now(), sessionID, ...data }) + EOL
```

Every event carries `type`, `timestamp`, `sessionID`, plus event-specific data.

Event types emitted in JSON mode (from the event loop `run.ts:697-817`):

- `tool_use` (`run.ts:719-720`) - emitted when a tool part reaches `completed`
  or `error`. Carries the full tool `part` object (tool name, args/input,
  state/result). Rich enough to render our own progress/feedback UI.
- `step_start` (`run.ts:740-741`).
- `step_finish` (`run.ts:744-745`).
- `text` (`run.ts:748-749`) - the assistant's output text parts. The final
  assistant text is what cast-agent returns as the delegation result.
- `reasoning` (`run.ts:761-762`) - reasoning/thinking parts (only when
  `--thinking`).
- `error` (`run.ts:776-784`) - carries `session.error` payload.

The run loop terminates when `session.status` becomes `idle`
(`run.ts:788-793`). cast-agent should treat the idle transition (or process
exit) as end-of-delegation.

## Permission behavior in headless mode

At `run.ts:796-816`:

- On `permission.asked`:
  - if `--auto` (or yolo / skip) is set -> auto-reply `once` (approve).
  - otherwise -> **auto-REJECT** every request.
- There is **no interactive middle ground** over the JSON stream in a plain
  `run` invocation. Note (verified separately): opencode's global default is
  `"*": "allow"`, and an `allow` rule short-circuits before `permission.asked`,
  so a default headless run is functional without `--auto`; the auto-reject only
  fires for tools resolved to `"ask"`. Permission escalation is a priority-3
  concern, not part of the MVP.

## Implications for the cast-agent MVP

1. cast-agent shells out to `opencode run --format json <prompt>` as a fresh
   child process (isolation; portable; survives the "no --attach in other
   harnesses" constraint).
2. Consume the JSON-lines stream; passthrough-log it; return the final `text`
   part as the delegation result (this is `extract_result` for the opencode
   adapter).
3. System-prompt / permission injection via a synthesized `--agent` config is a
   later-priority concern (there are no ad-hoc prompt/permission flags); the MVP
   relies on opencode's own resolved config.
