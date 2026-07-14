#!/usr/bin/env bash
set -euo pipefail

repo_root="$(
  cd "$(dirname "${BASH_SOURCE[0]}")/.." &&
  pwd
)"

cd "$repo_root"

grep -F './pkg/mech_wasm.js' examples/analog-clock/clock.js >/dev/null
grep -F 'examples/analog-clock/pkg' scripts/build-analog-clock-web.sh >/dev/null
grep -F 'scripts/build-analog-clock-web.sh' examples/analog-clock/README.md >/dev/null

if git ls-files examples/analog-clock/pkg | grep -q .; then
  echo "generated analog clock pkg files must not be tracked" >&2
  exit 1
fi
