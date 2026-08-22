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
browser_dir="$(mktemp -d "$target_dir/served-resident-nbody-browser.XXXXXX")"
server_log="$browser_dir/server.log"
native_log="$browser_dir/native.log"
chrome_log="$browser_dir/chrome.stderr"
dom_file="$browser_dir/chrome.dom"
chrome_profile="$browser_dir/chrome-profile"
server_pid=""

cleanup() {
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf "$project_dir"
  rm -rf "$browser_dir"
}
trap cleanup EXIT

cp examples/n-body/mech.mcfg "$project_dir/mech.mcfg"
cp examples/n-body/n-body.mec "$project_dir/n-body.mec"

"$MECH_BIN" run --max-live-turns 2 "$project_dir" >"$native_log"
if ! grep -q '\[f64\]:1,1' "$native_log" || ! grep -Eq -- '-0\.[0-9]+' "$native_log"; then
  echo "Native n-body run did not publish its live total energy" >&2
  sed -n '1,160p' "$native_log" >&2 || true
  exit 1
fi

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
  if curl --fail --silent "$page_url" >"$browser_dir/index.html.pending" 2>/dev/null; then
    mv "$browser_dir/index.html.pending" "$project_dir/index.html"
    break
  fi
  sleep 0.1
done
if [[ ! -s "$project_dir/index.html" ]]; then
  echo "Resident n-body server did not generate its document" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  exit 1
fi

python3 - "$project_dir/index.html" <<'PY'
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
      root.dataset.mechPageError = event.message || String(event.error);
    });
    window.addEventListener("unhandledrejection", (event) => {
      root.dataset.mechPageError = String(event.reason);
    });

    const originalSetTimeout = window.setTimeout.bind(window);
    let firstX;
    let firstY;
    const deadline = Date.now() + 20000;
    window.requestAnimationFrame = (callback) => originalSetTimeout(() => {
      if (root.dataset.mechDone === "true" || root.dataset.mechTimedOut === "true") return;
      callback(performance.now());
      const body = document.querySelector('[data-mech-scene-id="body-1"]');
      if (body && firstX === undefined) {
        firstX = body.getAttribute("cx");
        firstY = body.getAttribute("cy");
      }
      const display = document.querySelector('[data-mech-display-id="scene-orbit"]');
      const displayUpdates = Number(display?.dataset.mechDisplayUpdates || 0);
      const circles = document.querySelectorAll('[data-mech-scene-id^="body-"]');
      const orbitGuides = document.querySelectorAll('[data-mech-scene-id^="orbit-"]');
      const title = document.querySelector('[data-mech-scene-id="title"]');
      root.dataset.mechObservedRendered = String(displayUpdates);
      if (
        body &&
        circles.length === 10 &&
        orbitGuides.length === 9 &&
        title?.textContent === "Solar-System Orbit Viewer" &&
        displayUpdates >= 60
      ) {
          const outputPanel = document.querySelector('[data-mech-console-panel="output"]');
          root.dataset.mechDone = "true";
          root.dataset.mechRendered = String(displayUpdates);
          root.dataset.mechCircles = String(circles.length);
          root.dataset.mechOrbitGuides = String(orbitGuides.length);
          root.dataset.mechSceneTitle = title.textContent;
          root.dataset.mechBodyMoved = String(
            body.getAttribute("cx") !== firstX || body.getAttribute("cy") !== firstY
          );
          root.dataset.mechOutputPresentation = String(
            document.querySelector('.mech-root')?.dataset.mechPresentation === "output" &&
            document.querySelector('.mech-root')?.dataset.mechPresentationView === "output" &&
            outputPanel?.hidden === false
          );
          root.dataset.mechRichScene = String(
            display?.querySelector('[data-mech-rich-scene="true"]') !== null
          );
          root.dataset.mechRichDisplayOperation = display?.dataset.mechDisplayOperation || "";
          root.dataset.mechRichDisplayUpdates = String(displayUpdates);
          root.dataset.mechRichCircles = String(circles.length);
          globalThis.__MECH_STOP__?.();
      }
      if (Date.now() >= deadline && root.dataset.mechDone !== "true") {
        root.dataset.mechTimedOut = "true";
      }
    }, 16);
  </script>'''
if html.count(marker) != 1:
    raise SystemExit("could not find the head boundary in generated n-body HTML")
path.write_text(html.replace(marker, harness + "\n  " + marker, 1))
PY
for _ in $(seq 1 100); do
  curl --fail --silent "$page_url" >"$browser_dir/preflight.html" || true
  grep -q 'root.dataset.mechDone' "$browser_dir/preflight.html" && break
  sleep 0.1
done
if ! grep -q 'root.dataset.mechDone' "$browser_dir/preflight.html"; then
  echo "Resident n-body server did not load the generated test document" >&2
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
import time

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
    deadline = time.monotonic() + 90
    while True:
        return_code = process.poll()
        if return_code is not None:
            raise SystemExit(return_code)
        try:
            proof_emitted = b'data-mech-done="true"' in Path(dom_file).read_bytes()
        except OSError:
            proof_emitted = False
        if proof_emitted:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            print("headless Chrome retained its process after emitting the browser DOM proof", file=sys.stderr)
            raise SystemExit(124)
        if time.monotonic() >= deadline:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            print("headless Chrome did not emit the D4 DOM proof within 90 seconds", file=sys.stderr)
            raise SystemExit(124)
        time.sleep(0.25)
PY
}

set +e
run_chrome
chrome_status="$?"
set -e

rendered="$(sed -n 's/.*data-mech-rendered="\([0-9][0-9]*\)".*/\1/p' "$dom_file" | head -1)"
if [[ "$chrome_status" -ne 0 && "$chrome_status" -ne 124 ]] \
  || ! grep -q 'data-mech-done="true"' "$dom_file" \
  || ! grep -q 'data-mech-circles="10"' "$dom_file" \
  || ! grep -q 'data-mech-orbit-guides="9"' "$dom_file" \
  || ! grep -q 'data-mech-scene-title="Solar-System Orbit Viewer"' "$dom_file" \
  || ! grep -q 'data-mech-body-moved="true"' "$dom_file" \
  || ! grep -q 'data-mech-output-presentation="true"' "$dom_file" \
  || ! grep -q 'data-mech-rich-scene="true"' "$dom_file" \
  || ! grep -q 'data-mech-rich-display-operation="update"' "$dom_file" \
  || ! grep -q 'data-mech-rich-display-updates="[1-9][0-9]*"' "$dom_file" \
  || ! grep -q 'data-mech-rich-circles="10"' "$dom_file" \
  || [[ -z "$rendered" || "$rendered" -lt 60 ]] \
  || grep -qE 'data-mech-(console-error|page-error|timed-out)=' "$dom_file"; then
  echo "Served resident n-body browser smoke test failed" >&2
  echo "Server log:" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  echo "Chrome stderr:" >&2
  sed -n '1,240p' "$chrome_log" >&2 || true
  echo "Dumped DOM:" >&2
  sed -n '1,420p' "$dom_file" >&2 || true
  exit 1
fi

printf 'NBODY_E2E native_energy=true display_updates=%s bodies=10 orbit_guides=9 title=true rich_scene=true rich_operation=update moved=true output_presentation=true console_errors=0 page_errors=0\n' "$rendered"
