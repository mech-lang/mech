#!/usr/bin/env bash
set -euo pipefail

repo_root="$(
  cd "$(dirname "${BASH_SOURCE[0]}")/.." &&
  pwd
)"

cd "$repo_root/src/wasm"

wasm-pack build . \
  --target web \
  --out-dir ../../examples/analog-clock/pkg \
  --out-name mech_wasm \
  -- \
  --no-default-features \
  --features analog_clock_demo

test -f "$repo_root/examples/analog-clock/pkg/mech_wasm.js"
test -f "$repo_root/examples/analog-clock/pkg/mech_wasm_bg.wasm"
