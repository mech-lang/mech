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

compute_backend="${MECH_EKF_COMPUTE_BACKEND:-cpu-scalar}"
case "$compute_backend" in
  cpu-scalar|wgpu) expected_compute_backend="$compute_backend" ;;
  auto) expected_compute_backend="cpu-scalar" ;;
  *)
    echo "MECH_EKF_COMPUTE_BACKEND must be auto, cpu-scalar, or wgpu" >&2
    exit 1
    ;;
esac
python3 - "$project_dir/mech.mcfg" "$compute_backend" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
backend = sys.argv[2]
source = path.read_text()
needle = 'backend: "auto"'
if source.count(needle) != 1:
    raise SystemExit("could not select the EKF browser smoke compute backend")
path.write_text(source.replace(needle, f'backend: "{backend}"', 1))
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

python3 - "$project_dir/index.html" "$expected_compute_backend" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
compute_backend = sys.argv[2]
html = path.read_text()
marker = "</head>"
harness = r'''<script>
    const root = document.documentElement;
    const expectedComputeBackend = "__MECH_EXPECTED_COMPUTE_BACKEND__";
    const originalConsoleError = console.error;
    const diagnosticText = (value) => value?.stack || value?.message || String(value);
    console.error = (...args) => {
      root.dataset.mechConsoleError = args.map(diagnosticText).join(" ");
      originalConsoleError.apply(console, args);
    };
    window.addEventListener("error", (event) => {
      root.dataset.mechPageError = diagnosticText(event.error || event.message);
    });
    window.addEventListener("unhandledrejection", (event) => {
      root.dataset.mechPageError = diagnosticText(event.reason);
    });

    const originalSetTimeout = window.setTimeout.bind(window);
    let harnessFrames = 0;
    let firstTruth;
    let previousTruth;
    let previousHeading;
    let lastObservedUpdate = -1;
    let lastObservedComputeDispatch = 0;
    let departedStart = false;
    const squareSides = new Set();
    let turningSamples = 0;
    let curvedMotionSamples = 0;
    let maxHeadingStep = 0;
    let maxGuideDeviation = 0;
    let observedLapComplete = false;
    let sawNoCamera = false;
    let sawCamera = false;
    let maxVisibleCameras = 0;
    let previousVisibleCameraCount;
    const visibleCameraHistory = [];
    let stableCameraGeometry;
    let cameraGeometryStable = true;
    let cameraRangeOracleValid = true;
    let predictionOnlyValid = true;
    let predictionOnlyComparisons = 0;
    let predictionOnlyFailure;
    let sawInsideCameraRange = false;
    let sawOutsideCameraRange = false;
    window.requestAnimationFrame = (callback) => originalSetTimeout(() => {
      if (root.dataset.mechDone === "true" || root.dataset.mechTimedOut === "true") return;
      harnessFrames += 1;
      callback(performance.now());

      const display = document.querySelector('[data-mech-display-id="scene-localization"]');
      const scene = document.querySelector('[data-mech-rich-scene="true"]');
      const truth = document.querySelector('[data-mech-scene-id="truth"]');
      const estimate = document.querySelector('[data-mech-scene-id="estimate"]');
      const prediction = document.querySelector('[data-mech-scene-id="prediction"]');
      const covariance = document.querySelector('[data-mech-scene-id="covariance"]');
      const truthPath = document.querySelector('[data-mech-scene-id="truth-path"]');
      const estimatePath = document.querySelector('[data-mech-scene-id="estimate-path"]');
      const truthHeading = document.querySelector('[data-mech-scene-id="truth-heading"]');
      const squareGuide = document.querySelector('[data-mech-scene-id="square-guide"]');
      const title = document.querySelector('[data-mech-scene-id="title"]');
      const cameras = [...document.querySelectorAll('[data-mech-scene-id^="camera-"]')]
        .filter(element => /^camera-[1-4]$/.test(element.dataset.mechSceneId || ""));
      const cameraRanges = [...document.querySelectorAll('[data-mech-scene-id^="camera-range-"]')];
      const cameraRays = [...document.querySelectorAll('[data-mech-scene-id^="ray-"]')];
      const host = document.querySelector('[data-mech-repl-host]');
      const outputPanel = document.querySelector('[data-mech-console-panel="output"]');
      const errorPanel = document.querySelector('[data-mech-console-panel="errors"]');
      const outputToggle = document.querySelector('button[data-mech-output-fullscreen]');
      const tabs = [...document.querySelectorAll('[data-mech-console-tab]')]
        .map(tab => tab.dataset.mechConsoleTab).join(",");
      const updates = Number(display?.dataset.mechDisplayUpdates || 0);
      const finiteCoordinate = (element, attribute) => {
        const raw = element?.getAttribute(attribute);
        return typeof raw === "string" && raw.trim() !== "" && Number.isFinite(Number(raw));
      };
      const finitePoint = (element) =>
        finiteCoordinate(element, "cx") && finiteCoordinate(element, "cy");
      const finitePointContract = (() => {
        const probe = document.createElementNS("http://www.w3.org/2000/svg", "circle");
        const rejectsMissing = !finitePoint(probe);
        probe.setAttribute("cx", " ");
        probe.setAttribute("cy", "2");
        const rejectsEmpty = !finitePoint(probe);
        probe.setAttribute("cx", "1");
        const acceptsFinite = finitePoint(probe);
        return rejectsMissing && rejectsEmpty && acceptsFinite;
      })();
      const truthPoint = finitePoint(truth)
        ? {x: Number(truth.getAttribute("cx")), y: Number(truth.getAttribute("cy"))}
        : undefined;
      const guideCoordinates = (squareGuide?.getAttribute("points") || "")
        .trim().split(/[\s,]+/).filter(Boolean).map(Number);
      const distanceToSegment = (point, ax, ay, bx, by) => {
        const dx = bx - ax;
        const dy = by - ay;
        const lengthSquared = dx * dx + dy * dy;
        const fraction = lengthSquared === 0 ? 0 : Math.max(0, Math.min(1,
          ((point.x - ax) * dx + (point.y - ay) * dy) / lengthSquared,
        ));
        return Math.hypot(point.x - (ax + fraction * dx), point.y - (ay + fraction * dy));
      };
      const distanceToRenderedGuide = (point) => {
        if (guideCoordinates.length < 8 || guideCoordinates.length % 2 !== 0 ||
            !guideCoordinates.every(Number.isFinite)) return Number.POSITIVE_INFINITY;
        let nearest = Number.POSITIVE_INFINITY;
        const pointCount = guideCoordinates.length / 2;
        for (let index = 0; index < pointCount; index += 1) {
          const next = (index + 1) % pointCount;
          nearest = Math.min(nearest, distanceToSegment(
            point,
            guideCoordinates[index * 2],
            guideCoordinates[index * 2 + 1],
            guideCoordinates[next * 2],
            guideCoordinates[next * 2 + 1],
          ));
        }
        return nearest;
      };
      const heading = truthHeading
        ? Math.atan2(
            -(Number(truthHeading.getAttribute("y2")) - Number(truthHeading.getAttribute("y1"))),
            Number(truthHeading.getAttribute("x2")) - Number(truthHeading.getAttribute("x1")),
          )
        : undefined;
      if (!truthPoint && updates > 0 && updates !== lastObservedUpdate) {
        lastObservedUpdate = updates;
        cameraGeometryStable = false;
        cameraRangeOracleValid = false;
        predictionOnlyValid = false;
      }
      if (truthPoint && updates > 0 && updates !== lastObservedUpdate) {
        lastObservedUpdate = updates;
        const computeDispatches = Number(root.dataset.mechComputeDispatches || 0);
        const isComputeCompletionFrame = root.dataset.mechComputeBackend === "wgpu"
          ? computeDispatches > lastObservedComputeDispatch
          : Boolean(
              previousTruth &&
              Math.hypot(truthPoint.x - previousTruth.x, truthPoint.y - previousTruth.y) <= 1e-9
            );
        lastObservedComputeDispatch = computeDispatches;
        if (firstTruth === undefined) firstTruth = truthPoint;
        maxGuideDeviation = Math.max(maxGuideDeviation, distanceToRenderedGuide(truthPoint));
        const indexed = (elements, prefix) => [...elements].sort((left, right) =>
          Number((left.dataset.mechSceneId || "").slice(prefix.length)) -
          Number((right.dataset.mechSceneId || "").slice(prefix.length))
        );
        const orderedCameras = indexed(cameras, "camera-");
        const orderedRanges = indexed(cameraRanges, "camera-range-");
        const orderedRays = indexed(cameraRays, "ray-");
        const currentCameraGeometry = orderedCameras.map((camera, index) => {
          const range = orderedRanges[index];
          const ray = orderedRays[index];
          const x = Number(camera?.getAttribute("cx"));
          const y = Number(camera?.getAttribute("cy"));
          const radius = Number(camera?.getAttribute("r"));
          const opacity = Number(camera?.getAttribute("opacity"));
          const rangeX = Number(range?.getAttribute("cx"));
          const rangeY = Number(range?.getAttribute("cy"));
          const rangeRadius = Number(range?.getAttribute("r"));
          const rayX = Number(ray?.getAttribute("x1"));
          const rayY = Number(ray?.getAttribute("y1"));
          return {x, y, radius, opacity, rangeX, rangeY, rangeRadius, rayX, rayY};
        });
        if (stableCameraGeometry === undefined) {
          stableCameraGeometry = currentCameraGeometry;
        } else {
          cameraGeometryStable &&= JSON.stringify(currentCameraGeometry) ===
            JSON.stringify(stableCameraGeometry);
        }
        cameraGeometryStable &&= currentCameraGeometry.length === 4 &&
          currentCameraGeometry.every(sensor =>
            Object.values(sensor).every(Number.isFinite) &&
            sensor.radius === 9 && sensor.opacity === 1 &&
            sensor.x === sensor.rangeX && sensor.y === sensor.rangeY &&
            sensor.x === sensor.rayX && sensor.y === sensor.rayY
          );
        let visibleCameraCount = 0;
        for (let index = 0; index < currentCameraGeometry.length; index += 1) {
          const sensor = currentCameraGeometry[index];
          const rayOpacity = Number(orderedRays[index]?.getAttribute("opacity"));
          const distance = Math.hypot(truthPoint.x - sensor.x, truthPoint.y - sensor.y);
          const fadeWidth = sensor.rangeRadius * (0.18 / 3.6);
          const expectedVisibility = Math.max(0, Math.min(1,
            (sensor.rangeRadius - distance) / fadeWidth,
          ));
          const expectedRayOpacity = 0.4 * expectedVisibility;
          cameraRangeOracleValid &&= Number.isFinite(rayOpacity) &&
            Math.abs(rayOpacity - expectedRayOpacity) <= 1e-9;
          if (distance >= sensor.rangeRadius) {
            sawOutsideCameraRange = true;
            cameraRangeOracleValid &&= rayOpacity === 0;
          } else {
            sawInsideCameraRange = true;
          }
          if (rayOpacity > 0) visibleCameraCount += 1;
        }
        sawNoCamera ||= visibleCameraCount === 0;
        sawCamera ||= visibleCameraCount > 0;
        maxVisibleCameras = Math.max(maxVisibleCameras, visibleCameraCount);
        // WebGPU completion is consumed by the next resident turn, so its
        // sampled estimate corresponds to the preceding turn's measurement.
        // The scalar backend completes inside the current turn.
        const filterVisibleCameraCount = root.dataset.mechComputeBackend === "wgpu"
          ? previousVisibleCameraCount
          : visibleCameraCount;
        if (filterVisibleCameraCount === 0 && isComputeCompletionFrame) {
          const predictionGap = finitePoint(estimate) && finitePoint(prediction)
            ? Math.hypot(
                Number(estimate.getAttribute("cx")) - Number(prediction.getAttribute("cx")),
                Number(estimate.getAttribute("cy")) - Number(prediction.getAttribute("cy")),
              )
            : Number.POSITIVE_INFINITY;
          // The WGPU sample is consumed asynchronously, one resident output
          // behind the live ray geometry. At reacquisition boundaries permit
          // only a subpixel remnant; sustained dead-zone samples coincide.
          const comparisonValid = predictionGap <= 0.25;
          predictionOnlyValid &&= Boolean(comparisonValid);
          if (comparisonValid) predictionOnlyComparisons += 1;
          if (!comparisonValid && predictionOnlyFailure === undefined) {
            predictionOnlyFailure = {
              estimateX: estimate?.getAttribute("cx"),
              estimateY: estimate?.getAttribute("cy"),
              predictionX: prediction?.getAttribute("cx"),
              predictionY: prediction?.getAttribute("cy"),
              currentVisibleCameraCount: visibleCameraCount,
              previousVisibleCameraCount,
              visibleCameraHistory: visibleCameraHistory.slice(-8),
              computeDispatches,
              predictionGap,
            };
          }
        }
        previousVisibleCameraCount = visibleCameraCount;
        visibleCameraHistory.push(visibleCameraCount);
      }
      if (truthPoint && previousTruth && updates === lastObservedUpdate) {
        const dx = truthPoint.x - previousTruth.x;
        const dy = truthPoint.y - previousTruth.y;
        if (dx > 0.5) squareSides.add("east");
        if (dy < -0.5) squareSides.add("north");
        if (dx < -0.5) squareSides.add("west");
        if (dy > 0.5) squareSides.add("south");
        if (Math.abs(dx) > 0.2 && Math.abs(dy) > 0.2) curvedMotionSamples += 1;
        if (Number.isFinite(heading) && Number.isFinite(previousHeading)) {
          const headingStep = Math.abs(Math.atan2(
            Math.sin(heading - previousHeading),
            Math.cos(heading - previousHeading),
          ));
          maxHeadingStep = Math.max(maxHeadingStep, headingStep);
          if (headingStep > 0.005) turningSamples += 1;
        }
      }
      if (truthPoint && updates === lastObservedUpdate) {
        previousTruth = truthPoint;
        if (Number.isFinite(heading)) previousHeading = heading;
      }
      const distanceFromStart = firstTruth && truthPoint
        ? Math.hypot(truthPoint.x - firstTruth.x, truthPoint.y - firstTruth.y)
        : 0;
      departedStart ||= distanceFromStart > 100;
      const truthMoved = Boolean(departedStart);
      observedLapComplete ||= Boolean(
        departedStart && squareSides.size === 4 && updates >= 340 && distanceFromStart <= 60
      );
      const smoothTurning = turningSamples >= 20 && curvedMotionSamples >= 12 &&
        maxHeadingStep > 0 && maxHeadingStep < 0.35;
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
      const computeBackend = root.dataset.mechComputeBackend || "";
      const computeInstances = Number(root.dataset.mechComputeInstances || 0);

      root.dataset.mechObservedUpdates = String(updates);
      root.dataset.mechObservedSquareSides = ["east", "north", "west", "south"]
        .filter(side => squareSides.has(side)).join(",");
      root.dataset.mechObservedDistanceFromStart = distanceFromStart.toFixed(4);
      root.dataset.mechObservedLapComplete = String(observedLapComplete);
      root.dataset.mechObservedTrackingErrorPixels = trackingError.toFixed(4);
      root.dataset.mechObservedCovariance = JSON.stringify(covarianceGeometry);
      root.dataset.mechObservedTruthPath = JSON.stringify(truthPathGeometry);
      root.dataset.mechObservedEstimatePath = JSON.stringify(estimatePathGeometry);
      root.dataset.mechObservedTurningSamples = String(turningSamples);
      root.dataset.mechObservedCurvedMotionSamples = String(curvedMotionSamples);
      root.dataset.mechObservedMaxHeadingStep = maxHeadingStep.toFixed(6);
      root.dataset.mechObservedMaxGuideDeviation = maxGuideDeviation.toFixed(4);
      root.dataset.mechObservedSawNoCamera = String(sawNoCamera);
      root.dataset.mechObservedSawCamera = String(sawCamera);
      root.dataset.mechObservedMaxVisibleCameras = String(maxVisibleCameras);
      root.dataset.mechObservedCameraGeometryStable = String(cameraGeometryStable);
      root.dataset.mechObservedCameraRangeOracle = String(cameraRangeOracleValid);
      root.dataset.mechObservedPredictionOnly = String(predictionOnlyValid);
      root.dataset.mechObservedPredictionOnlyComparisons = String(predictionOnlyComparisons);
      root.dataset.mechObservedFinitePointContract = String(finitePointContract);
      root.dataset.mechObservedPredictionOnlyFailure = JSON.stringify(predictionOnlyFailure || null);
      root.dataset.mechObservedSmoothTurning = String(smoothTurning);
      root.dataset.mechObservedOutputPresentation = String(outputPresentation);
      root.dataset.mechObservedErrorText = (errorPanel?.textContent || "").trim().slice(0, 1000);
      root.dataset.mechObservedDisplayIds = [...document.querySelectorAll('[data-mech-display-id]')]
        .map(element => element.dataset.mechDisplayId).join(",");
      if (
        updates >= 376 &&
        finitePointContract &&
        cameras.length === 4 && cameraRanges.length === 4 && cameraRays.length === 4 &&
        cameraGeometryStable && cameraRangeOracleValid && predictionOnlyValid &&
        predictionOnlyComparisons > 0 &&
        sawInsideCameraRange && sawOutsideCameraRange &&
        sawNoCamera && sawCamera && maxVisibleCameras === 1 &&
        finitePoint(truth) && finitePoint(estimate) && finitePoint(prediction) &&
        trackingError <= 25 &&
        // The rounded v/ω-controlled corners intentionally leave the square's
        // one-pixel centerline, but every sample must remain inside the guide's
        // 50-pixel cornering corridor. A diagonal cut is roughly 135 pixels
        // from the nearest rendered side and therefore cannot satisfy this.
        truthMoved && observedLapComplete && smoothTurning && maxGuideDeviation <= 50 &&
        covarianceGeometry.finite && covarianceGeometry.points >= 48 &&
        covarianceGeometry.extent >= 5 &&
        truthPathGeometry.finite && truthPathGeometry.points >= 376 &&
        truthPathGeometry.extent > 100 &&
        estimatePathGeometry.finite && estimatePathGeometry.points >= 376 &&
        estimatePathGeometry.extent > 100 &&
        title?.textContent === "Camera EKF Localization" &&
        sceneVisible && outputPresentation &&
        computeBackend === expectedComputeBackend && computeInstances === 1000
      ) {
        root.dataset.mechUpdates = String(updates);
        root.dataset.mechCameras = String(cameras.length);
        root.dataset.mechCameraRanges = String(cameraRanges.length);
        root.dataset.mechSawNoCamera = String(sawNoCamera);
        root.dataset.mechSawCamera = String(sawCamera);
        root.dataset.mechMaxVisibleCameras = String(maxVisibleCameras);
        root.dataset.mechCameraGeometryStable = String(cameraGeometryStable);
        root.dataset.mechCameraRangeOracle = String(cameraRangeOracleValid);
        root.dataset.mechPredictionOnly = String(predictionOnlyValid);
        root.dataset.mechPredictionOnlyComparisons = String(predictionOnlyComparisons);
        root.dataset.mechFinitePointContract = String(finitePointContract);
        root.dataset.mechTruthMoved = String(truthMoved);
        root.dataset.mechSquareSides = ["east", "north", "west", "south"]
          .filter(side => squareSides.has(side)).join(",");
        root.dataset.mechLapComplete = String(observedLapComplete);
        root.dataset.mechSmoothTurning = String(smoothTurning);
        root.dataset.mechTurningSamples = String(turningSamples);
        root.dataset.mechCurvedMotionSamples = String(curvedMotionSamples);
        root.dataset.mechMaxHeadingStep = maxHeadingStep.toFixed(6);
        root.dataset.mechMaxGuideDeviation = maxGuideDeviation.toFixed(4);
        root.dataset.mechCovarianceExtent = covarianceGeometry.extent.toFixed(4);
        root.dataset.mechCovariancePoints = String(covarianceGeometry.points);
        root.dataset.mechCovarianceFinite = String(covarianceGeometry.finite);
        root.dataset.mechTruthPathPoints = String(truthPathGeometry.points);
        root.dataset.mechTruthPathFinite = String(truthPathGeometry.finite);
        root.dataset.mechEstimatePathPoints = String(estimatePathGeometry.points);
        root.dataset.mechEstimatePathFinite = String(estimatePathGeometry.finite);
        root.dataset.mechTrackingErrorPixels = trackingError.toFixed(4);
        root.dataset.mechSceneVisible = String(sceneVisible);
        root.dataset.mechOutputPresentation = String(outputPresentation);
        root.dataset.mechVerifiedComputeBackend = computeBackend;
        root.dataset.mechVerifiedComputeInstances = String(computeInstances);
        root.dataset.mechDone = "true";
        globalThis.__MECH_STOP__?.();
        return;
      }
      if (harnessFrames >= 5000) {
        root.dataset.mechTimedOut = "true";
        globalThis.__MECH_STOP__?.();
      }
    }, 16);
  </script>'''
