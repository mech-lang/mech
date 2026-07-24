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
project_dir="$(mktemp -d "$target_dir/served-analog-clock.XXXXXX")"
server_log="$project_dir/server.log"
chrome_log="$project_dir/chrome.stderr"
dom_file="$project_dir/chrome.dom"
chrome_profile="$project_dir/chrome-profile"
server_pid=""

cleanup() {
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf "$project_dir"
}
trap cleanup EXIT

cp examples/analog-clock/mech.mcfg "$project_dir/mech.mcfg"
cp examples/analog-clock/clock.mec "$project_dir/clock.mec"
cp examples/analog-clock/clock.css "$project_dir/clock.css"
cp examples/analog-clock/index.html "$project_dir/index.html"

python3 - "$project_dir/index.html" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
html = path.read_text()
marker = '<script\n    type="module"\n    src="/_mech/project.js"'
harness = '''<script>
    const root = document.documentElement;
    const originalConsoleError = console.error;
    console.error = (...args) => {
      root.dataset.mechConsoleError = args.map(String).join(" ");
      originalConsoleError.apply(console, args);
    };
    window.addEventListener("error", (event) => {
      root.dataset.mechWindowError = event.message || String(event.error);
    });
    window.addEventListener("unhandledrejection", (event) => {
      root.dataset.mechUnhandledRejection = String(event.reason);
    });

    let initialTransform;
    let deadline;
    let polling = false;
    const poll = () => {
      const secondHand = document.querySelector('[data-mech-scene-id="clock-second-hand"]');
      if (secondHand) {
        const transform = secondHand.getAttribute("transform") || "";
        if (initialTransform === undefined) {
          initialTransform = transform;
          deadline = Date.now() + 5000;
        } else if (transform !== initialTransform) {
          root.dataset.mechLiveUpdated = "true";
          return;
        }
      }
      if (deadline !== undefined && Date.now() >= deadline) {
        root.dataset.mechLiveUpdated = "false";
        return;
      }
      window.setTimeout(poll, 50);
    };

    const originalSetTimeout = window.setTimeout.bind(window);
    window.requestAnimationFrame = (callback) => originalSetTimeout(() => {
      callback(performance.now());
      if (!polling) {
        polling = true;
        poll();
      }
    }, 16);
</script>'''
if marker not in html:
    raise SystemExit("could not find the project module script in temporary index.html")
path.write_text(html.replace(marker, harness + "\n  " + marker, 1))
PY

port="$(python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
)"

"$MECH_BIN" serve \
  --address 127.0.0.1 \
  --port "$port" \
  --wasm "$repo_root/src/wasm/pkg" \
  "$project_dir" >"$server_log" 2>&1 &
server_pid="$!"

page_url="http://127.0.0.1:${port}/"
for _ in $(seq 1 100); do
  if curl --fail --silent --show-error --output /dev/null "$page_url"; then
    break
  fi
  sleep 0.1
done
if ! curl --fail --silent --show-error --output /dev/null "$page_url"; then
  echo "Server did not respond at $page_url" >&2
  echo "Server log:" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  exit 1
fi

google-chrome \
  --headless=new \
  --no-sandbox \
  --disable-gpu \
  --disable-dev-shm-usage \
  --run-all-compositor-stages-before-draw \
  --virtual-time-budget=30000 \
  --dump-dom \
  --user-data-dir="$chrome_profile" \
  "$page_url" >"$project_dir/chrome-warmup.dom" 2>"$project_dir/chrome-warmup.stderr"

set +e
google-chrome \
  --headless=new \
  --no-sandbox \
  --disable-gpu \
  --disable-dev-shm-usage \
  --run-all-compositor-stages-before-draw \
  --virtual-time-budget=7000 \
  --dump-dom \
  --user-data-dir="$chrome_profile" \
  "$page_url" >"$dom_file" 2>"$chrome_log"
chrome_status="$?"
set -e

if [[ "$chrome_status" -ne 0 ]] \
  || ! grep -q 'data-mech-live-updated="true"' "$dom_file" \
  || ! grep -q 'data-mech-scene-id="clock-second-hand"' "$dom_file" \
  || grep -qE 'data-mech-console-error|data-mech-window-error|data-mech-unhandled-rejection|data-mech-live-updated="false"' "$dom_file"; then
  echo "Served analog-clock browser smoke test failed" >&2
  echo "Server log:" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  echo "Chrome stderr:" >&2
  sed -n '1,240p' "$chrome_log" >&2 || true
  echo "Dumped DOM:" >&2
  sed -n '1,400p' "$dom_file" >&2 || true
  exit 1
fi
