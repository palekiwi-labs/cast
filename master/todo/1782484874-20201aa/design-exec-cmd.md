---
priority: normal
status: open
title: Design Exec Cmd
tags:
  - idea
---

## Context

Design a command that would allow the user to start a container
and execute an arbitrary command, such as `/bin/bash`, or script of their choice.

Use cases:

- a user wants to explore the environment of the container without starting an
  agent with `cast run <agent>`
- a user wants to debug an issue inside the container
- a user wants to run a particular script headlessly, such as a loop which executes
  an agent harness repeatedly in sequencec until a condition is met without having
  to restart the container

## Proposed API

`cast exec [--headless] [--name] <cmd>`

- `--headless`: optioally run the container headlessly
- `--name`: set the container name
- `cmd`: the command to execute
