---
priority: critical
status: complete
---
Now that we have started working on v0.2.0, we need to examine our security
policy in nix config for both the daemon and the dev container.

Some observations:

1. We may be misusing `trusted-users`

It is now set to `*`
(`/home/pl/code/palekiwi-labs/cast/crates/cast/src/nix_daemon/config.rs L14`)
most likely in a rushed attempt to fix an issue with the dev container user to
use the daemon via the mounted socket. However, this setting has very serious
security implications. Could we identify what the issue is and possibly fix it
by using `allowed-users` instead?

We need to keep in mind that the daemon container only has a root user, and the
dev container has a user with username and UID/GID that are dynamically resolved
at container start to match the user's host machine.

As as a consequence of tightening `trusted-users`, we should verify that our
method of setting `trusted-substituters` and `trusted-public-keys` via
config.json (e.g. `/home/pl/.config/cast/cast.json`) will be enough to allow
using the cache, such as: `/home/pl/.config/cast/nix/flake.nix L8`

2. Should we revise how we mount `/nix`?

Are we mounting in dev containers as `ro` or `rw`? Should we be mounting the nix
socket as rw only as a separate nested mount?

3. Should we enable `sandbox = true`?
