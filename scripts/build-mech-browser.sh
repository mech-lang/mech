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

python - <<'PY'
from pathlib import Path
try:
    import brotli
except ImportError as exc:
    raise SystemExit('python brotli module is required to compress src/wasm/pkg/mech_wasm_bg.wasm.br') from exc
wasm = Path('src/wasm/pkg/mech_wasm_bg.wasm')
out = Path('src/wasm/pkg/mech_wasm_bg.wasm.br')
out.write_bytes(brotli.compress(wasm.read_bytes()))
PY

test -f src/wasm/pkg/mech_wasm.js
test -f src/wasm/pkg/mech_wasm_bg.wasm
test -f src/wasm/pkg/mech_wasm_bg.wasm.br
cargo build --bin mech
