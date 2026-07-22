---
title: Universal Container
status: open
priority: high
---
## Goal

Allow any agent binary to be called as a subprocess by another agent running inside the same container. Enable this via a new `universal_container` config flag that causes all `cast run <agent>` sessions to use a single hybrid Docker image containing every agent pinned in `agent_versions`. The existing CLI API is unchanged.

## Background

See original brief in master/task/1783779669-0dbafc9/universal-container.md and implementation plan for full design rationale.

## Acceptance Criteria

| # | Criterion |
|---|-----------|
| 1 | `universal_container: bool` recognised in cast.json; default false |
| 2 | When false, all existing behaviour is identical (no regression) |
| 3 | When true, cast run opencode builds/uses the universal image not the agent-specific one |
| 4 | Universal image contains exactly the agents listed in agent_versions |
| 5 | Universal image tag is a deterministic content hash of the agent-to-version map |
| 6 | cast run agent errors clearly when the agent is not in agent_versions |
| 7 | In universal mode, shared {namespace}-universal-{cache,local} volumes used |
| 8 | In universal mode, all included agents config dirs bind-mounted simultaneously |
| 9 | Volume names respect volumes_namespace from config |
| 10 | prepare_host() called for every included agent in universal mode |
| 11 | cast build agent in universal mode builds the universal image with informative message |
| 12 | All new behaviour covered by unit tests; all existing tests remain green |
