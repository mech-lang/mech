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

if [[ -n "${CHROME_BIN:-}" ]]; then
  chrome_bin="$CHROME_BIN"
elif command -v google-chrome >/dev/null 2>&1; then
  chrome_bin="$(command -v google-chrome)"
elif [[ -x "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" ]]; then
  chrome_bin="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
elif command -v chromium >/dev/null 2>&1; then
  chrome_bin="$(command -v chromium)"
else
  echo "No supported headless Chrome executable was found" >&2
  exit 1
fi

mkdir -p "$target_dir"
project_dir="$(mktemp -d "$target_dir/served-resident-nbody.XXXXXX")"
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

cp examples/resident-n-body/mech.mcfg "$project_dir/mech.mcfg"
cp examples/resident-n-body/n-body.mec "$project_dir/n-body.mec"
cp examples/resident-n-body/n-body.css "$project_dir/n-body.css"
cp examples/resident-n-body/index.html "$project_dir/index.html"

python3 - "$project_dir/mech.mcfg" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
config = path.read_text()
marker = '  run: {\n    paths: ["n-body.mec"]'
replacement = '  serve: {\n    paths: ["."]\n  }\n\n  run: {\n    paths: ["n-body.mec"]'
if config.count(marker) != 1:
    raise SystemExit("temporary n-body config has an unexpected run block")
path.write_text(config.replace(marker, replacement, 1))
PY

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
      root.dataset.mechPageError = event.message || String(event.error);
    });
    window.addEventListener("unhandledrejection", (event) => {
      root.dataset.mechPageError = String(event.reason);
    });

    const originalSetTimeout = window.setTimeout.bind(window);
    let renderedUpdates = 0;
    let firstX;
    let firstY;
    const deadline = Date.now() + 20000;
    window.requestAnimationFrame = (callback) => originalSetTimeout(() => {
      if (root.dataset.mechDone === "true" || root.dataset.mechTimedOut === "true") return;
      callback(performance.now());
      const frame = globalThis.__MECH_LAST_FRAME__;
      if (frame) renderedUpdates += Number(frame.rendered || 0);
      const info = globalThis.__MECH_RUNTIME_INFO__?.();
      const body = document.querySelector('[data-mech-scene-id="body-1"]');
      if (body && firstX === undefined) {
        firstX = body.getAttribute("cx");
        firstY = body.getAttribute("cy");
      }
      if (info) {
        root.dataset.mechObservedAccepted = String(info.resident_accepted_turns);
        root.dataset.mechObservedRendered = String(renderedUpdates);
        if (body && info.resident_accepted_turns >= 60) {
          const circles = document.querySelectorAll('[data-mech-scene-id^="body-"]');
          root.dataset.mechDone = "true";
          root.dataset.mechRoute = info.route;
          root.dataset.mechAccepted = String(info.resident_accepted_turns);
          root.dataset.mechRejected = String(info.resident_rejected_turns);
          root.dataset.mechLegacy = String(info.legacy_turns);
          root.dataset.mechRendered = String(renderedUpdates);
          root.dataset.mechCircles = String(circles.length);
          root.dataset.mechBodyMoved = String(
            body.getAttribute("cx") !== firstX || body.getAttribute("cy") !== firstY
          );
        }
      }
      if (Date.now() >= deadline && root.dataset.mechDone !== "true") {
        root.dataset.mechTimedOut = "true";
      }
    }, 16);
  </script>'''
if html.count(marker) != 1:
    raise SystemExit("could not find project module script in n-body index.html")
path.write_text(html.replace(marker, harness + "\n  " + marker, 1))
PY

port="$(python3 - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
)"

"$MECH_BIN" serve --address 127.0.0.1 --port "$port" "$project_dir" >"$server_log" 2>&1 &
server_pid="$!"
page_url="http://127.0.0.1:${port}/"
for _ in $(seq 1 100); do
  curl --fail --silent --output /dev/null "$page_url" && break
  sleep 0.1
done
if ! curl --fail --silent --show-error --output /dev/null "$page_url"; then
  echo "Resident n-body server did not respond" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  exit 1
fi

run_chrome() {
  python3 - "$chrome_bin" "$page_url" "$chrome_profile" "$dom_file" "$chrome_log" <<'PY'
import os
from pathlib import Path
import signal
import subprocess
import sys

chrome, page_url, profile, dom_file, chrome_log = sys.argv[1:]
args = [
    chrome,
    "--headless=new",
    "--no-sandbox",
    "--disable-gpu",
    "--disable-dev-shm-usage",
    "--run-all-compositor-stages-before-draw",
    "--virtual-time-budget=22000",
    "--dump-dom",
    f"--user-data-dir={profile}",
    page_url,
]
with Path(dom_file).open("wb") as stdout, Path(chrome_log).open("wb") as stderr:
    process = subprocess.Popen(args, stdout=stdout, stderr=stderr, start_new_session=True)
    try:
        raise SystemExit(process.wait(timeout=75))
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()
        print("headless Chrome exceeded the 75-second D4 deadline", file=sys.stderr)
        raise SystemExit(124)
PY
}

set +e
run_chrome
chrome_status="$?"
set -e

accepted="$(sed -n 's/.*data-mech-accepted="\([0-9][0-9]*\)".*/\1/p' "$dom_file" | head -1)"
rendered="$(sed -n 's/.*data-mech-rendered="\([0-9][0-9]*\)".*/\1/p' "$dom_file" | head -1)"
if [[ "$chrome_status" -ne 0 ]] \
  || ! grep -q 'data-mech-done="true"' "$dom_file" \
  || ! grep -q 'data-mech-route="resident-external"' "$dom_file" \
  || ! grep -q 'data-mech-rejected="0"' "$dom_file" \
  || ! grep -q 'data-mech-legacy="0"' "$dom_file" \
  || ! grep -q 'data-mech-circles="10"' "$dom_file" \
  || ! grep -q 'data-mech-body-moved="true"' "$dom_file" \
  || [[ -z "$accepted" || "$accepted" -lt 60 ]] \
  || [[ -z "$rendered" || "$rendered" -lt 60 ]] \
  || grep -qE 'data-mech-console-error|data-mech-page-error|data-mech-timed-out' "$dom_file"; then
  echo "Served resident n-body browser smoke test failed" >&2
  echo "Server log:" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  echo "Chrome stderr:" >&2
  sed -n '1,240p' "$chrome_log" >&2 || true
  echo "Dumped DOM:" >&2
  sed -n '1,420p' "$dom_file" >&2 || true
  exit 1
fi

printf 'D4_BROWSER route=resident-external accepted=%s rejected=0 legacy=0 rendered=%s circles=10 moved=true console_errors=0 page_errors=0\n' "$accepted" "$rendered"
