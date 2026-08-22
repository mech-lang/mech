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
project_dir="$(mktemp -d "$target_dir/served-resident-ekf.XXXXXX")"
browser_dir="$(mktemp -d "$target_dir/served-resident-ekf-browser.XXXXXX")"
server_log="$browser_dir/server.log"
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

cp examples/ekf/mech.mcfg "$project_dir/mech.mcfg"
cp examples/ekf/localization.mec "$project_dir/localization.mec"

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
server_ready_timeout_seconds="${MECH_BROWSER_SERVER_READY_TIMEOUT_SECONDS:-60}"
server_ready_deadline=$((SECONDS + server_ready_timeout_seconds))
while ((SECONDS < server_ready_deadline)); do
  if curl --fail --silent "$page_url" >"$browser_dir/index.html.pending" 2>/dev/null; then
    mv "$browser_dir/index.html.pending" "$project_dir/index.html"
    break
  fi
  sleep 0.1
done
if [[ ! -s "$project_dir/index.html" ]]; then
  echo "Resident EKF server did not generate its document" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  exit 1
fi

python3 - "$project_dir/index.html" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
html = path.read_text()
marker = "</head>"
harness = r'''<script>
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
    const deadline = Date.now() + 20000;
    let firstTruth;
    window.requestAnimationFrame = (callback) => originalSetTimeout(() => {
      if (root.dataset.mechDone === "true" || root.dataset.mechTimedOut === "true") return;
      callback(performance.now());

      const display = document.querySelector('[data-mech-display-id="scene-localization"]');
      const scene = document.querySelector('[data-mech-rich-scene="true"]');
      const truth = document.querySelector('[data-mech-scene-id="truth"]');
      const estimate = document.querySelector('[data-mech-scene-id="estimate"]');
      const prediction = document.querySelector('[data-mech-scene-id="prediction"]');
      const covariance = document.querySelector('[data-mech-scene-id="covariance"]');
      const truthPath = document.querySelector('[data-mech-scene-id="truth-path"]');
      const estimatePath = document.querySelector('[data-mech-scene-id="estimate-path"]');
      const title = document.querySelector('[data-mech-scene-id="title"]');
      const cameras = document.querySelectorAll('[data-mech-scene-id^="camera-"]:not([data-mech-scene-id^="camera-label-"])');
      const host = document.querySelector('[data-mech-repl-host]');
      const outputPanel = document.querySelector('[data-mech-console-panel="output"]');
      const outputToggle = document.querySelector('button[data-mech-output-fullscreen]');
      const tabs = [...document.querySelectorAll('[data-mech-console-tab]')]
        .map(tab => tab.dataset.mechConsoleTab).join(",");
      const updates = Number(display?.dataset.mechDisplayUpdates || 0);
      const truthPoint = truth
        ? `${truth.getAttribute("cx")},${truth.getAttribute("cy")}`
        : undefined;
      if (truthPoint && firstTruth === undefined) firstTruth = truthPoint;
      const truthMoved = Boolean(firstTruth && truthPoint && truthPoint !== firstTruth);
      const finitePoint = (element) => element &&
        Number.isFinite(Number(element.getAttribute("cx"))) &&
        Number.isFinite(Number(element.getAttribute("cy")));
      const lineStripPoints = (element) => (element?.getAttribute("points") || "")
        .trim().split(/\s+/).filter(Boolean).length;
      const sceneRect = scene?.getBoundingClientRect();
      const sceneVisible = Boolean(sceneRect && sceneRect.width > 0 && sceneRect.height > 0);
      const outputPresentation = Boolean(
        host?.dataset.mechPresentation === "output" &&
        host?.dataset.mechPresentationView === "output" &&
        host?.dataset.mechOutputFullscreenActive === "true" &&
        outputToggle?.getAttribute("aria-pressed") === "true" &&
        outputPanel?.hidden === false &&
        tabs === "output,console,errors"
      );

      root.dataset.mechObservedUpdates = String(updates);
      if (
        updates >= 160 &&
        cameras.length === 4 &&
        finitePoint(truth) && finitePoint(estimate) && finitePoint(prediction) &&
        truthMoved &&
        lineStripPoints(covariance) >= 48 &&
        lineStripPoints(truthPath) >= 64 &&
        lineStripPoints(estimatePath) >= 64 &&
        title?.textContent === "Camera EKF Localization" &&
        sceneVisible && outputPresentation
      ) {
        root.dataset.mechUpdates = String(updates);
        root.dataset.mechCameras = String(cameras.length);
        root.dataset.mechTruthMoved = String(truthMoved);
        root.dataset.mechCovariancePoints = String(lineStripPoints(covariance));
        root.dataset.mechTruthPathPoints = String(lineStripPoints(truthPath));
        root.dataset.mechEstimatePathPoints = String(lineStripPoints(estimatePath));
        root.dataset.mechSceneVisible = String(sceneVisible);
        root.dataset.mechOutputPresentation = String(outputPresentation);
        root.dataset.mechDone = "true";
        globalThis.__MECH_STOP__?.();
        return;
      }
      if (Date.now() >= deadline) {
        root.dataset.mechTimedOut = "true";
        globalThis.__MECH_STOP__?.();
      }
    }, 16);
  </script>'''
