#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

chrome_bin="${CHROME_BIN:-google-chrome}"
cargo_target_dir="${CARGO_TARGET_DIR:-target}"
mech_bin="${MECH_BIN:-${cargo_target_dir}/debug/mech}"
port="${MECH_BROWSER_SMOKE_PORT:-18081}"
url="http://127.0.0.1:${port}/"
scratch_dir="$(mktemp -d "${TMPDIR:-/tmp}/mech-browser-smoke.XXXXXX")"
server_log="$scratch_dir/server.log"
chrome_log="$scratch_dir/chrome.log"
rendered_dom="$scratch_dir/rendered.html"
server_pid=""

cleanup() {
  status=$?
  trap - EXIT INT TERM
  if [[ -n "$server_pid" ]] && kill -0 "$server_pid" 2>/dev/null; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  if [[ "$status" -ne 0 ]]; then
    if [[ -s "$server_log" ]]; then
      echo "mech serve output:" >&2
      sed -n '1,240p' "$server_log" >&2
    fi
    if [[ -s "$chrome_log" ]]; then
      echo "headless Chrome output:" >&2
      sed -n '1,240p' "$chrome_log" >&2
    fi
    if [[ -s "$rendered_dom" ]]; then
      echo "rendered DOM:" >&2
      sed -n '1,80p' "$rendered_dom" >&2
    fi
  fi
  rm -rf -- "$scratch_dir"
  exit "$status"
}
trap cleanup EXIT INT TERM

if ! command -v "$chrome_bin" >/dev/null 2>&1; then
  echo "headless Chrome executable not found: $chrome_bin" >&2
  exit 1
fi
if [[ ! -x "$mech_bin" ]]; then
  echo "$mech_bin is missing; build the serve-enabled CLI before this smoke test" >&2
  exit 1
fi
if [[ ! -f src/wasm/pkg/mech_wasm.js || ! -f src/wasm/pkg/mech_wasm_bg.wasm ]]; then
  echo "browser package is missing; run scripts/build-mech-browser.sh before this smoke test" >&2
  exit 1
fi

"$mech_bin" serve examples/analog-clock \
  --address 127.0.0.1 \
  --port "$port" \
  --wasm src/wasm/pkg \
  >"$server_log" 2>&1 &
server_pid=$!

ready=false
for _ in {1..120}; do
  if ! kill -0 "$server_pid" 2>/dev/null; then
    echo "mech serve exited before accepting connections" >&2
    exit 1
  fi
  if curl --fail --silent --output /dev/null "$url"; then
    ready=true
    break
  fi
  sleep 0.25
done
if [[ "$ready" != true ]]; then
  echo "mech serve did not become ready at $url" >&2
  exit 1
fi

chrome_command=(
  "$chrome_bin"
  --headless
  --no-sandbox
  --no-proxy-server
  --disable-dev-shm-usage
  --disable-gpu
  "--user-data-dir=$scratch_dir/chrome"
  --run-all-compositor-stages-before-draw
  --virtual-time-budget=5000
  --dump-dom
  "$url"
)
if command -v timeout >/dev/null 2>&1; then
  timeout 30s "${chrome_command[@]}" >"$rendered_dom" 2>"$chrome_log"
else
  "${chrome_command[@]}" >"$rendered_dom" 2>"$chrome_log"
fi

for scene_id in clock-hour-hand clock-minute-hand clock-second-hand clock-center-pin; do
  if ! grep -F "data-mech-scene-id=\"$scene_id\"" "$rendered_dom" >/dev/null; then
    echo "served analog clock did not render scene element: $scene_id" >&2
    exit 1
  fi
done
for hand_id in clock-hour-hand clock-minute-hand clock-second-hand; do
  if ! grep -E "data-mech-scene-id=\"$hand_id\"[^>]*transform=\"rotate\\(" "$rendered_dom" >/dev/null; then
    echo "served analog clock did not rotate scene element: $hand_id" >&2
    exit 1
  fi
done
