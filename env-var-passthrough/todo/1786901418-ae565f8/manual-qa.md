---
status: closed
priority: high
parent: env-var-passthrough
refs:
- .cue/master/task/env-var-passthrough.md
- .cue/env-var-passthrough/plan/1785678626-4f8ccff/env-passthrough.md
---
# Manual QA: env_passthrough

> **Closed 2026-08-23 as superseded.** This checklist targets the
> single-key design. Sections 1, 3, 4, 5, 6 were executed and ticked
> on a real host (valueless `-e` inheritance proven end to end; no
> leaks to argv, approval store, logs, or config show; empty treated
> as unset; passthrough beats cast.env; cast authoritative for USER);
> their Result lines were left blank. Sections 2, 7, 8, 9 and sign-off
> never ran, and the pivot redefines section 7's replace semantics for
> both keys — notably, the end-to-end approval-gate check (section 2,
> the security property) is still unproven on a real host. A fresh
> checklist must be derived from the two-key design in the pivot's
> plan and must include that check.

Automated tests cover the pure helper, the config merge, the approval
hash, and the argv construction. They do **not** cover the part that
only a real `docker run` can prove: that the valueless `-e NAME` form
actually carries the value across, because that relies on the docker
child inheriting `cast`'s environment.

Run these against a built image on a real host. Sign off by filling the
Result lines, then set `status: complete`.

Setup, once:

```bash
cd <a scratch project with a cast.json>
cargo build -p cast            # or use the branch binary directly
export CAST=<path>/target/debug/cast
export QA_TOKEN=hunter2
```

Throughout, `cast shell --raw opencode` is the cheapest way into a
container; `--raw` skips the Nix devshell so startup is fast.

## 1. The happy path: value actually arrives

- [x] Add `"env_passthrough": ["QA_TOKEN"]` to the project `cast.json`.
- [x] `$CAST config show` lists `QA_TOKEN` under `env_passthrough`.
- [x] `$CAST config allow`
- [x] `QA_TOKEN=hunter2 $CAST shell --raw opencode`, then inside the
      container: `echo "$QA_TOKEN"`.

Expect `hunter2`. This is the single most important check: it is the
only proof that the inheritance assumption holds end to end.

Result:

## 2. Approval actually gates it

 [ ] With the config approved, add a second name (`"QA_OTHER"`) to
      `cast.json` by hand.
- [ ] `$CAST config diff` shows the added name.
- [ ] `$CAST shell --raw opencode` **fails** with an approval error
      rather than starting.
- [ ] `$CAST config allow`, then the shell starts again.

This is the security property. If the run succeeds at step three, the
feature is not safe to merge.

Result:

## 3. No secret leaks to the host

With a container running from step 1, on the host:

- [x] `ps aux | rg QA_TOKEN` shows the name but **never** `hunter2`.
- [x] `rg -i hunter2 ~/.local/share/cast/approved_configs.json` finds
      nothing.
- [x] `$CAST config show` prints the name, not the value.
- [x] `RUST_LOG=debug $CAST shell --raw opencode 2>&1 | rg hunter2`
      finds nothing; `... | rg QA_TOKEN` does show the name.
- [x] Check today's file in `$CAST_LOG_DIR` (or the default log dir) for
      `hunter2`. Expect no match.

Result:

## 4. Unset and empty names

- [x] `unset QA_TOKEN`, then `$CAST shell --raw opencode`; inside,
      `env | rg QA_TOKEN` finds nothing. The container starts normally —
      a missing name is not an error.
- [x] `QA_TOKEN= $CAST shell --raw opencode`; inside, `env | rg QA_TOKEN`
      finds nothing. Set-but-empty is treated as unset, so it cannot
      shadow a real `cast.env` value.

Result:

## 5. Precedence against cast.env

- [x] Put `QA_TOKEN=from-env-file` in the project `cast.env`.
- [x] `QA_TOKEN=from-host $CAST shell --raw opencode`; inside,
      `echo "$QA_TOKEN"` prints `from-host`.
- [x] Unset `QA_TOKEN` on the host and repeat: it prints
      `from-env-file`.

Result:

## 6. cast stays authoritative for its own names

- [x] Add `"USER"` to `env_passthrough`, `$CAST config allow`, then
      `USER=nobody $CAST shell --raw opencode`.
- [x] Inside, `echo "$USER"` prints the container user that cast sets,
      **not** `nobody`.

Result:

## 7. Project config replaces the global one

- [ ] Put `"env_passthrough": ["GLOBAL_ONLY"]` in the global config
      (`~/.config/cast/cast.json`).
- [ ] With no `env_passthrough` in the project `cast.json`,
      `$CAST config show` lists `GLOBAL_ONLY`.
- [ ] Add `"env_passthrough": ["QA_TOKEN"]` to the project config.
      `$CAST config show` now lists `QA_TOKEN` **only** — `GLOBAL_ONLY`
      is gone, not merged.

Result:

## 8. Invalid names do not break the run

- [ ] Set `"env_passthrough": ["", "BAD-NAME", "9LEADING", "QA_TOKEN"]`,
      approve, and run.
- [ ] The container starts and `QA_TOKEN` is present inside. The three
      invalid entries are skipped silently rather than aborting the run.

Result:

## 9. Docs match reality

- [ ] Read `crates/cast/docs/config/env-overrides.md` against what you
      just observed. Every claim in the section should have been
      exercised above.
- [ ] Confirm the trust-boundary paragraph reads as an honest statement
      of the limitation, not a reassurance.

Result:

## Sign-off

- [ ] All of the above pass, or the failures are recorded above and
      accepted.
- [ ] Task `env-var-passthrough` may be set to `complete`.