if html.count(marker) != 1:
    raise SystemExit("could not find the head boundary in generated EKF HTML")
path.write_text(html.replace(marker, harness + "\n  " + marker, 1))
PY

server_ready_deadline=$((SECONDS + server_ready_timeout_seconds))
while ((SECONDS < server_ready_deadline)); do
  curl --fail --silent "$page_url" >"$browser_dir/preflight.html" || true
  grep -q 'root.dataset.mechDone' "$browser_dir/preflight.html" && break
  sleep 0.1
done
if ! grep -q 'root.dataset.mechDone' "$browser_dir/preflight.html"; then
  echo "Resident EKF server did not load the generated test document" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  exit 1
fi

set +e
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
            dom_bytes = Path(dom_file).read_bytes()
            proof_emitted = (
                b'data-mech-done="true"' in dom_bytes
                or b'data-mech-timed-out="true"' in dom_bytes
            )
        except OSError:
            proof_emitted = False
        if proof_emitted:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            raise SystemExit(124)
        if time.monotonic() >= deadline:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            raise SystemExit(124)
        time.sleep(0.25)
PY
chrome_status="$?"
set -e

updates="$(sed -n 's/.*data-mech-updates="\([0-9][0-9]*\)".*/\1/p' "$dom_file" | head -1)"
if [[ "$chrome_status" -ne 0 && "$chrome_status" -ne 124 ]] \
  || ! grep -q 'data-mech-done="true"' "$dom_file" \
  || ! grep -q 'data-mech-cameras="4"' "$dom_file" \
  || ! grep -q 'data-mech-truth-moved="true"' "$dom_file" \
  || ! grep -q 'data-mech-covariance-points="[4-9][0-9]' "$dom_file" \
  || ! grep -q 'data-mech-scene-visible="true"' "$dom_file" \
  || ! grep -q 'data-mech-output-presentation="true"' "$dom_file" \
  || [[ -z "$updates" || "$updates" -lt 160 ]] \
  || grep -qE 'data-mech-(console-error|page-error|timed-out)=' "$dom_file"; then
  echo "Served resident EKF browser smoke test failed" >&2
  echo "Server log:" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  echo "Chrome stderr:" >&2
  sed -n '1,240p' "$chrome_log" >&2 || true
  echo "Dumped DOM:" >&2
  sed -n '1,420p' "$dom_file" >&2 || true
  exit 1
fi

printf 'EKF_E2E display_updates=%s cameras=4 truth_moved=true covariance=true paths=true output_presentation=true console_errors=0 page_errors=0\n' "$updates"
