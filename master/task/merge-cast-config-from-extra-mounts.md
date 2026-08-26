---
status: open
title: Merge Cast Config From Extra Mounts
priority: normal
kind: research
---
## Context

Since we have started using "coordination workspaces" such as this one, which
rely on `extra_data_volumes` in `cast.json` to mount other projects we have
exposed these projects' protected files in directories that would otherwise have
been shadowed by thes `forbidden_paths` setting in their project `cast.json`.

## Purpose

Investigate the feasibility of `cast` automatically scanning through all entries
in `extra_data_volumes` for `cast.json` and merging in critical settings, such
as `forbidden_paths` into cast config in such a way that the `cast` container
applies them even if run form another workspace.
