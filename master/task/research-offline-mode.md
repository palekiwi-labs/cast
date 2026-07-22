---
status: open
title: research offline mode
priority: normal
---
## Context

some users have reported inability to effectively launch to `cast`
in situations where network access is unstable or has low-bandwidth
such as mobile or congested networks and poor quality hot-spots.

As `cast` heavily relies on `nix`, users frequently face situations
where `nix` makes network calls and starts downloading large amounts
of data, e.g. to start devshells.

## Scope

1) Phase 1: Research `cast` source code to indentify all the instances where `cast` user `nix`.
Map out flows and list reasons and conditions under which `nix` would attemt to make
networks calls and download data.

2) Phase 2: Analyze the feasibility of implementing an optional flag-enabled "offline" mode
for `cast` in which `nix` initiated network calls could be reduced or eliminated.