if html.count(marker) != 1:
    raise SystemExit("could not find the head boundary in generated EKF HTML")
harness = harness.replace("__MECH_EXPECTED_COMPUTE_BACKEND__", compute_backend)
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
python3 - "$chrome_bin" "$page_url" "$chrome_profile" "$dom_file" "$chrome_log" "$compute_backend" <<'PY'
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

chrome, page_url, profile, dom_file, chrome_log, compute_backend = sys.argv[1:]
args = [
    chrome,
    "--headless=new",
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--run-all-compositor-stages-before-draw",
    "--virtual-time-budget=120000",
    "--dump-dom",
    f"--user-data-dir={profile}",
    page_url,
]
if compute_backend != "wgpu":
    args.insert(-1, "--disable-gpu")
else:
    args.insert(-1, "--enable-unsafe-webgpu")
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
                or b'data-mech-console-error=' in dom_bytes
                or b'data-mech-page-error=' in dom_bytes
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
  || ! grep -q 'data-mech-camera-ranges="4"' "$dom_file" \
  || ! grep -q 'data-mech-saw-no-camera="true"' "$dom_file" \
  || ! grep -q 'data-mech-saw-camera="true"' "$dom_file" \
  || ! grep -q 'data-mech-max-visible-cameras="1"' "$dom_file" \
  || ! grep -q 'data-mech-camera-geometry-stable="true"' "$dom_file" \
  || ! grep -q 'data-mech-camera-range-oracle="true"' "$dom_file" \
  || ! grep -q 'data-mech-prediction-only="true"' "$dom_file" \
  || ! grep -qE 'data-mech-prediction-only-comparisons="[1-9][0-9]*"' "$dom_file" \
  || ! grep -q 'data-mech-finite-point-contract="true"' "$dom_file" \
  || ! grep -q 'data-mech-truth-moved="true"' "$dom_file" \
  || ! grep -q 'data-mech-square-sides="east,north,west,south"' "$dom_file" \
  || ! grep -q 'data-mech-lap-complete="true"' "$dom_file" \
  || ! grep -q 'data-mech-smooth-turning="true"' "$dom_file" \
  || ! grep -qE 'data-mech-turning-samples="([2-9][0-9]|[1-9][0-9]{2,})"' "$dom_file" \
  || ! grep -qE 'data-mech-curved-motion-samples="([1-9][0-9]|[1-9][0-9]{2,})"' "$dom_file" \
  || ! grep -q 'data-mech-max-heading-step="0\.' "$dom_file" \
  || ! grep -q 'data-mech-max-guide-deviation="[0-9]' "$dom_file" \
  || ! grep -q 'data-mech-covariance-points="[4-9][0-9]' "$dom_file" \
  || ! grep -q 'data-mech-covariance-finite="true"' "$dom_file" \
  || ! grep -q 'data-mech-covariance-extent="[1-9][0-9]' "$dom_file" \
  || ! grep -q 'data-mech-truth-path-finite="true"' "$dom_file" \
  || ! grep -q 'data-mech-estimate-path-finite="true"' "$dom_file" \
  || ! grep -q 'data-mech-scene-visible="true"' "$dom_file" \
  || ! grep -q 'data-mech-output-presentation="true"' "$dom_file" \
  || ! grep -q "data-mech-verified-compute-backend=\"$expected_compute_backend\"" "$dom_file" \
  || ! grep -q 'data-mech-verified-compute-instances="1000"' "$dom_file" \
  || ! grep -q 'data-mech-tracking-error-pixels="[0-9]' "$dom_file" \
  || [[ -z "$updates" || "$updates" -lt 376 ]] \
  || grep -qE 'data-mech-(console-error|page-error|timed-out)=' "$dom_file"; then
  echo "Served resident EKF browser smoke test failed" >&2
  python3 - "$dom_file" <<'PY' >&2 || true
