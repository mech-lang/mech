#!/usr/bin/env bash
set -euo pipefail

repo_root="$(
  cd "$(dirname "${BASH_SOURCE[0]}")/.." &&
  pwd
)"

cd "$repo_root"

test ! -f examples/analog-clock/clock.js
grep -F '../common/mech-browser.js' examples/analog-clock/index.html >/dev/null
grep -F '../pkg/mech_wasm.js' examples/common/mech-browser.js >/dev/null
grep -F 'examples/pkg' scripts/build-mech-browser.sh >/dev/null
grep -F 'scripts/build-mech-browser.sh' examples/analog-clock/README.md >/dev/null

if git ls-files examples/pkg | grep -q .; then
  echo "generated browser pkg files must not be tracked" >&2
  exit 1
fi
