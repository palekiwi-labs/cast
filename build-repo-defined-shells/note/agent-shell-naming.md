---
title: Agent Shell Naming
status: closed
---

I wonder if we should rename the "global shell" to "agent shell". After all,
this is a shell that in practice defines the agent - as opposed the project
shell that defines the project. On top of that, we may want to look ahead and
consider the implications of the upcoming pivot: /home/pl/code/palekiwi-labs/cast/.cue/master/task/herdr-service-pivot.md

The pivot would mean that there would indeed for the first time exist a package
that is trully global: the multiplexer that is intended to run as PID 1. I think
this is an important consideration to address.
