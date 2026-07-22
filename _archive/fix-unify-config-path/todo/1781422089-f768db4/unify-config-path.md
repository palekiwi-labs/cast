---
title: Unify config path
priority: 0
status: open
---

find all locations such as this:
`/home/pl/code/palekiwi-labs/cast/crates/cast/src/config/loader.rs L59`

where we use:

```
dirs::config_dir()
```

The issue is that we want the config dir to always point to `$HOME/.config`,
whether it is for `cast` or for any agent harness (e.g. `opencode`).

`dirs::config_dir()` returns a different dir on macos that doesn't map
1:1 to the path inside a Linux container. Let's resolve the home dir
instead and append `.config`

