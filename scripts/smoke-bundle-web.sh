#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  if [[ "$CARGO_TARGET_DIR" = /* ]]; then
    target_dir="$CARGO_TARGET_DIR"
  else
    target_dir="$repo_root/$CARGO_TARGET_DIR"
  fi
else
  target_dir="$repo_root/target"
fi

MECH_BIN="${MECH_BIN:-$target_dir/debug/mech}"
if [[ ! -x "$MECH_BIN" ]]; then
  echo "Mech binary is not executable: $MECH_BIN" >&2
  exit 1
fi

mkdir -p "$target_dir"
output_dir="$(mktemp -d "$target_dir/browser-dom-demo-bundle.XXXXXX")"

cleanup() {
  rm -rf "$output_dir"
}
trap cleanup EXIT

"$MECH_BIN" bundle-web examples/browser-dom-demo --out "$output_dir"

for path in \
  index.html \
  style.css \
  pkg/mech_wasm.js \
  pkg/mech_wasm_bg.wasm \
  source \
  code \
  html; do
  if [[ ! -e "$output_dir/$path" ]]; then
    echo "Expected bundle output is missing: $path" >&2
    exit 1
  fi
done
