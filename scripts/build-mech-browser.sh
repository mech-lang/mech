#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

rustup target add wasm32-unknown-unknown
wasm-pack build src/wasm \
  --target web \
  --out-dir ../../examples/pkg \
  --no-default-features \
  --features browser_project
