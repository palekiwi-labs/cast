---
title: new crate cast-agent
priority: normal
status: open
---
## Background

`cast` already has a concept of an `Agent` defined by a trait and representing
its extensible support of agent harness programs. We currently support
`claudecode`, `opencode` and `pi`, all of which support running in both a TUI
and headless modes. Unlike both `opencode` and `claudecode`, the `pi` harness
does not come with built-in idea of "subagents" also known as "tasks" which
allow primary (or main) agents to spawn child agents in subprocesses. I use
subagents a lot but I don't want to build and maintain a subagent
implementantation for `pi` or the next n+1 agent that becomes popluar. At the
same time, I would like to be able to mix and match freely my agents: perhaps
start my main agent session in `opencode` but allow the main agent to spawn a
subagent process that runs in `pi`. Therefore rather than build a harness
extension in Typescript (which is required by all popular harnesses), I prefer
to build a CLI tool in Rust with an API that I control that does all the heavy
lifting and hand it to our agents, or wrap it very thinly as an agent tool that
harnesses expose natively to agents. On top of that centralization and
abstraction from 3rd party agents harness tools, I want to achive greater
control and observability over these sub-agents processes.

## Idea

Enter `cast-agent` (name subject to change) - a new crate in the `cast` project
whose responsibility is spawning and managing agents. Not just sub-agents, which
is a relative term, but having the ability to take full advantage of the
parameters exposed by agent harnesses to freely define and run agents as needed
for the job.

My proposed tool to leverage under the hood is a terminal multiplexer, most
likely `tmux` because I'm the most familiar with it and it can be a good start,
but most importantly we should make it replacable (via trait?), not welded in forever.
`tmux` allows us the humans but also the agents themselves better observability
and control. We can name, group, list, attach, detach, kill, send keys and
capture panes which is useful for fingerprinting of frames and detection if an
agent has gotten stuck or started looping.

`cast-agent` should also be able to parse, process and return the output of the
agents. When run headlessly, agents return a stream of `json`. Since we wrap our
agents, we retain full control of what we take as input, what we do with it and
what we return - either standardized, or depending on the caller's needs.

I imagine `cast-agent` to be used in practice for these two major applications:
1. Spawning and managing subagents which will replace the built-in `subagent` or
`task`tool in agent harnesses that already support it

2. Allow a relay-style "hand-off" and continuation between primary agents: when
   one primary agents reaches a threshold of context saturation (e.g. 100k
   tokens), we force it to wrap up current work (e.g. write to cue log) and
   spawn a new primary agent that takes over the job with a fresh context. Of
   course, we would need to perform clean-up to ensure that old agent is isn't
   left dangling after the hand-off.

## Context

With `cast` and with `cue` (refer to the `cue` skill for now), we have the
tools to treat our agents as disposable because `cue` strives to preserve
important context and memory in file-system artifacts which is very imporantant
for any agent to agent interaction, but most imporatntly primary agent hand-off.


## Extra considerations

We need to analyze this idea thoroughly, define its scope, refine its features
and propose an API.

Apart from that here are some loose connecting ideas that I have:
- currently we create fully separate enviroments per agent: we can create an MVP
  of this feature assuming we stick to one agent harness only which should for
  now be `opencode` but eventually I think we should allow users to run any
  mix of agents inside the same container
- we should be able to associate spawned sub-agents with the spawining agent as
  in maintaing a logical connection and continuity of session with a concept of nesting
- we may want to consider how our idea of spawning agents can relate to
  leveraging git worktrees to separate the working context of parallel agents
- we need to think how we can support spawing parallel agents: how do you spawn
  agents in parallel right now with you built-in tools? how can we achieve this
  capability with `cast-agent`? Imagine a user asks you to perform multiple
  tasks in parallel - how could `cast-agent` support such workflows?
