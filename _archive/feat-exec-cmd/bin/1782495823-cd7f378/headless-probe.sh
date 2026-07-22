#!/usr/bin/env bash
#
# QA probe for `cast exec --headless` (todo A2, feat/exec-cmd).
#
# Runs INSIDE the fresh agent container. Verifies the three things A2 needs:
#   1. stdout/stderr are captured with NO TTY (headless invariant)
#   2. the container environment is real (/nix mounted, workspace present)
#   3. a deterministic exit code propagates
#
# Exit codes:
#   0  success (headless confirmed)
#   2  a TTY was detected -> NOT actually headless
#
# Usage (from the repo root, which is mounted into the container):
#   cp .cue/feat-exec-cmd/bin/<ts>-<hash>/headless-probe.sh ./script.sh
#   chmod +x ./script.sh
#   cast exec --headless opencode ./script.sh
set -u

echo "=== cast-exec-headless-probe: start ==="

# 1. Headless invariant: stdin and stdout must NOT be a terminal.
if [ -t 0 ] || [ -t 1 ]; then
  echo "FAIL: TTY detected on stdin/stdout -- not headless" >&2
  exit 2
fi
echo "ok: no TTY on stdin/stdout (headless confirmed)"

# 2. Container-environment sanity.
echo "hostname: $(hostname)"
echo "cwd:      $(pwd)"
echo "id:       $(id -u)/$(id -g)"

# 3. /nix must be mounted -- cast exec starts the nix daemon unconditionally,
#    even when the command itself is not Nix-wrapped.
if [ -d /nix ]; then
  echo "ok: /nix mounted"
else
  echo "WARN: /nix not present" >&2
fi

# 4. Workspace mount check: a recognizable project marker should be visible.
if [ -f ./Cargo.toml ] || [ -f ./cast.json ]; then
  echo "ok: workspace mounted"
else
  echo "WARN: no Cargo.toml/cast.json marker in cwd" >&2
fi

echo "=== cast-exec-headless-probe: end ==="
exit 0
