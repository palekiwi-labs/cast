---
status: open
---
# Print Better Config Messages

currently on a new project users get:

```
Run `cast config diff` to see what changed, then `cast config allow` to approve.
❯ cast config diff
No approved config for this workspace.
```

If there are no approved configs for this workspace, then we should tell them immediately in the first message.
