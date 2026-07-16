#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

bash scripts/build-mech-browser.sh
cargo build --bin mech

servers=()
cleanup() {
  for pid in "${servers[@]:-}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
}
trap cleanup EXIT

wait_http() {
  local url="$1"
  for _ in $(seq 1 100); do
    if curl -fsS "$url" >/dev/null; then
      return 0
    fi
    sleep 0.1
  done
  echo "timed out waiting for $url" >&2
  return 1
}

check_routes() {
  local base="$1" source="$2"
  curl -fsS "$base/" >/dev/null
  curl -fsS "$base/mech.mcfg" >/dev/null
  curl -fsS "$base/$source" >/dev/null
  curl -fsS "$base/_mech/project.js" >/dev/null
  curl -fsS "$base/_mech/pkg/mech_wasm.js" >/dev/null
  local headers body magic content_type content_encoding
  headers="$(mktemp)"
  body="$(mktemp)"
  curl -fsS -D "$headers" -o "$body" "$base/_mech/pkg/mech_wasm_bg.wasm"
  magic="$(head -c 4 "$body" | od -An -t x1 | tr -d ' \n')"
  test "$magic" = "0061736d"
  content_type="$(awk 'BEGIN{IGNORECASE=1} /^content-type:/ {print tolower($0)}' "$headers")"
  echo "$content_type" | grep -q 'application/wasm'
  content_encoding="$(awk 'BEGIN{IGNORECASE=1} /^content-encoding:/ {print tolower($0)}' "$headers")"
  test -z "$content_encoding"
}

run_server() {
  local project="$1" port="$2"
  target/debug/mech serve "$project" --port "$port" >"/tmp/mech-serve-${port}.log" 2>&1 &
  local pid=$!
  servers+=("$pid")
  wait_http "http://127.0.0.1:${port}/"
}

run_server examples/analog-clock 8123
check_routes http://127.0.0.1:8123 clock.mec


command -v google-chrome >/dev/null

google-chrome --headless=new --disable-gpu --virtual-time-budget=3000 --screenshot=/tmp/analog-clock.png http://127.0.0.1:8123/ >/tmp/chrome-clock.log 2>&1
test -s /tmp/analog-clock.png

clock_hash="$(sha256sum /tmp/analog-clock.png | awk '{print $1}')"
test -n "$clock_hash"
! grep -iE 'error|exception|panic' /tmp/chrome-clock.log
