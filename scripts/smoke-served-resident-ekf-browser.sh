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
server_ready_timeout_seconds="${MECH_BROWSER_SERVER_READY_TIMEOUT_SECONDS:-120}"
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
    const deadline = Date.now() + 30000;
    let firstTruth;
    let previousTruth;
    let departedStart = false;
    const squareSides = new Set();
    const truthBounds = {minX: Infinity, maxX: -Infinity, minY: Infinity, maxY: -Infinity};
    let cornerDistances = [];
    window.requestAnimationFrame = (callback) => originalSetTimeout(() => {
      if (root.dataset.mechDone === "true" || root.dataset.mechTimedOut === "true") return;
      callback(performance.now());

      const display = document.querySelector('[data-mech-display-id="scene-localization"]');
      const scene = document.querySelector('[data-mech-rich-scene="true"]');
      const truth = document.querySelector('[data-mech-scene-id="truth"]');
      const estimate = document.querySelector('[data-mech-scene-id="estimate"]');
      const prediction = document.querySelector('[data-mech-scene-id="prediction"]');
      const covariance = document.querySelector('[data-mech-scene-id="covariance"]');
      const squareGuide = document.querySelector('[data-mech-scene-id="square-guide"]');
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
      const finitePoint = (element) => element &&
        Number.isFinite(Number(element.getAttribute("cx"))) &&
        Number.isFinite(Number(element.getAttribute("cy")));
      const truthPoint = finitePoint(truth)
        ? {x: Number(truth.getAttribute("cx")), y: Number(truth.getAttribute("cy"))}
        : undefined;
      if (truthPoint && firstTruth === undefined) firstTruth = truthPoint;
      if (truthPoint && previousTruth) {
        const dx = truthPoint.x - previousTruth.x;
        const dy = truthPoint.y - previousTruth.y;
        if (dx > 0.5) squareSides.add("east");
        if (dy < -0.5) squareSides.add("north");
        if (dx < -0.5) squareSides.add("west");
        if (dy > 0.5) squareSides.add("south");
      }
      if (truthPoint) previousTruth = truthPoint;
      const distanceFromStart = firstTruth && truthPoint
        ? Math.hypot(truthPoint.x - firstTruth.x, truthPoint.y - firstTruth.y)
        : 0;
      departedStart ||= distanceFromStart > 100;
      const truthMoved = Boolean(departedStart);
      const lapComplete = Boolean(
        departedStart && squareSides.size === 4 && distanceFromStart <= 12
      );
      const trackingError = finitePoint(truth) && finitePoint(estimate)
        ? Math.hypot(
            Number(estimate.getAttribute("cx")) - Number(truth.getAttribute("cx")),
            Number(estimate.getAttribute("cy")) - Number(truth.getAttribute("cy")),
          )
        : Number.POSITIVE_INFINITY;
      const lineStripCoordinates = (element) => {
        const coordinates = (element?.getAttribute("points") || "")
          .trim().split(/[\s,]+/).filter(Boolean).map(Number);
        return coordinates.length >= 4 &&
          coordinates.length % 2 === 0 &&
          coordinates.every(Number.isFinite) ? coordinates : [];
      };
      const lineStripGeometry = (element) => {
        const coordinates = lineStripCoordinates(element);
        if (coordinates.length === 0) return {finite: false, points: 0, extent: 0};
        const xs = coordinates.filter((_, index) => index % 2 === 0);
        const ys = coordinates.filter((_, index) => index % 2 === 1);
        const extent = Math.max(
          Math.max(...xs) - Math.min(...xs),
          Math.max(...ys) - Math.min(...ys),
        );
        return {finite: true, points: coordinates.length / 2, extent};
      };
      const covarianceGeometry = lineStripGeometry(covariance);
      const truthPathGeometry = lineStripGeometry(truthPath);
      const estimatePathGeometry = lineStripGeometry(estimatePath);
      const truthPathCoordinates = lineStripCoordinates(truthPath);
      const guideCoordinates = lineStripCoordinates(squareGuide);
      const guideCorners = [];
      for (let index = 0; index + 1 < guideCoordinates.length; index += 2) {
        const point = {x: guideCoordinates[index], y: guideCoordinates[index + 1]};
        if (!guideCorners.some(corner => corner.x === point.x && corner.y === point.y)) {
          guideCorners.push(point);
        }
      }
      const observedTruthPoints = [];
      if (truthPoint) observedTruthPoints.push(truthPoint);
      for (let index = 0; index + 1 < truthPathCoordinates.length; index += 2) {
        observedTruthPoints.push({
          x: truthPathCoordinates[index],
          y: truthPathCoordinates[index + 1],
        });
      }
      for (const point of observedTruthPoints) {
        truthBounds.minX = Math.min(truthBounds.minX, point.x);
        truthBounds.maxX = Math.max(truthBounds.maxX, point.x);
        truthBounds.minY = Math.min(truthBounds.minY, point.y);
        truthBounds.maxY = Math.max(truthBounds.maxY, point.y);
      }
      if (guideCorners.length === 4) {
        if (cornerDistances.length !== 4) cornerDistances = Array(4).fill(Infinity);
        guideCorners.forEach((corner, cornerIndex) => {
          for (const point of observedTruthPoints) {
            cornerDistances[cornerIndex] = Math.min(
              cornerDistances[cornerIndex],
              Math.hypot(point.x - corner.x, point.y - corner.y),
            );
          }
        });
      }
      const guideXs = guideCorners.map(corner => corner.x);
      const guideYs = guideCorners.map(corner => corner.y);
      const guideBounds = guideCorners.length === 4 ? {
        minX: Math.min(...guideXs), maxX: Math.max(...guideXs),
        minY: Math.min(...guideYs), maxY: Math.max(...guideYs),
      } : undefined;
      const boundError = guideBounds ? Math.max(
        Math.abs(truthBounds.minX - guideBounds.minX),
        Math.abs(truthBounds.maxX - guideBounds.maxX),
        Math.abs(truthBounds.minY - guideBounds.minY),
        Math.abs(truthBounds.maxY - guideBounds.maxY),
      ) : Infinity;
      const squareBoundsExact = guideCorners.length === 4 &&
        cornerDistances.length === 4 && cornerDistances.every(distance => distance <= 1.5) &&
        boundError <= 1.5;
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
      root.dataset.mechObservedSquareSides = ["east", "north", "west", "south"]
        .filter(side => squareSides.has(side)).join(",");
      root.dataset.mechObservedDistanceFromStart = distanceFromStart.toFixed(4);
      root.dataset.mechObservedLapComplete = String(lapComplete);
      root.dataset.mechObservedTrackingErrorPixels = trackingError.toFixed(4);
      root.dataset.mechObservedCovariance = JSON.stringify(covarianceGeometry);
      root.dataset.mechObservedTruthPath = JSON.stringify(truthPathGeometry);
      root.dataset.mechObservedEstimatePath = JSON.stringify(estimatePathGeometry);
      root.dataset.mechObservedTruthBounds = JSON.stringify(truthBounds);
      root.dataset.mechObservedCornerDistances = JSON.stringify(cornerDistances);
      root.dataset.mechObservedSquareBoundError = boundError.toFixed(4);
      root.dataset.mechObservedSquareBoundsExact = String(squareBoundsExact);
      root.dataset.mechObservedOutputPresentation = String(outputPresentation);
      if (
        updates >= 320 &&
        cameras.length === 4 &&
        finitePoint(truth) && finitePoint(estimate) && finitePoint(prediction) &&
        trackingError <= 25 &&
        truthMoved && lapComplete && squareBoundsExact &&
        covarianceGeometry.finite && covarianceGeometry.points >= 48 &&
        covarianceGeometry.extent > 0.1 &&
        truthPathGeometry.finite && truthPathGeometry.points >= 64 &&
        truthPathGeometry.extent > 100 &&
        estimatePathGeometry.finite && estimatePathGeometry.points >= 64 &&
        estimatePathGeometry.extent > 100 &&
        title?.textContent === "Camera EKF Localization" &&
        sceneVisible && outputPresentation
      ) {
        root.dataset.mechUpdates = String(updates);
        root.dataset.mechCameras = String(cameras.length);
        root.dataset.mechTruthMoved = String(truthMoved);
        root.dataset.mechSquareSides = ["east", "north", "west", "south"]
          .filter(side => squareSides.has(side)).join(",");
        root.dataset.mechLapComplete = String(lapComplete);
        root.dataset.mechSquareBoundsExact = String(squareBoundsExact);
        root.dataset.mechSquareBoundError = boundError.toFixed(4);
        root.dataset.mechCovariancePoints = String(covarianceGeometry.points);
        root.dataset.mechCovarianceFinite = String(covarianceGeometry.finite);
        root.dataset.mechTruthPathPoints = String(truthPathGeometry.points);
        root.dataset.mechTruthPathFinite = String(truthPathGeometry.finite);
        root.dataset.mechEstimatePathPoints = String(estimatePathGeometry.points);
        root.dataset.mechEstimatePathFinite = String(estimatePathGeometry.finite);
        root.dataset.mechTrackingErrorPixels = trackingError.toFixed(4);
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
    "--virtual-time-budget=32000",
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
  || ! grep -q 'data-mech-square-sides="east,north,west,south"' "$dom_file" \
  || ! grep -q 'data-mech-lap-complete="true"' "$dom_file" \
  || ! grep -q 'data-mech-square-bounds-exact="true"' "$dom_file" \
  || ! grep -q 'data-mech-covariance-points="[4-9][0-9]' "$dom_file" \
  || ! grep -q 'data-mech-covariance-finite="true"' "$dom_file" \
  || ! grep -q 'data-mech-truth-path-finite="true"' "$dom_file" \
  || ! grep -q 'data-mech-estimate-path-finite="true"' "$dom_file" \
  || ! grep -q 'data-mech-scene-visible="true"' "$dom_file" \
  || ! grep -q 'data-mech-output-presentation="true"' "$dom_file" \
  || ! grep -q 'data-mech-tracking-error-pixels="[0-9]' "$dom_file" \
  || [[ -z "$updates" || "$updates" -lt 320 ]] \
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

tracking_error_pixels="$(sed -n 's/.*data-mech-tracking-error-pixels="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
square_bound_error="$(sed -n 's/.*data-mech-square-bound-error="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
printf 'EKF_E2E display_updates=%s cameras=4 square_sides=4 lap_complete=true square_bounds_exact=true square_bound_error_pixels=%s truth_moved=true covariance_finite=true paths_finite=true tracking_error_pixels=%s output_presentation=true console_errors=0 page_errors=0\n' "$updates" "$square_bound_error" "$tracking_error_pixels"