from html import unescape
from pathlib import Path
import re
import sys

line = Path(sys.argv[1]).read_text(errors="replace").split("<head>", 1)[0]
for name in (
    "data-mech-document-error",
    "data-mech-console-error",
    "data-mech-page-error",
    "data-mech-observed-error-text",
    "data-mech-observed-prediction-only-failure",
):
    match = re.search(rf'{name}="([^"]*)"', line)
    if match:
        print(f"{name}: {unescape(match.group(1))}")
PY
  echo "Server log:" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  echo "Chrome stderr:" >&2
  sed -n '1,240p' "$chrome_log" >&2 || true
  echo "Dumped DOM:" >&2
  sed -n '1,420p' "$dom_file" >&2 || true
  exit 1
fi

tracking_error_pixels="$(sed -n 's/.*data-mech-tracking-error-pixels="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
turning_samples="$(sed -n 's/.*data-mech-turning-samples="\([0-9][0-9]*\)".*/\1/p' "$dom_file" | head -1)"
max_heading_step="$(sed -n 's/.*data-mech-max-heading-step="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
max_guide_deviation="$(sed -n 's/.*data-mech-max-guide-deviation="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
covariance_extent="$(sed -n 's/.*data-mech-covariance-extent="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
printf 'EKF_E2E display_updates=%s requested_compute_backend=%s compute_backend=%s compute_instances=1000 cameras=4 max_visible_cameras=1 saw_no_camera=true square_sides=4 lap_complete=true smooth_turning=true turning_samples=%s max_heading_step=%s max_guide_deviation_pixels=%s truth_moved=true covariance_finite=true covariance_extent_pixels=%s paths_finite=true tracking_error_pixels=%s output_presentation=true console_errors=0 page_errors=0\n' "$updates" "$compute_backend" "$expected_compute_backend" "$turning_samples" "$max_heading_step" "$max_guide_deviation" "$covariance_extent" "$tracking_error_pixels"
