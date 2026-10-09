# Nix Daemon

The containerized Nix daemon allows sandboxes to perform Nix operations
securely.

## Lifecycle

`cast` manages the daemon automatically:

- When you run an agent, `cast` checks the configured generation container,
  `cast-nix-daemon-<nix_version>` by default.
- If it is not running, Cast builds its daemon image from
  `nixos/nix:<nix_version>` and starts the effective
  `localhost/cast-nix-daemon-<nix_version>:<cast_version>` image.
- You can manually control it using `cast nix-daemon {start|stop|build}`.

## Shared Store

The core of the integration is the shared `/nix` volume.

- **Daemon**: Mounts the volume as `rw`.
- **Agents**: Mount the same volume as `ro`.
- **Protocol**: Agents communicate with the daemon by setting
  `NIX_REMOTE=daemon` and connecting to the Unix socket at
  `/nix/var/nix/daemon-socket/socket`.

The configured daemon container and volume names are base names. Cast appends
`-<nix_version>` to both default and custom bases. Changing `nix_version`
therefore selects a separate container and persistent store; multiple versions
can coexist on one Docker engine. Cast leaves old unsuffixed resources and
stores from other versions untouched. It does not migrate or automatically
delete their data.

## Configuration

Set an exact `nix_version` in `cast.json`. You can also configure additional
substituters and trusted keys:

- `nix_extra_substituters`
- `nix_extra_trusted_public_keys`

For implementation, see [crates/cast/src/nix_daemon/](../../src/nix_daemon/).
