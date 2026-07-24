---
refs:
- .cue/master/trace/1784881352-9224622/container-identity-vs-shell-selection.md
- .cue/nix-native-harnesses/spec/index.md
---
# Should we ship a "universal" shell? And what is the container-identity model?

Conversation anchor arising from reviewing how the nix-native pivot affects
`cast shell` / `cast run`. Full current-state analysis in the linked trace.

## The two intertwined questions

1. **Should the shipped global-flake template include a `universal` devShell,
   and what does it even mean?**
2. **What is the right container-identity model now that image and shell are
   decoupled?**

## Where we landed so far

- Confirmed and agreed: keep the **per-agent container model** as the default
  (`cast-{agent}-{basename}-{port}`, per-agent port). It enables running
  opencode / claudecode / pi as three independent containers per project,
  solving different problems in isolation. This is a genuinely valuable default,
  not an accident to be undone.

## The unresolved tension

- The `universal` devShell (all three harnesses on PATH) is the concrete
  enabler of the spec's `cast-agent` motivation: one harness invoking another as
  a subprocess in the same environment.
- But it currently has **no coherent home**: there is no `universal` agent, so
  `cast run universal` is invalid. It is reachable only via
  `global_shell = "universal"` on a real agent, which yields a container whose
  name (e.g. `cast-opencode-...`) lies about its contents (all three harnesses).

## Options to develop

- **Drop it (for now):** ship only per-harness shells + `default`. The universal
  environment is deferred until `cast-agent` actually needs it and we can design
  the invocation/identity story properly. Smallest surface; avoids shipping an
  orphaned, misleading capability.
- **Keep it, give it a home:** introduce a first-class notion (e.g. a
  `universal`/`all` pseudo-agent or a `cast run --all` / `cast run --shell
  universal` flag) that names the container after the *shell*, not a harness —
  `cast-universal-{basename}-{port}`. Cleanly separates identity from harness.
- **Rethink identity generally:** allow container identity to be user-labelled
  (`cast run --name feature-x --shell <shell>`), making harness fully orthogonal
  to identity. Larger design; the universal shell then just becomes one shell
  choice among many.

## Sub-question worth naming

`agent.name()` currently overloads two roles — container identity key AND
default devShell selector. Now that `global_shell` can override the shell, these
can drift apart (name says opencode, shell is universal). Any resolution should
decide whether to re-fuse them (name derives from resolved shell) or fully
separate them (identity is its own axis).

## Next step

Discuss and decide. If the outcome warrants tracked work, promote to a `task`
(likely a follow-up to nix-native-harnesses) and close this note. The immediate,
mergeable decision is just: does the 0.2.0 template ship `universal` or not?
