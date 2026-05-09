---
status: todo
tags: [security]
---

# `cast config allow/deny`

In comands that run or execute containers (e.g. `cast run opencode`) require that the config
has been manually approved by the user. After building the config, build its contents hash
and compare it to approved hashes stored somewhere in `~/.local`. If an identical hash
already exists, continue with command executing and start relevant container/process.

If the hash does not exist yet, exit and inform the user that the config has changed and
ask the user to review it and explicitly approve with `cast config allow`.

## Proposed commands:

- `cast config allow`: assembles the config, computes the hash, and saves it to `~/.local`
- `cast config deny`: assembles the config, computes the hash and removes it from stored approved hashes

## Considerations:

Should the absolute project path on the host be included in the hash? That would prevent situations
where a user has changed a setting for one project, i.e. increased memory limits and this setting
configuration would now become approved for all other projects.
