#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

test ! -f examples/analog-clock/clock.js
test ! -e examples/common
grep -F '/_mech/project.js' examples/analog-clock/index.html >/dev/null
grep -F '/_mech/pkg/mech_wasm.js' include/project.js >/dev/null
grep -F 'src/wasm/pkg' scripts/build-mech-browser.sh >/dev/null
if git ls-files examples/pkg | grep -q .; then
  echo "generated browser pkg files must not be tracked" >&2
  exit 1
fi
