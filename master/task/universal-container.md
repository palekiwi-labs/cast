---
priority: high
title: universal container
status: open
---
## Context

`cast` has been designed to support multiple agents however our implementation
so far has assumed the user will only want to use one agent type per container.
For that reason we have designed an API that requires the user to specify which
agent they want to run: `cast run opencode` or `cast run pi`. We also ship
dockerfiles which build images with only one harness installed in it. As a
consequence, our container naming system also assumes only one agent will be
used and the container is named after that agent and its version.

## Problem

However, the reality is that I use all three agents, some to a different degree
and for different purposes but I am currently unable to use two different agents
in the same container. As a human I may need that for testing or development
purposes but more importantly my AI Agents which run in harness A may benefit
from calling an agent in harness B as a sub-process or a subagent.

## Scope

Investigate whether we can extend or adapt our code to allow running a hybrid
docker image that combines all of our supported agents that is:
- it sets up binaries for all of the agents in docker image
- it mounts all the necessary directories that agent harnesses require to run
and stor their configuration and state
- it is properly named after the versions of all the agents so that when a user
wants to update or change the version of any of the agent, then a new image gets
built
