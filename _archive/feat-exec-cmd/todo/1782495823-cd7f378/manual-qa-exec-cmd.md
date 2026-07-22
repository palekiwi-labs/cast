---
status: open
priority: normal
---
# Manual QA checklist for cargo run -p cast -- exec (feat/exec-cmd)

Generated during review of feat/exec-cmd. The unit tests cover pure logic
(`build_exec_cmd`, `resolve_container_name`, clap parsing, empty-cmd `bail!`).
This checklist covers the docker-side behavior that AC-1..AC-11 require, which
the unit tests cannot reach.

Ref: `.cue/master/task/1782484874-20201aa/exec-cmd.md`
Branch: `feat/exec-cmd`

## Prerequisites
- Docker daemon running (`docker ps` works).
- Working dir with/without cast.json (test both defaults).
- Agent images built, or allow time for `ensure_image` on first run.
- Inspect with: `docker ps --filter name=cast- --format '{{.Names}}\t{{.Ports}}'`
  and `docker port <name>`.
- Learn `<port>` via `cast port opencode`.

## A. cargo run -p cast -- exec core (AC-1..AC-6)
- [x] A1 (AC-1) `cargo run -p cast -- exec opencode /bin/bash` -> interactive bash prompt; `--rm` on exit.
- [x] A2 (AC-2) `cargo run -p cast -- exec --headless opencode .cue/feat-exec-cmd/bin/1782495823-cd7f378/headless-probe.sh`
- [x] A3 (AC-3) `cargo run -p cast -- exec --raw opencode /bin/bash -c "echo hi"` -> prints hi, no `nix develop` wrap; `/nix` still mounted.
- [x] A4 (AC-4) `cargo run -p cast -- exec --publish opencode /bin/bash` -> `docker port` shows `<port>:80`.
- [x] A5 (AC-6) `cargo run -p cast -- exec opencode` (no cmd) -> rejected with "requires a command"; NO container created.
- [x] A6 exit-code propagation: `cargo run -p cast -- exec opencode /bin/bash -c "exit 7"; echo $?` -> 7.

## B. Container naming (AC-8, AC-9, AC-10)
- [ ] B1 (AC-9) `cargo run -p cast -- exec opencode /bin/bash` -> name `cast-opencode-<base>-<port>-exec-<token>`.
- [x] B2 `cargo run -p cast -- exec --headless opencode ...sleep 30` -> `cast-opencode-<base>-<port>-<token>` (no `exec-`, no `-headless-`).
- [ ] B3 (AC-8) `cast run --headless opencode ...` -> `cast-opencode-<base>-<port>-<token>` (no `-headless-` literal).
- [ ] B4 `cast run opencode` (interactive) -> stable `cast-opencode-<base>-<port>`.
- [ ] B5 (AC-10) after B4: `cast shell opencode` -> attaches to the B4 container.
- [ ] B6 `cargo run -p cast -- exec --name custom opencode /bin/bash` -> container name == `custom`.
- [ ] Invariant: B1 name `starts_with` B4 name (prefix-filter preserved).

## C. `--publish` redesign on `cast run` (AC-7)
- [ ] C1 (AC-7) `cast run opencode` -> NO port published (new default).
- [ ] C2 `cast run --publish opencode` -> port published.
- [ ] C3 `cast run -p opencode` -> same as C2.
- [ ] C4 `cargo run -p cast -- exec --headless --publish opencode ...sleep 30` -> publish works in headless mode (decoupled from TTY).

## D. Flags / parsing / aliases
- [ ] D1 `cargo run -p cast -- exec o /bin/bash` (and `p`, `c`) aliases work.
- [ ] D2 hyphen values forwarded: `cargo run -p cast -- exec opencode /bin/bash -c "echo -n hi"`.
- [ ] D3 flags after agent: `cargo run -p cast -- exec opencode --raw /bin/bash` -> confirm `--raw` is NOT consumed by clap.
- [ ] D4 `cargo run -p cast -- exec --help` lists all flags + 3 agent subcommands.
- [ ] D5 `cast help exec` equivalent text.

## E. Regression / cross-cutting
- [ ] E1 `cast run opencode` end-to-end (verifies `run_in_container` extraction).
- [ ] E2 `cast build opencode` still builds.
- [ ] E3 inside raw shell: `ls /nix`, `which nix` (mounted even with `--raw`).
- [ ] E4 two concurrent `cargo run -p cast -- exec --headless ...sleep 30` -> distinct container names.
- [ ] E5 `cargo test` + `cargo clippy -- -D warnings` clean.

## F. Docs gap (Group 4 not executed)
- [ ] F1 `crates/cast/docs/commands/reference.md` has no `cargo run -p cast -- exec` / `--publish` entries; no docs commit on the branch. Decide: in-scope here or separate task.

## Open questions (asked in chat, unresolved)
- [ ] Docs (F1) in-scope for this task or a follow-up?
- [ ] Test against `opencode` only, or all three agents?
- [ ] Include destructive `--name` collision case?
