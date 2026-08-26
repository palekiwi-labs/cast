# herdr as a Sandbox Container Service — Implementation Guide

**Purpose:** Run a sandbox Docker container as a long-lived *herdr service* that hosts coding-agent
harness sessions (opencode, pi, etc.). The container stays up indefinitely; agents are started on
demand by `docker exec`-ing into it. The human stays in tmux on the host and attaches to the herdr
UI only when they want to watch.

**Audience:** Agents helping implement/set up this workflow. Everything below was verified against
the herdr source (`herdrdev/herdr`, `src/main.rs`, `src/cli/spec.rs`, `src/server/headless.rs`,
`src/session.rs`, `src/detect/manifests/`), not guessed. Where something is a design choice rather
than a hard constraint, it says so.

---

## 1. Mental model (read this first)

herdr is **not** a wrapper you launch an agent through. It is a **terminal multiplexer with a
server/client split**, and the distinguishing feature is that it *recognizes coding agents* inside
panes and tracks their lifecycle state (`idle`, `working`, `blocked`, `done`, `unknown`).

- **The server** is the long-lived process that owns the panes and agent PTYs. Runs headless via
  `herdr server`. Foreground event loop — does **not** fork or daemonize. Keeps running after all
  clients disconnect.
- **The client** is the TUI (bare `herdr`) *and* every `herdr <subcommand>` CLI call. All of these
  are thin socket clients that talk to the running server over a unix socket.

This shape is exactly a **service**: one durable server process, many transient client calls. That
is why "container = herdr service, agents = things you exec in afterward" is the right architecture.

herdr and tmux are **peers**, not layered. tmux runs on the host; herdr runs in the container. They
never nest, so there is no prefix-key or escape collision to solve.

---

## 2. Hard constraints discovered in the source (do not violate)

1. **The `herdr` binary takes no "run this program" argument.** Launch-time args are only
   `--session <name>`, `--no-session`, `--remote`, `--remote-keybindings`, `--handoff`, and info
   flags (`--version`, `--skill`, `--default-config`, `--help`). There is **no** flag to boot the
   container "with opencode." You start herdr, then start the agent as a separate step.

2. **`workspace create` makes an *empty* workspace.** Its flags are `--cwd`, `--label`, `--env`,
   `--focus`/`--no-focus`. There is **no** `--command`. The workspace's root pane is just a shell
   prompt; you start the agent into that pane separately.

3. **`agent start` needs a pre-existing pane at an interactive shell prompt.** It never creates
   layout. It blocks until the agent is detected ready (default 30000ms, `--timeout MS`, max
   300000). The agent name must match `[a-z][a-z0-9_-]{0,31}` and be unique among live agents.

4. **`HERDR_ENV=1` is a required gate for control commands.** The agent skill's control commands
   first check `test "${HERDR_ENV:-}" = 1` and refuse otherwise. Set it in the image/exec env.

5. **herdr refuses to run nested inside itself** when `HERDR_ENV=1` (unless
   `experimental.allow_nested = true`). Not a problem here — host tmux and container herdr are not
   nested — but do not try to launch `herdr` from inside a herdr pane.

6. **PID 1 must be `herdr server`, not bare `herdr`.** Bare `herdr` is the *client*: it wants a real
   terminal and stdin and tries to attach a UI. `herdr server` is the headless server meant to run
   without a terminal. Using bare `herdr` as the container command is wrong.

7. **The server refuses to double-start.** A second `herdr server` against a live socket exits with
   `AddrInUse` → "herdr server is already running." Restart must *replace* a dead server, never run
   two. The server cleans up stale sockets on startup, so a crashed→restarted container recovers.

8. **The server always does session persistence** (`no_session = false` is hardcoded in the server
   path). Session/workspace state is written under herdr's config/state home. Persist that dir on a
   volume so restarts restore workspaces/agents instead of coming up blank.

9. **Graceful shutdown is on SIGINT.** The server registers a Ctrl+C/SIGINT handler and shuts down
   cleanly. `docker stop` sends SIGTERM → use a real init (tini / `--init`) to forward it so the
   graceful path runs.

---

## 3. Reference facts (verified)

