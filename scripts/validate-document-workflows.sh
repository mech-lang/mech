#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

repeat="${MECH_SMOKE_REPEAT:-1}"
[[ "$repeat" =~ ^[1-9][0-9]*$ ]] || {
  echo "MECH_SMOKE_REPEAT must be a positive integer" >&2
  exit 1
}

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  target_dir="$CARGO_TARGET_DIR"
  [[ "$target_dir" = /* ]] || target_dir="$repo_root/$target_dir"
else
  target_dir="$repo_root/target"
fi

cargo test -p mech \
  mech_tests_preserve_discovered_non_utf8_filename \
  -- --exact --nocapture
cargo test -p mech-runtime \
  --lib \
  resolver::memory \
  -- --nocapture
cargo test -p mech \
  --lib \
  cli::commands::format::document_bundle \
  -- --nocapture
cargo test -p mech \
  --lib \
  cli::commands::format::publication \
  -- --nocapture
cargo test -p mech \
  --no-default-features \
  --features formatter \
  --test mech_format_shims \
  -- --nocapture
cargo test -p mech \
  --no-default-features \
  --features "serve,formatter" \
  --test mech_serve \
  -- --nocapture
bash scripts/check-shipped-document-shims.sh

rm -rf src/wasm/pkg
bash scripts/build-mech-browser.sh
cargo build \
  --bin mech \
  --no-default-features \
  --features "serve,bundle_web,formatter"
cp "$target_dir/debug/mech" "$target_dir/embedded-browser-mech"
cargo test \
  -p mech \
  --no-default-features \
  --features "serve,bundle_web,formatter" \
  --test mech_format_shims \
  -- --nocapture
wasm-pack test \
  --headless \
  --chrome \
  src/wasm \
  --no-default-features \
  --features browser_project \
  -- \
  --nocapture

for attempt in $(seq 1 "$repeat"); do
  echo "Document workflow browser attempt $attempt of $repeat"
  MECH_BIN="$target_dir/embedded-browser-mech" \
    bash scripts/smoke-formatted-document-browser.sh
  MECH_BIN="$target_dir/embedded-browser-mech" \
    bash scripts/smoke-served-fizzbuzz-browser.sh
  MECH_BIN="$target_dir/embedded-browser-mech" \
    bash scripts/smoke-served-rich-document-browser.sh
  MECH_BIN="$target_dir/embedded-browser-mech" \
    bash scripts/smoke-served-analog-clock-browser.sh
  bash scripts/build-mech-browser.sh
  MECH_BIN="$target_dir/embedded-browser-mech" \
    bash scripts/smoke-bundle-web.sh
done

rm -rf src/wasm/pkg
