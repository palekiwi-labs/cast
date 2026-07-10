---
title: Link containers with host
priority: normal
status: in-progress
branch: feat/cast-host-name
---

# Link containers with host

I would like to be able to know from inside the container the host name of my
host. It is important for the purpose of collecting diagnostics and grouping the
diagnostics by the host.

One idea that I have is injecting an environment variable into the container
that contains the name of the host.

## Acceptance Criteria

| #   | Criterion (outcome)                                                      | Verify by                              | Evidence |
| --- | ------------------------------------------------------------------------ | -------------------------------------- | -------- |
| 1   | Host name is readable via `CAST_HOST_NAME` env var inside the container  | `cast exec opencode env \| grep`       |          |
| 2   | Injection present in both interactive (`run`) and headless (`exec`) modes | unit test on `build_docker_run_flags`  |          |
| 3   | Failure to resolve hostname does not abort the session                   | unit test on `resolve_run_opts`        |          |
| 4   | All existing tests pass under `nix build`                                | `nix build .#cast.x86_64-linux` check  |          |
| 5   | `CAST_HOST_NAME` is documented                                           | concepts.md updated                    |          |
