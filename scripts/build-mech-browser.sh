#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

rustup target add wasm32-unknown-unknown
rm -rf src/wasm/pkg
wasm-pack build src/wasm \
  --target web \
  --out-dir pkg \
  --no-default-features \
  --features browser_project

test -f src/wasm/pkg/mech_wasm.js
test -f src/wasm/pkg/mech_wasm_bg.wasm
