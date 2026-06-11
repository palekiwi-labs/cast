# Nix Daemon

The containerized Nix daemon allows sandboxes to perform Nix operations securely.

## Lifecycle

`cast` manages the daemon automatically:
- When you run an agent, `cast` checks if the `cast-nix-daemon` container is running.
- If not, it starts it using the `localhost/cast-nix-daemon` image.
- You can manually control it using `cast nix-daemon {start|stop|build}`.

## Shared Store

The core of the integration is the shared `/nix` volume.
- **Daemon**: Mounts the volume as `rw`.
- **Agents**: Mount the same volume as `ro`.
- **Protocol**: Agents communicate with the daemon by setting `NIX_REMOTE=daemon` and connecting to the Unix socket at `/nix/var/nix/daemon-socket/socket`.

## Configuration

You can configure additional substituters and trusted keys in `cast.json`:
- `nix_extra_substituters`
- `nix_extra_trusted_public_keys`

For implementation, see [crates/cast/src/nix_daemon/](../../src/nix_daemon/).
