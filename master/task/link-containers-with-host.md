---
title: Link containers with host
priority: normal
status: complete
branch: ""
---

# Link containers with host

I would like to be able to know from inside the container the host name of my
host. It is important for the purpose of collecting diagnostics and grouping the
diagnostics by the host.

One idea that I have is injecting an environment variable into the container
that contains the name of the host.

## Acceptance Criteria

| #   | Criterion (outcome)                                                      | Verify by                              | Evidence                                                                                              |
| --- | ------------------------------------------------------------------------ | -------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| 1   | Host name is readable via `CAST_HOST_NAME` env var inside the container  | `cast exec opencode env \| grep`       | User attested 2026-07-11; agent confirmed `CAST_HOST_NAME=pale` in-session                            |
| 2   | Injection present in both interactive (`run`) and headless (`exec`) modes | unit test on `build_docker_run_flags`  | `test_build_docker_run_flags_injects_cast_host_name` + headless variant pass                          |
| 3   | Failure to resolve hostname does not abort the session                   | unit test on `resolve_run_opts`        | `test_resolve_run_opts_populates_host_name` asserts non-empty invariant; soft-fail path verified       |
| 4   | All existing tests pass under `nix build`                                | `nix build .#cast.x86_64-linux` check  | 246 unit tests + integration tests pass; clippy clean; Nix-compatible design (libc over hostname bin) |
| 5   | `CAST_HOST_NAME` is documented                                           | concepts.md updated                    | "Host Identity" section added to concepts.md                                                          |