- Session name defaults to `default`. Override with `--session <name>` or env `HERDR_SESSION`
  (source: `SESSION_ENV_VAR = "HERDR_SESSION"`, `DEFAULT_SESSION_NAME = "default"`).
- `opencode` is a recognized agent kind (`src/detect/manifests/opencode.toml`), so it can be started
  via `agent start --kind opencode` and get lifecycle tracking. `pi` is likewise a supported kind.
- CLI responses are JSON. Read IDs from the response; do not predict them. IDs are opaque handles
  (workspace `w1`, tab `w1:t1`, pane `w1:p1` style).
- `agent start` vs `pane run`: `agent start` gives lifecycle tracking (the reason to use herdr).
  `pane run <pane> <cmd>` runs a raw process with no agent-state tracking. **Prefer `agent start`.**

---

## 4. Target architecture

```
HOST (you)                          CONTAINER (service)
+------------------+                +-----------------------------+
| tmux             |                | PID 1: tini                 |
|  +------------+  |  docker exec   |   └─ herdr server (headless)|
|  | ctl calls  |--+--------------->|        owns sockets + PTYs  |
|  +------------+  |   (one-shot)   |        workspace: opencode-1|
|  +------------+  |                |          └─ agent oc1       |
|  | herdr UI   |--+--------------->|        workspace: opencode-2|
|  | (attach)   |  |  docker exec -it         └─ agent oc2       |
|  +------------+  |   herdr        |                             |
+------------------+                +-----------------------------+
                                     volume: herdr state (persist)
```

- Container's only job: keep the herdr server alive.
- Agents are created by exec-ing in; each is a separate socket call against the running server.
- Human attaches the UI (`docker exec -it … herdr`) only to watch; otherwise drives via one-shot
  non-interactive `docker exec … herdr <subcommand>`.

---

## 5. Dockerfile

```dockerfile
FROM debian:stable-slim

# tini = real init so `docker stop` SIGTERM is forwarded to herdr's graceful shutdown
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates curl jq tini \
    && rm -rf /var/lib/apt/lists/*

# Install herdr (single binary). NOTE: verify this works in your base image / arch.
# Fallback: COPY a pinned release binary from GitHub releases instead (see §9).
RUN curl -fsSL https://herdr.dev/install.sh | sh

# HERDR_ENV=1 so every `docker exec … herdr <subcommand>` inherits the control gate
ENV HERDR_ENV=1
# herdr state home — persist via a volume so sessions/workspaces survive restarts
ENV XDG_CONFIG_HOME=/state/config \
    XDG_STATE_HOME=/state/state \
    XDG_DATA_HOME=/state/data

WORKDIR /work

# tini at PID 1 → forwards signals + reaps zombies → herdr server is the service
ENTRYPOINT ["tini", "--"]
CMD ["herdr", "server"]
```

Critical: `CMD` is `herdr server` (headless service), **not** bare `herdr` (client UI).

---

## 6. docker-compose.yml

```yaml
services:
  herdr:
    build: .
    container_name: agent-box
    init: true                     # belt-and-suspenders with tini
    restart: unless-stopped        # keep the SERVICE alive; restart replaces, never stacks
    stop_signal: SIGTERM           # forwarded by tini → herdr graceful shutdown
    stop_grace_period: 15s         # let agents settle before SIGKILL
    volumes:
      - herdr-state:/state         # persist sessions/workspaces across restarts
      - ./work:/work               # your code, shared in
    environment:
      HERDR_ENV: "1"
    tty: false                     # headless service, no TTY needed
    healthcheck:
      test: ["CMD", "herdr", "status", "server"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  herdr-state:
```

`restart: unless-stopped` now means "keep the herdr service running." Because the server refuses to
double-bind the socket, a restart is always a clean replace of a dead server.

---

## 7. Operating the service

### Bring it up and confirm it's serving
```bash
docker compose up -d
docker exec agent-box herdr status server     # liveness: proves API socket is bound
docker exec agent-box herdr workspace list
```
`herdr status server` succeeding is the canonical liveness check (also used as the healthcheck).

