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
server_log="$output_dir/server.log"
chrome_log="$output_dir/chrome.stderr"
dom_file="$output_dir/chrome.dom"
chrome_profile="$output_dir/chrome-profile"
server_pid=""

cleanup() {
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf "$output_dir"
}
trap cleanup EXIT

"$MECH_BIN" bundle-web examples/browser-dom-demo --out "$output_dir"

for path in \
  index.html \
  mech.mcfg \
  style.css \
  pkg/mech_wasm.js \
  pkg/mech_wasm_bg.wasm \
  _mech/project.js \
  _mech/project-sources.json \
  source \
  code \
  html; do
  if [[ ! -e "$output_dir/$path" ]]; then
    echo "Expected bundle output is missing: $path" >&2
    exit 1
  fi
done

python3 - "$output_dir/index.html" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
html = path.read_text()
marker = "</head>"
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

  const expected = {
    title: "Hello, Ada",
    output: "Hello, Ada — computed in Mech",
    status: "Read `Ada` from the DOM and wrote the computed result back.",
  };
  const deadline = Date.now() + 20000;
  const poll = () => {
    const title = document.getElementById("title")?.textContent || "";
    const output = document.getElementById("roundtrip-output")?.value || "";
    const status = document.getElementById("status")?.textContent || "";
    root.dataset.mechBundleTitle = title;
    root.dataset.mechBundleOutput = output;
    root.dataset.mechBundleStatus = status;
    if (title === expected.title && output === expected.output && status === expected.status) {
      root.dataset.mechBundleReady = "true";
      return;
    }
    if (Date.now() >= deadline) {
      root.dataset.mechBundleReady = "false";
      return;
    }
    window.setTimeout(poll, 25);
  };
  poll();
</script>'''
if marker not in html:
    raise SystemExit("could not find </head> in bundled index.html")
path.write_text(html.replace(marker, harness + "\n" + marker, 1))
PY

port="$(python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
)"

python3 -m http.server "$port" --bind 127.0.0.1 --directory "$output_dir" >"$server_log" 2>&1 &
server_pid="$!"
page_url="http://127.0.0.1:${port}/"

for _ in $(seq 1 100); do
  if curl --fail --silent --show-error --output /dev/null "$page_url"; then
    break
  fi
  sleep 0.1
done
if ! curl --fail --silent --show-error --output /dev/null "$page_url"; then
  echo "Static bundle server did not respond at $page_url" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  exit 1
fi

python3 - "$output_dir/_mech/project-sources.json" <<'PY'
import json
from pathlib import Path
import sys

manifest = json.loads(Path(sys.argv[1]).read_text())
if manifest.get("version") != 1:
    raise SystemExit(f"unexpected source manifest version: {manifest!r}")
sources = manifest.get("sources")
if not isinstance(sources, list):
    raise SystemExit(f"source manifest does not contain a source list: {manifest!r}")
pairs = {(source.get("specifier"), source.get("url")) for source in sources}
expected = {
    ("demo.mec", "source/demo.mec"),
    ("denied.mec", "source/denied.mec"),
}
if pairs != expected:
    raise SystemExit(f"unexpected source manifest entries: {manifest!r}")
PY

google-chrome \
  --headless=new \
  --no-sandbox \
  --disable-gpu \
  --disable-dev-shm-usage \
  --run-all-compositor-stages-before-draw \
  --virtual-time-budget=30000 \
  --dump-dom \
  --user-data-dir="$chrome_profile" \
  "$page_url" >"$output_dir/chrome-warmup.dom" 2>"$output_dir/chrome-warmup.stderr"

set +e
google-chrome \
  --headless=new \
  --no-sandbox \
  --disable-gpu \
  --disable-dev-shm-usage \
  --run-all-compositor-stages-before-draw \
  --virtual-time-budget=20000 \
  --dump-dom \
  --user-data-dir="$chrome_profile" \
  "$page_url" >"$dom_file" 2>"$chrome_log"
chrome_status="$?"
set -e

if [[ "$chrome_status" -ne 0 ]] \
  || ! grep -q 'data-mech-bundle-ready="true"' "$dom_file" \
  || ! grep -q 'data-mech-bundle-title="Hello, Ada"' "$dom_file" \
  || ! grep -q 'data-mech-bundle-output="Hello, Ada — computed in Mech"' "$dom_file" \
  || ! grep -q 'data-mech-bundle-status="Read `Ada` from the DOM and wrote the computed result back."' "$dom_file" \
  || grep -qE 'data-mech-console-error|data-mech-window-error|data-mech-unhandled-rejection|data-mech-bundle-ready="false"' "$dom_file"; then
  echo "Static browser bundle smoke test failed" >&2
  echo "Server log:" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  echo "Chrome stderr:" >&2
  sed -n '1,240p' "$chrome_log" >&2 || true
  echo "Dumped DOM:" >&2
  sed -n '1,400p' "$dom_file" >&2 || true
  exit 1
fi
