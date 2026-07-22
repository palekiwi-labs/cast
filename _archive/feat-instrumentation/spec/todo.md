# Todo: Instrumentation Integration

## Phase 1: Docker Child Process Refactor (Completed)

- [x] Add `libc` to `[dependencies]` in `Cargo.toml`
- [x] Implement `DockerClient::interactive_command` with child supervision
- [x] Implement `SignalGuard` RAII for robust signal restoration
- [x] Update `src/dev/run.rs`, `src/dev/shell.rs`, `src/nix_daemon/daemon.rs`
- [x] Implement `exit_with_status` with `128 + signal` convention
- [x] All existing tests pass

## Phase 2: Tracing Integration (Completed)

- [x] Add dependencies to `Cargo.toml`:
  - `tracing`
  - `tracing-subscriber` (with `fmt` and `env-filter` features)
  - `tracing-appender`

- [x] Create `src/logging.rs`:
  - `generate_invocation_id() -> String`
    - Uses `std::collections::hash_map::RandomState` + `BuildHasher` for OS-seeded entropy
    - Returns 8 hex chars: `format!("{:08x}", random_u64 as u32)`
    - No new dependencies required
  - `init_file_logger() -> Result<()>`
    - Resolves log dir: `dirs::data_dir() / "cast/logs"`
    - Creates log dir with `create_dir_all`
    - Builds `tracing_appender::rolling::daily(log_dir, "cast")` appender
    - Parses `CAST_LOG` env var for level (default `INFO`)
    - Initializes `tracing_subscriber::fmt().with_writer(appender).with_ansi(false).with_max_level(level).init()`

- [x] Update `src/commands/cli.rs` `run()`:
  - Call `init_file_logger()` immediately after `load_config()`, before any dispatch
  - Generate invocation ID: `let invocation_id = generate_invocation_id()`
  - Enter root span for entire `run()` body:
    `let root = info_span!("cast", id = %invocation_id, pid = std::process::id())`
    `let _root_guard = root.enter()`
  - Root span guard must remain in scope until `run()` returns

- [x] Instrument `src/dev/run.rs`:
  - Resolve `container_name` before the session span (it is already computed — move it up if needed)
  - Enter `info_span!("agent_session", agent, container, port)` wrapping the main body
  - `info!` event: session started (`image_tag`, `port`, `container_name`)
  - `info!` event: session ended (`exit_code`, `duration_secs`) — compute elapsed with `Instant`
  - `debug!` events: port resolved, container name derived

- [x] Instrument `src/docker/client.rs`:
  - `debug!` in `run_command`: log full args before executing
  - `debug!` in `stream_command`: log full args before executing
  - `debug!` in `interactive_command`: log full args before spawning

- [x] Instrument `src/nix_daemon/daemon.rs`:
  - `info!`: daemon already running (`container_name`)
  - `info!`: starting daemon (`container_name`, `image_tag`)
  - `info!`: daemon started successfully

- [x] Instrument image resolution (locate the image existence check in `src/dev/`):
  - `info!`: image exists, skipping build (`tag`)
  - `info!`: image not found, building (`tag`)

- [x] Instrument `src/config/loader.rs`:
  - `info!`: config loaded (`memory`, `cpus`, `pids_limit`, `network`)

- [x] Run `cargo test` -- all tests pass

- [x] Manual verification:
  - Run `cast run opencode`, confirm log file created at `~/.local/share/cast/logs/cast.YYYY-MM-DD`
  - Confirm root span fields present on every line: `id=XXXXXXXX pid=NNNNN`
  - Confirm INFO entries present: session start/end, image decision, nix daemon status
  - Run two concurrent `cast` invocations, confirm lines interleave cleanly with distinct `id` values
  - Run with `CAST_LOG=debug`, confirm DEBUG entries present (full docker args, etc.)
  - Confirm no ANSI escape codes in log file