### Spawn an agent (the two-step pattern)
Run non-interactively — no TTY needed, it's all socket calls:
```bash
docker exec -e HERDR_ENV=1 agent-box sh -c '
  WS=$(herdr workspace create --label opencode-1 --cwd /work | jq -r .result.workspace.workspace_id)
  PANE=$(herdr workspace get "$WS" | jq -r .result.root_pane.pane_id)
  herdr agent start oc1 --kind opencode --pane "$PANE"
'
```
For a second agent in a different workspace, repeat with a different label and name (`opencode-2`,
`oc2`). For `pi`, use `--kind pi`.

> Verify the exact JSON paths (`.result.workspace.workspace_id`, `.result.root_pane.pane_id`)
> against a live response the first time — read the IDs from the response, never predict them.

### Drive an agent
```bash
docker exec agent-box herdr agent prompt oc1 "run the tests and summarize failures" \
  --wait --timeout 120000
docker exec agent-box herdr agent read oc1 --source recent-unwrapped --lines 120
docker exec agent-box herdr agent get oc1
docker exec agent-box herdr agent list
```

### Watch a session (attach the UI from tmux)
```bash
docker exec -it agent-box herdr                 # attach the herdr TUI (session "default")
docker exec -it agent-box herdr agent attach oc1  # full-screen a single agent
# detach a direct attach with:  ctrl+b q
```

### Tear down
```bash
docker compose stop     # SIGTERM → tini → herdr graceful shutdown
docker compose down     # stop + remove (herdr-state volume persists unless you add -v)
```

---

## 8. Suggested host-side helpers (optional convenience)

Drop into the host shell profile so spawning/watching from tmux is one command:
```bash
BOX=agent-box

agent-spawn() {  # agent-spawn <label> <name> [kind] [cwd]
  local label="$1" name="$2" kind="${3:-opencode}" cwd="${4:-/work}"
  docker exec -e HERDR_ENV=1 "$BOX" sh -c "
    WS=\$(herdr workspace create --label '$label' --cwd '$cwd' | jq -r .result.workspace.workspace_id)
    PANE=\$(herdr workspace get \"\$WS\" | jq -r .result.root_pane.pane_id)
    herdr agent start '$name' --kind '$kind' --pane \"\$PANE\"
  "
}
agent-watch() { docker exec -it "$BOX" herdr agent attach "$1"; }   # agent-watch <name>
agent-ls()    { docker exec "$BOX" herdr agent list; }
agent-ui()    { docker exec -it "$BOX" herdr; }
```

---

## 9. Open items / things to verify during implementation

1. **Installer in the base image.** `curl … install.sh | sh` pulls the latest release at build time;
   its behavior inside minimal Debian (arch, extra libs) is unverified. If the build fails, switch to
   a pinned binary: download the correct-arch release asset from
   `https://github.com/herdrdev/herdr/releases` and `COPY`/`ADD` it into the image, `chmod +x`.
2. **Exact state-home paths.** herdr follows XDG-style config/state locations; confirm the dirs it
   actually writes under your chosen `XDG_*` values, and make sure the volume covers them (check
   `herdr --help` output line "Config:" / "Logs:" and the running container's filesystem).
3. **Response JSON shape.** Confirm `.result.*` field paths against a live `workspace create` /
   `workspace get` response before hard-coding them in scripts.
4. **`pi` agent kind flags.** Confirm `--kind pi` and any pi-specific launch args via
   `herdr agent start --help` and `src/detect/manifests/pi.toml`.
5. **Alt-screen output caveat.** Some agents render on the terminal alternate screen; those rows
   don't enter herdr scrollback, so `agent read --lines N` won't recover them. Documented fallback:
   have the agent write its full response to a temp file and reply with the path, then read the file.
6. **Multi-arch.** If building for arm64 + amd64, make the binary install arch-aware.

---

## 10. Safety rules for agents operating this service

- Use `--no-focus` for background work; target agents/panes explicitly (by name or ID), never rely
  on "the focused pane" — another client may be attached.
- Never close panes/workspaces you did not create.
- Never run `herdr server stop` or kill the herdr server process unless you intend to tear the whole
  service down — it takes every agent with it.
- Do not launch `herdr` from inside a herdr pane (nesting is blocked by design).
- Treat one-shot `docker exec … herdr <subcommand>` as the default; attach the UI only to observe.
