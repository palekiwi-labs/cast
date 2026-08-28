# Renovate merge review

## Scope

Reviewed `master...chore/setup-renovate` (commit `79e8960`). The branch adds
`.github/renovate.json` only.

## Finding

### High — Nix flake updates are not enabled

`.github/renovate.json` adds a `packageRule` that matches the `nix` manager,
but it never enables that manager. Renovate documents the Nix manager as beta
and disabled by default; it must be explicitly enabled with:

```json
"nix": { "enabled": true }
```

Consequently, the configured rule for grouping Nix flake inputs has no effect,
so the stated task intent to manage Nix flake inputs is not achieved. Add the
explicit manager enablement, then validate the configuration in the Renovate
app.

## Merge assessment

Do not merge as-is if the accepted scope includes Nix flake input updates. The
Cargo and GitHub Actions rules are syntactically valid, but the Nix portion is
non-functional.

## Observations

- The JSON is well-formed and `git diff --check` is clean.
- Cargo lock-file maintenance is supported by Renovate.
- This repository has no pull-request workflow: the existing build runs only
  after pushes to `master`. This is an operational risk for automated updates,
  but is not a defect in the Renovate configuration itself.
- Keep `chore(deps)` for major Cargo updates unless the project intentionally
  wants dependency majors classified as features; the existing per-rule
  `feat(deps):` overrides the global semantic commit type.
