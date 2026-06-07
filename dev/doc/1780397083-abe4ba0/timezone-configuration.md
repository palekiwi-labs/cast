# Timezone Configuration in Cast Dev Containers

This report details how the `cast` CLI manages timezone settings in its dev containers and how users can ensure or override these settings.

## 1. Command Context
While users may refer to starting the environment as `cast start`, the actual command to launch an agent container is:
- `cast run opencode` (alias: `cast run o`)
- `cast run pi` (alias: `cast run p`)

The `cast start` command is not defined for dev containers; `start` is only used for `cast mcp start` and `cast nix-daemon start`.

## 2. Automatic Timezone Synchronization
By default, `cast` synchronizes the container's timezone with the host machine by mounting the host's local time configuration.

- **Implementation**: Every agent container is started with a read-only bind mount:
  - Source: `/etc/localtime` (host)
  - Target: `/etc/localtime` (container)
- **Source Code Reference**: `crates/cast/src/dev/run.rs:178`
  ```rust
  // Timezone.
  run_args.extend([
      "-v".to_string(),
      "/etc/localtime:/etc/localtime:ro".to_string(),
  ]);
  ```

This ensures that the container's standard C library (glibc) and most applications respect the host's system timezone.

## 3. Manual Override via Environment Variables
Some applications require the `TZ` environment variable to be set explicitly. Since `cast` does not currently pass the `TZ` variable from the host automatically, users can set it manually.

### Using `cast.env`
The recommended way to set environment variables is via a `cast.env` file. `cast` automatically looks for these files and passes them to the container via Docker's `--env-file` flag.

- **Project-Local**: Create a `cast.env` file in your project root.
- **Global**: Create a `~/.config/cast/cast.env` file.

**Example `cast.env`:**
```bash
TZ=Europe/London
```

### CLI Limitations
Users **cannot** pass Docker flags like `-e TZ=...` directly to `cast run`. Any arguments following the `--` separator in `cast run` are passed to the agent binary *inside* the container, not to the `docker run` command itself.

## 4. Verification
To verify the current timezone inside a running container, use the `date` command:
```bash
cast run o -- date
```
If configured correctly, this will return the time in the expected timezone.
