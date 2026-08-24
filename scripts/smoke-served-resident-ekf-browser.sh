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

filter_count="${MECH_EKF_FILTER_COUNT:-1000}"
if [[ ! "$filter_count" =~ ^[1-9][0-9]*$ ]]; then
  echo "MECH_EKF_FILTER_COUNT must be a positive integer" >&2
  exit 1
fi
continuity_edit="${MECH_EKF_CONTINUITY_EDIT:-true}"
case "$continuity_edit" in
  true|false) ;;
  *)
    echo "MECH_EKF_CONTINUITY_EDIT must be true or false" >&2
    exit 1
    ;;
esac
python3 - "$project_dir/localization.mec" "$filter_count" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
filter_count = sys.argv[2]
source = path.read_text()
needle = "  filter-count := 1000.0"
if source.count(needle) != 1:
    raise SystemExit("could not select the EKF browser smoke filter count")
path.write_text(source.replace(needle, f"  filter-count := {filter_count}.0", 1))
PY

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

python3 - "$project_dir/index.html" "$expected_compute_backend" "$filter_count" \
  "$continuity_edit" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
compute_backend = sys.argv[2]
filter_count = sys.argv[3]
continuity_edit = sys.argv[4]
html = path.read_text()
marker = "</head>"
harness = r'''<script>
    const root = document.documentElement;
    const expectedComputeBackend = "__MECH_EXPECTED_COMPUTE_BACKEND__";
    const expectedComputeInstances = Number("__MECH_EXPECTED_COMPUTE_INSTANCES__");
    const performContinuityEdit = "__MECH_PERFORM_CONTINUITY_EDIT__" === "true";
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

    const parityComputeTurn = 376;
    const continuityEditTurn = 120;
    const busyReplacementTurn = 40;
    let computeParitySample;
    let continuityNextSample;
    let continuityEditRequested = false;
    let continuityGenerationChanged = false;
    let continuityResourcePreserved =
      expectedComputeBackend !== "wgpu" || !performContinuityEdit;
    let continuityActiveBufferPreserved =
      expectedComputeBackend !== "wgpu" || !performContinuityEdit;
    let continuityBefore;
    let busyReplacementAttempted = false;
    let busyReplacementRejected = expectedComputeBackend !== "wgpu";
    let busySourceUntouched = expectedComputeBackend !== "wgpu";
    let busySymbolAbsent = expectedComputeBackend !== "wgpu";
    let busyLogicalProgressUnchanged = expectedComputeBackend !== "wgpu";
    let applicationQualifiedBeforeReset = false;
    let incompatibleEditRequested = expectedComputeBackend !== "wgpu";
    let incompatibleGenerationChanged = expectedComputeBackend !== "wgpu";
    let incompatibleResourcesReplaced = expectedComputeBackend !== "wgpu";
    let incompatibleOldResourceDisposed = expectedComputeBackend !== "wgpu";
    let incompatibleStateReset = expectedComputeBackend !== "wgpu";
    let incompatibleResetDiagnostic = expectedComputeBackend !== "wgpu";
    let incompatibleBefore;
    let computeStateResetEvents = 0;
    let displayParityTrackingError;
    let computeReadbackBytes = 0;
    let sampledReadbackEfficient = true;
    const computeIdentity = () => ({
      generation: root.dataset.mechComputeGeneration || "",
      revision: root.dataset.mechComputeRevision || "",
      resource: root.dataset.mechComputeResourceIdentity || "",
      device: root.dataset.mechComputeDeviceIdentity || "",
      pipeline: root.dataset.mechComputePipelineIdentity || "",
      state: root.dataset.mechComputeStateIdentity || "",
      pipelineBuilds: root.dataset.mechComputePipelineBuildCount || "",
      activeBuffer: root.dataset.mechComputeActiveBuffer || "",
      dispatches: root.dataset.mechComputeDispatches || "",
    });
    const acceptedSource = () =>
      globalThis.__MECH_ACCEPTED_REPL_SOURCE__?.() || "";
    const submitResidentSource = (source) => {
      const input = document.querySelector(".mech-repl-active-prompt .repl-input");
      if (!input || input.disabled || input.readOnly) return false;
      input.value = source;
      input.dispatchEvent(new Event("input", { bubbles: true }));
      input.dispatchEvent(new KeyboardEvent("keydown", {
        key: "Enter",
        bubbles: true,
        cancelable: true,
      }));
      return true;
    };
    const renderedDocumentNumbers = (name) => {
      const rendered = globalThis.__MECH_RENDERED_DOCUMENT_VALUE__?.(name);
      const text = document.createElement("span");
      text.innerHTML = rendered?.inlineHtml || "";
      return (text.textContent || "")
        .match(/[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?/g)
        ?.map(Number) || [];
    };
    const renderedFilterSample = () => {
      const values = renderedDocumentNumbers("filter-sample");
      return values.length === 15 && values.every(Number.isFinite) ? values : undefined;
    };
    const renderedFilterVisibility = () => {
      const values = renderedDocumentNumbers("filter-camera-visibility");
      return values.length === 1 && Number.isFinite(values[0]) ? values[0] : undefined;
    };
    const renderedFilterTurn = () => {
      const values = renderedDocumentNumbers("last-filter-turn");
      return values.length === 1 && Number.isFinite(values[0]) ? values[0] : undefined;
    };
    window.addEventListener("mech:compute-submit", () => {
      if (
        expectedComputeBackend !== "wgpu" || busyReplacementAttempted ||
        Number(root.dataset.mechComputeDispatches || 0) !== busyReplacementTurn
      ) return;
      busyReplacementAttempted = true;
      const before = computeIdentity();
      const logicalTurnBefore = renderedFilterTurn();
      const sourceBefore = acceptedSource();
      const submitted = submitResidentSource("busy-replacement-probe := 1");
      const after = computeIdentity();
      const logicalTurnAfter = renderedFilterTurn();
      const sourceAfter = acceptedSource();
      busySourceUntouched = Boolean(
        submitted && sourceAfter === sourceBefore &&
        !sourceAfter.includes("busy-replacement-probe")
      );
      busySymbolAbsent =
        globalThis.__MECH_RENDERED_DOCUMENT_VALUE__?.("busy-replacement-probe") == null;
      busyLogicalProgressUnchanged = Boolean(
        after.generation === before.generation &&
        after.dispatches === before.dispatches &&
        after.activeBuffer === before.activeBuffer &&
        logicalTurnAfter === logicalTurnBefore
      );
      busyReplacementRejected = Boolean(
        busySourceUntouched && busySymbolAbsent && busyLogicalProgressUnchanged &&
        document.querySelector(
          '[data-mech-diagnostic-code="ComputeSourceReplacementBusy"]',
        )
      );
    });
    window.addEventListener("mech:compute-state-reset", (event) => {
      computeStateResetEvents += 1;
      const after = computeIdentity();
      incompatibleGenerationChanged = Boolean(
        incompatibleBefore && after.generation !== incompatibleBefore.generation
      );
      incompatibleResourcesReplaced = Boolean(
        incompatibleBefore && after.revision !== incompatibleBefore.revision &&
        after.resource !== incompatibleBefore.resource &&
        after.device !== incompatibleBefore.device &&
        after.pipeline !== incompatibleBefore.pipeline &&
        after.state !== incompatibleBefore.state &&
        Number(after.pipelineBuilds) === Number(incompatibleBefore.pipelineBuilds) + 1
      );
      incompatibleOldResourceDisposed = Boolean(
        incompatibleBefore &&
        event.detail?.retiredResourceIdentity === incompatibleBefore.resource &&
        event.detail?.retiredResourceDisposed === true
      );
      incompatibleStateReset = Boolean(
        after.activeBuffer === "0" && after.dispatches === "0"
      );
      incompatibleResetDiagnostic = Boolean(
        computeStateResetEvents === 1 && event.detail?.resetCount === 1 &&
        Number(root.dataset.mechComputeStateResets || 0) === 1 &&
        document.querySelectorAll(
          '[data-mech-diagnostic-code="ComputeStateReset"]',
        ).length === 1
      );
    });
    window.addEventListener("mech:compute-complete", (event) => {
      const completedTurns = event.detail?.completedTurns;
      root.dataset.mechObservedComputeCompletion = String(completedTurns ?? "missing");
      if (expectedComputeBackend === "wgpu") {
        computeReadbackBytes = Number(root.dataset.mechComputeGpuToCpuReadbackBytes || 0);
        const computeOutputBytes = Number(root.dataset.mechComputeGpuToCpuOutputBytes || 0);
        sampledReadbackEfficient &&= Boolean(
          Number(root.dataset.mechComputeLogicalOutputs || 0) === 2 &&
          Number(root.dataset.mechComputePhysicalOutputBuffers || 0) === 2 &&
          computeReadbackBytes === completedTurns * (16 * Float32Array.BYTES_PER_ELEMENT + 8) &&
          computeOutputBytes === completedTurns * 16 * Float32Array.BYTES_PER_ELEMENT
        );
      }
      if (completedTurns !== parityComputeTurn) return;
      root.dataset.mechObservedParityOutputs = (event.detail.outputs || [])
        .map(output => `${output.name}:${output.values?.length ?? "missing"}`)
        .join(",");
      const output = event.detail.outputs?.find(candidate => candidate.name === "result.0");
      const values = Array.from(output?.values || []);
      // Both backends publish the first 15-value lane used by the document;
      // the full 1,000-filter batch stays backend-local on every turn.
      if (
        event.detail?.backend === expectedComputeBackend &&
        values.length === 15 &&
        values.every(Number.isFinite)
      ) {
        computeParitySample = values.slice(0, 15);
      }
    });
    window.addEventListener("mech:compute-complete", (event) => {
      const completedTurns = event.detail?.completedTurns;
      if (
        performContinuityEdit && completedTurns === continuityEditTurn &&
        !continuityEditRequested
      ) {
        continuityBefore = computeIdentity();
        continuityEditRequested = submitResidentSource("continuity-probe := 1");
        return;
      }
      if (
        (continuityEditRequested || !performContinuityEdit) &&
        continuityNextSample === undefined &&
        completedTurns > continuityEditTurn
      ) {
        const output = event.detail.outputs?.find(candidate => candidate.name === "result.0");
        const values = Array.from(output?.values || []);
        if (values.length === 15 && values.every(Number.isFinite)) {
          continuityNextSample = values;
        }
      }
      if (
        expectedComputeBackend === "wgpu" && applicationQualifiedBeforeReset &&
        !incompatibleEditRequested
      ) {
        const source = acceptedSource();
        const nextSource = source.replace(
          `filter-count := ${expectedComputeInstances}.0`,
          `filter-count := ${expectedComputeInstances + 1}.0`,
        );
        if (nextSource === source) {
          throw new Error(
            "the incompatible EKF source probe could not change the compute storage shape",
          );
        }
        incompatibleBefore = computeIdentity();
        const accepted = globalThis.__MECH_REPLACE_ACCEPTED_REPL_SOURCE__?.(nextSource);
        incompatibleEditRequested = accepted === nextSource;
      }
    });

    const originalSetTimeout = window.setTimeout.bind(window);
    let harnessFrames = 0;
    let firstTruth;
    let previousTruth;
    let previousHeading;
    let lastObservedUpdate = -1;
    let lastObservedComputeDispatch = 0;
    let lastObservedCpuFilterTurn;
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
    let cameraToggleDisableRequested = false;
    let cameraToggleDisabled = false;
    let cameraToggleMeasurementBlocked = false;
    let cameraToggleEnableRequested = false;
    let cameraToggleRestored = false;
    let cameraToggleVisualStateValid = false;
    let cameraToggleDispatchBeforeDisable = -1;
    let cameraToggleDispatchBeforeEnable = -1;
    // Keep the interaction proof outside the turn-121 continuity oracle and
    // the turn-376 backend parity oracle. That makes the numerical baselines
    // independent of browser presentation cadence while still exercising a
    // real disabled measurement on the same resident application run.
    const cameraToggleDisableTurn = 380;
    const clickSceneCamera = camera => {
      const rect = camera?.getBoundingClientRect();
      if (!rect || rect.width <= 0 || rect.height <= 0) return false;
      const clientX = rect.left + rect.width / 2;
      const clientY = rect.top + rect.height / 2;
      const handled = !camera.dispatchEvent(new PointerEvent("pointerdown", {
        bubbles: true,
        cancelable: true,
        button: 0,
        buttons: 1,
        clientX,
        clientY,
      }));
      window.dispatchEvent(new PointerEvent("pointerup", {
        bubbles: true,
        cancelable: true,
        button: 0,
        buttons: 0,
        clientX,
        clientY,
      }));
      return handled;
    };
    window.requestAnimationFrame = (callback) => originalSetTimeout(() => {
      if (root.dataset.mechDone === "true" || root.dataset.mechTimedOut === "true") return;
      callback(performance.now());

      // Adapter discovery and pipeline creation are asynchronous. Chrome's
      // virtual-time budget can advance thousands of unrelated layout frames
      // while WebGPU initialization is still legitimately pending, especially
      // with a software adapter in CI. Start the simulation-frame budget only
      // after the document runtime is ready; the outer process deadline still
      // catches an initialization that actually hangs.
      if (root.dataset.mechDocumentStatus === "ready") harnessFrames += 1;

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
      const disabledCameras = [...document.querySelectorAll(
        '[data-mech-scene-id^="camera-disabled-"]',
      )];
      const cameraRanges = [...document.querySelectorAll('[data-mech-scene-id^="camera-range-"]')];
      const cameraRays = [...document.querySelectorAll('[data-mech-scene-id^="ray-"]')];
      const cameraLabels = [...document.querySelectorAll(
        '[data-mech-scene-id^="camera-label-"]',
      )];
      const host = document.querySelector('[data-mech-repl-host]');
      const outputPanel = document.querySelector('[data-mech-console-panel="output"]');
      const errorPanel = document.querySelector('[data-mech-console-panel="errors"]');
      const outputToggle = document.querySelector('button[data-mech-output-fullscreen]');
      const tabs = [...document.querySelectorAll('[data-mech-console-tab]')]
        .map(tab => tab.dataset.mechConsoleTab).join(",");
      const updates = Number(display?.dataset.mechDisplayUpdates || 0);
      const completedFilterTurn = expectedComputeBackend === "cpu-scalar"
        ? renderedFilterTurn()
        : undefined;
      if (
        expectedComputeBackend === "cpu-scalar" &&
        performContinuityEdit &&
        completedFilterTurn === continuityEditTurn &&
        !continuityEditRequested
      ) {
        continuityBefore = computeIdentity();
        continuityEditRequested = submitResidentSource("continuity-probe := 1");
      }
      if (
        continuityEditRequested && continuityBefore &&
        !continuityGenerationChanged
      ) {
        const after = computeIdentity();
        if (after.generation && after.generation !== continuityBefore.generation) {
          continuityGenerationChanged = true;
          if (expectedComputeBackend === "wgpu") {
            continuityResourcePreserved = Boolean(
              after.revision === continuityBefore.revision &&
              after.resource === continuityBefore.resource &&
              after.device === continuityBefore.device &&
              after.pipeline === continuityBefore.pipeline &&
              after.state === continuityBefore.state &&
              after.pipelineBuilds === continuityBefore.pipelineBuilds
            );
            continuityActiveBufferPreserved =
              after.activeBuffer === continuityBefore.activeBuffer;
          }
        }
      }
      if (
        expectedComputeBackend === "cpu-scalar" &&
        (continuityEditRequested || !performContinuityEdit) &&
        continuityNextSample === undefined &&
        completedFilterTurn === continuityEditTurn + 1
      ) {
        continuityNextSample = renderedFilterSample();
      }
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
        const logicalComputeTurn = expectedComputeBackend === "cpu-scalar"
          ? Number(completedFilterTurn)
          : computeDispatches;
        const isComputeCompletionFrame = expectedComputeBackend === "cpu-scalar"
          ? Number.isFinite(completedFilterTurn) &&
            completedFilterTurn !== lastObservedCpuFilterTurn
          : computeDispatches > lastObservedComputeDispatch;
        if (Number.isFinite(completedFilterTurn)) {
          lastObservedCpuFilterTurn = completedFilterTurn;
        }
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
          const rangeX = Number(range?.getAttribute("cx"));
          const rangeY = Number(range?.getAttribute("cy"));
          const rangeRadius = Number(range?.getAttribute("r"));
          const rayX = Number(ray?.getAttribute("x1"));
          const rayY = Number(ray?.getAttribute("y1"));
          return {x, y, radius, rangeX, rangeY, rangeRadius, rayX, rayY};
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
            sensor.radius === 9 &&
            sensor.x === sensor.rangeX && sensor.y === sensor.rangeY &&
            sensor.x === sensor.rayX && sensor.y === sensor.rayY
          );
        const enabledMask = renderedDocumentNumbers("camera-enabled");
        const cameraToggleStateReadable = enabledMask.length === 4 &&
          enabledMask.every(value => value === 0 || value === 1);
        const filterVisibility = renderedFilterVisibility();
        if (
          !cameraToggleDisableRequested && logicalComputeTurn === cameraToggleDisableTurn &&
          cameraToggleStateReadable && enabledMask.every(value => value === 1) &&
          Number.isFinite(filterVisibility) && filterVisibility > 0
        ) {
          cameraToggleDispatchBeforeDisable = logicalComputeTurn;
          cameraToggleDisableRequested = clickSceneCamera(orderedCameras[0]);
        }
        if (
          cameraToggleDisableRequested && !cameraToggleDisabled &&
          cameraToggleStateReadable && enabledMask[0] === 0 &&
          enabledMask.slice(1).every(value => value === 1)
        ) {
          const activeOpacity = Number(orderedCameras[0]?.getAttribute("opacity"));
          const disabledOpacity = Number(indexed(disabledCameras, "camera-disabled-")[0]
            ?.getAttribute("opacity"));
          const rangeOpacity = Number(orderedRanges[0]?.getAttribute("opacity"));
          const rayOpacity = Number(orderedRays[0]?.getAttribute("opacity"));
          const labelOpacity = Number(indexed(cameraLabels, "camera-label-")[0]
            ?.getAttribute("opacity"));
          cameraToggleVisualStateValid = activeOpacity === 0 && disabledOpacity === 1 &&
            rangeOpacity === 0 && rayOpacity === 0 && labelOpacity === 0.32;
          cameraToggleDisabled = cameraToggleVisualStateValid;
        }
        if (
          cameraToggleDisabled && !cameraToggleMeasurementBlocked &&
          logicalComputeTurn > cameraToggleDispatchBeforeDisable && filterVisibility === 0
        ) {
          cameraToggleMeasurementBlocked = true;
          cameraToggleDispatchBeforeEnable = logicalComputeTurn;
          cameraToggleEnableRequested = clickSceneCamera(orderedCameras[0]);
        }
        if (
          cameraToggleEnableRequested && cameraToggleStateReadable &&
          enabledMask.every(value => value === 1) &&
          logicalComputeTurn > cameraToggleDispatchBeforeEnable &&
          Number.isFinite(filterVisibility) && filterVisibility > 0
        ) {
          const activeOpacity = Number(orderedCameras[0]?.getAttribute("opacity"));
          const disabledOpacity = Number(indexed(disabledCameras, "camera-disabled-")[0]
            ?.getAttribute("opacity"));
          const rangeOpacity = Number(orderedRanges[0]?.getAttribute("opacity"));
          cameraToggleRestored = activeOpacity === 1 && disabledOpacity === 0 &&
            rangeOpacity === 0.16;
        }
        let visibleCameraCount = 0;
        for (let index = 0; index < currentCameraGeometry.length; index += 1) {
          const sensor = currentCameraGeometry[index];
          const rayOpacity = Number(orderedRays[index]?.getAttribute("opacity"));
          const distance = Math.hypot(truthPoint.x - sensor.x, truthPoint.y - sensor.y);
          const fadeWidth = sensor.rangeRadius * (0.18 / 3.6);
          const expectedVisibility = Math.max(0, Math.min(1,
            (sensor.rangeRadius - distance) / fadeWidth,
          ));
          const cameraEnabled = cameraToggleStateReadable ? enabledMask[index] : 1;
          const expectedRayOpacity = 0.4 * expectedVisibility * cameraEnabled;
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
        // Asynchronous WebGPU completion publishes the originating turn
        // directly, so its sampled estimate and camera visibility share one
        // resident snapshot just like the scalar backend.
        if (filterVisibility === 0 && isComputeCompletionFrame) {
          const predictionGap = finitePoint(estimate) && finitePoint(prediction)
            ? Math.hypot(
                Number(estimate.getAttribute("cx")) - Number(prediction.getAttribute("cx")),
                Number(estimate.getAttribute("cy")) - Number(prediction.getAttribute("cy")),
              )
            : Number.POSITIVE_INFINITY;
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
      if (
        updates === parityComputeTurn &&
        displayParityTrackingError === undefined &&
        Number.isFinite(trackingError)
      ) {
        displayParityTrackingError = trackingError;
      }
      if (
        completedFilterTurn === parityComputeTurn &&
        expectedComputeBackend === "cpu-scalar" &&
        computeParitySample === undefined
      ) {
        computeParitySample = renderedFilterSample();
      }
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
      root.dataset.mechObservedParityCaptured = String(computeParitySample !== undefined);
      root.dataset.mechObservedParityTrackingCaptured =
        String(displayParityTrackingError !== undefined);
      root.dataset.mechObservedSampledReadbackEfficient = String(sampledReadbackEfficient);
      root.dataset.mechObservedComputeReadbackBytes = String(computeReadbackBytes);
      root.dataset.mechObservedComputeLogicalOutputs = root.dataset.mechComputeLogicalOutputs || "missing";
      root.dataset.mechObservedComputePhysicalOutputBuffers =
        root.dataset.mechComputePhysicalOutputBuffers || "missing";
      root.dataset.mechObservedContinuityEditRequested = String(continuityEditRequested);
      root.dataset.mechObservedContinuityGenerationChanged =
        String(continuityGenerationChanged);
      root.dataset.mechObservedContinuityResourcePreserved =
        String(continuityResourcePreserved);
      root.dataset.mechObservedContinuityActiveBufferPreserved =
        String(continuityActiveBufferPreserved);
      root.dataset.mechObservedContinuityNextSample =
        JSON.stringify(continuityNextSample || null);
      root.dataset.mechObservedBusyReplacementAttempted =
        String(busyReplacementAttempted);
      root.dataset.mechObservedBusyReplacementRejected =
        String(busyReplacementRejected);
      root.dataset.mechObservedBusySourceUntouched = String(busySourceUntouched);
      root.dataset.mechObservedBusySymbolAbsent = String(busySymbolAbsent);
      root.dataset.mechObservedBusyLogicalProgressUnchanged =
        String(busyLogicalProgressUnchanged);
      root.dataset.mechObservedComputeStateResets =
        root.dataset.mechComputeStateResets || "0";
      root.dataset.mechObservedStaleCompletionRejected =
        root.dataset.mechComputeStaleCompletionRejected ||
        String(expectedComputeBackend !== "wgpu");
      root.dataset.mechObservedExpectedReadbackBytes = String(
        Number(root.dataset.mechObservedComputeCompletion || 0) *
          16 * Float32Array.BYTES_PER_ELEMENT
      );
      root.dataset.mechObservedOutputPresentation = String(outputPresentation);
      root.dataset.mechObservedErrorText = (errorPanel?.textContent || "").trim().slice(0, 1000);
      root.dataset.mechObservedDisplayIds = [...document.querySelectorAll('[data-mech-display-id]')]
        .map(element => element.dataset.mechDisplayId).join(",");
      root.dataset.mechObservedApplicationQualifiedBeforeReset =
        String(applicationQualifiedBeforeReset);
      root.dataset.mechObservedCameraToggleDisableRequested =
        String(cameraToggleDisableRequested);
      root.dataset.mechObservedCameraToggleDisabled = String(cameraToggleDisabled);
      root.dataset.mechObservedCameraToggleMeasurementBlocked =
        String(cameraToggleMeasurementBlocked);
      root.dataset.mechObservedCameraToggleEnableRequested =
        String(cameraToggleEnableRequested);
      root.dataset.mechObservedCameraToggleRestored = String(cameraToggleRestored);
      root.dataset.mechObservedCameraEnabled = JSON.stringify(
        renderedDocumentNumbers("camera-enabled"),
      );
      root.dataset.mechObservedScenePointerPulse = JSON.stringify(
        renderedDocumentNumbers("scene-pointer-pulse"),
      );
      root.dataset.mechObservedScenePointerSurface = String(Boolean(
        scene?.dataset.mechScenePointerSurface,
      ));
      root.dataset.mechObservedScenePointerSubmissions =
        document.querySelector(".mech-root")?.dataset.mechScenePointerSubmissions || "0";
      root.dataset.mechObservedScenePointerPosition =
        document.querySelector(".mech-root")?.dataset.mechScenePointerPosition || "";
      root.dataset.mechObservedCameraOneOpacity =
        document.querySelector('[data-mech-scene-id="camera-1"]')?.getAttribute("opacity") || "";
      root.dataset.mechObservedCameraDisabledOneOpacity = document.querySelector(
        '[data-mech-scene-id="camera-disabled-1"]',
      )?.getAttribute("opacity") || "";
      root.dataset.mechObservedIncompatibleEditRequested =
        String(incompatibleEditRequested);
      root.dataset.mechObservedIncompatibleGenerationChanged =
        String(incompatibleGenerationChanged);
      root.dataset.mechObservedIncompatibleResourcesReplaced =
        String(incompatibleResourcesReplaced);
      root.dataset.mechObservedIncompatibleOldResourceDisposed =
        String(incompatibleOldResourceDisposed);
      root.dataset.mechObservedIncompatibleStateReset = String(incompatibleStateReset);
      root.dataset.mechObservedIncompatibleResetDiagnostic =
        String(incompatibleResetDiagnostic);
      root.dataset.mechObservedComputeStateResetEvents = String(computeStateResetEvents);
      const continuityQualified = Boolean(
        continuityNextSample !== undefined &&
        (!performContinuityEdit || (
          continuityEditRequested && continuityGenerationChanged &&
          continuityResourcePreserved && continuityActiveBufferPreserved
        ))
      );
      const preResetQualified = Boolean(
        updates >= 376 &&
        finitePointContract &&
        cameras.length === 4 && cameraRanges.length === 4 && cameraRays.length === 4 &&
        cameraGeometryStable && cameraRangeOracleValid && predictionOnlyValid &&
        predictionOnlyComparisons > 0 &&
        cameraToggleDisableRequested && cameraToggleDisabled &&
        cameraToggleMeasurementBlocked && cameraToggleEnableRequested &&
        cameraToggleRestored && cameraToggleVisualStateValid &&
        cameraToggleDispatchBeforeDisable === cameraToggleDisableTurn &&
        Number(document.querySelector(".mech-root")?.dataset.mechScenePointerSubmissions) === 4 &&
        computeParitySample !== undefined &&
        displayParityTrackingError !== undefined &&
        sampledReadbackEfficient &&
        continuityQualified &&
        busyReplacementRejected && busySourceUntouched && busySymbolAbsent &&
        busyLogicalProgressUnchanged &&
        (expectedComputeBackend !== "wgpu" || !performContinuityEdit ||
          root.dataset.mechComputeStaleCompletionRejected === "true") &&
        Number(root.dataset.mechComputeStateResets || 0) === 0 &&
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
        computeBackend === expectedComputeBackend && computeInstances === expectedComputeInstances
      );
      if (preResetQualified) applicationQualifiedBeforeReset = true;
      const incompatibleQualified = Boolean(
        incompatibleEditRequested && incompatibleGenerationChanged &&
        incompatibleResourcesReplaced && incompatibleOldResourceDisposed &&
        incompatibleStateReset &&
        incompatibleResetDiagnostic &&
        computeStateResetEvents === (expectedComputeBackend === "wgpu" ? 1 : 0) &&
        Number(root.dataset.mechComputeStateResets || 0) ===
          (expectedComputeBackend === "wgpu" ? 1 : 0) &&
        (expectedComputeBackend !== "wgpu" ||
          document.querySelectorAll(
            '[data-mech-diagnostic-code="ComputeStateReset"]',
          ).length === 1)
      );
      if (applicationQualifiedBeforeReset && incompatibleQualified) {
        root.dataset.mechUpdates = String(updates);
        root.dataset.mechCameras = String(cameras.length);
        root.dataset.mechCameraRanges = String(cameraRanges.length);
        root.dataset.mechSawNoCamera = String(sawNoCamera);
        root.dataset.mechSawCamera = String(sawCamera);
        root.dataset.mechMaxVisibleCameras = String(maxVisibleCameras);
        root.dataset.mechCameraGeometryStable = String(cameraGeometryStable);
        root.dataset.mechCameraToggleDisabled = String(cameraToggleDisabled);
        root.dataset.mechCameraToggleMeasurementBlocked =
          String(cameraToggleMeasurementBlocked);
        root.dataset.mechCameraToggleRestored = String(cameraToggleRestored);
        root.dataset.mechCameraTogglePointerSubmissions =
          document.querySelector(".mech-root")?.dataset.mechScenePointerSubmissions || "0";
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
        root.dataset.mechTruthX = truth.getAttribute("cx");
        root.dataset.mechTruthY = truth.getAttribute("cy");
        root.dataset.mechEstimateX = estimate.getAttribute("cx");
        root.dataset.mechEstimateY = estimate.getAttribute("cy");
        root.dataset.mechParityUpdates = String(parityComputeTurn);
        root.dataset.mechParityOutput = computeParitySample.join(",");
        root.dataset.mechParityTrackingError = displayParityTrackingError.toFixed(4);
        root.dataset.mechSceneVisible = String(sceneVisible);
        root.dataset.mechOutputPresentation = String(outputPresentation);
        root.dataset.mechVerifiedComputeBackend = computeBackend;
        root.dataset.mechVerifiedComputeInstances = String(computeInstances);
        root.dataset.mechSampledReadbackEfficient = String(sampledReadbackEfficient);
        root.dataset.mechGpuToCpuReadbackBytes = String(computeReadbackBytes);
        root.dataset.mechContinuityGenerationChanged =
          String(continuityGenerationChanged);
        root.dataset.mechContinuityResourcePreserved =
          String(continuityResourcePreserved);
        root.dataset.mechContinuityActiveBufferPreserved =
          String(continuityActiveBufferPreserved);
        root.dataset.mechContinuityNextSample = continuityNextSample.join(",");
        root.dataset.mechBusyReplacementRejected = String(busyReplacementRejected);
        root.dataset.mechBusySourceUntouched = String(busySourceUntouched);
        root.dataset.mechBusySymbolAbsent = String(busySymbolAbsent);
        root.dataset.mechBusyLogicalProgressUnchanged =
          String(busyLogicalProgressUnchanged);
        root.dataset.mechStaleCompletionRejected =
          root.dataset.mechComputeStaleCompletionRejected || "true";
        root.dataset.mechIncompatibleGenerationChanged =
          String(incompatibleGenerationChanged);
        root.dataset.mechIncompatibleResourcesReplaced =
          String(incompatibleResourcesReplaced);
        root.dataset.mechIncompatibleOldResourceDisposed =
          String(incompatibleOldResourceDisposed);
        root.dataset.mechIncompatibleStateReset = String(incompatibleStateReset);
        root.dataset.mechIncompatibleResetDiagnostic =
          String(incompatibleResetDiagnostic);
        root.dataset.mechComputeStateResetsVerified =
          root.dataset.mechComputeStateResets || "0";
        root.dataset.mechDone = "true";
        globalThis.__MECH_STOP__?.();
        return;
      }
      if (updates >= 450 && !cameraToggleDisabled) {
        root.dataset.mechTimedOut = "true";
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
harness = harness.replace("__MECH_EXPECTED_COMPUTE_INSTANCES__", filter_count)
harness = harness.replace("__MECH_PERFORM_CONTINUITY_EDIT__", continuity_edit)
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
import base64
import json
import os
from pathlib import Path
import signal
import socket
import struct
import subprocess
import sys
import time
import urllib.request
from urllib.parse import urlparse

from scripts.browser_webgpu_flags import chrome_webgpu_test_flags

chrome, page_url, profile, dom_file, chrome_log, compute_backend = sys.argv[1:]
args = [
    chrome,
    "--headless=new",
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--remote-debugging-port=0",
    "--remote-allow-origins=*",
    f"--user-data-dir={profile}",
    page_url,
]
if compute_backend != "wgpu":
    args.insert(-1, "--disable-gpu")
else:
    # The canary proves the browser WebGPU transport, not host-driver setup.
    # Force Chromium's test adapter so headless macOS and GPU-less Linux CI do
    # not hang while probing an unavailable presentation-capable device.
    for flag in chrome_webgpu_test_flags(
        software_adapter=True,
        linux=sys.platform.startswith("linux"),
    ):
        args.insert(-1, flag)

def read_exact(connection, length):
    chunks = []
    remaining = length
    while remaining:
        chunk = connection.recv(remaining)
        if not chunk:
            raise RuntimeError("Chrome closed the debugging socket")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)

def send_frame(connection, payload, opcode=1):
    payload = payload if isinstance(payload, bytes) else payload.encode()
    mask = os.urandom(4)
    length = len(payload)
    header = bytearray([0x80 | opcode])
    if length < 126:
        header.append(0x80 | length)
    elif length <= 0xffff:
        header.append(0x80 | 126)
        header.extend(struct.pack("!H", length))
    else:
        header.append(0x80 | 127)
        header.extend(struct.pack("!Q", length))
    header.extend(mask)
    header.extend(byte ^ mask[index % 4] for index, byte in enumerate(payload))
    connection.sendall(header)

def receive_message(connection):
    fragments = []
    message_opcode = None
    while True:
        first, second = read_exact(connection, 2)
        final = bool(first & 0x80)
        opcode = first & 0x0f
        length = second & 0x7f
        if length == 126:
            length = struct.unpack("!H", read_exact(connection, 2))[0]
        elif length == 127:
            length = struct.unpack("!Q", read_exact(connection, 8))[0]
        mask = read_exact(connection, 4) if second & 0x80 else None
        payload = read_exact(connection, length)
        if mask:
            payload = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
        if opcode == 8:
            raise RuntimeError("Chrome closed the debugging websocket")
        if opcode == 9:
            send_frame(connection, payload, opcode=10)
            continue
        if opcode in (1, 2):
            message_opcode = opcode
            fragments = [payload]
        elif opcode == 0:
            fragments.append(payload)
        else:
            continue
        if final:
            joined = b"".join(fragments)
            return joined.decode() if message_opcode == 1 else joined

def connect_debugger(process, deadline):
    active_port = Path(profile) / "DevToolsActivePort"
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"Chrome exited during debugger startup ({process.returncode})")
        try:
            port = int(active_port.read_text().splitlines()[0])
            targets = json.load(urllib.request.urlopen(
                f"http://127.0.0.1:{port}/json/list", timeout=1,
            ))
            target = next(
                (candidate for candidate in targets if candidate.get("type") == "page"),
                None,
            )
            if target:
                endpoint = urlparse(target["webSocketDebuggerUrl"])
                connection = socket.create_connection((endpoint.hostname, endpoint.port), timeout=2)
                key = base64.b64encode(os.urandom(16)).decode()
                request = (
                    f"GET {endpoint.path} HTTP/1.1\r\n"
                    f"Host: {endpoint.hostname}:{endpoint.port}\r\n"
                    "Upgrade: websocket\r\n"
                    "Connection: Upgrade\r\n"
                    f"Sec-WebSocket-Key: {key}\r\n"
                    "Sec-WebSocket-Version: 13\r\n\r\n"
                )
                connection.sendall(request.encode())
                response = b""
                while b"\r\n\r\n" not in response:
                    response += connection.recv(4096)
                if not response.startswith(b"HTTP/1.1 101"):
                    raise RuntimeError(f"debugging websocket rejected the upgrade: {response[:200]!r}")
                connection.settimeout(5)
                return connection
        except (OSError, ValueError, StopIteration, urllib.error.URLError):
            pass
        time.sleep(0.1)
    raise RuntimeError("Chrome did not expose its debugging target")

next_command_id = 0
def command(connection, method, params=None):
    global next_command_id
    next_command_id += 1
    command_id = next_command_id
    send_frame(connection, json.dumps({
        "id": command_id,
        "method": method,
        "params": params or {},
    }))
    while True:
        message = json.loads(receive_message(connection))
        if message.get("id") == command_id:
            if "error" in message:
                raise RuntimeError(f"Chrome debugging command failed: {message['error']}")
            return message.get("result", {})

def evaluate(connection, expression):
    result = command(connection, "Runtime.evaluate", {
        "expression": expression,
        "returnByValue": True,
        "awaitPromise": True,
    })
    if "exceptionDetails" in result:
        raise RuntimeError(f"browser evaluation failed: {result['exceptionDetails']}")
    return result.get("result", {}).get("value")

def stop(process):
    if process.poll() is None:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait()

deadline = time.monotonic() + 90
milestones = {}
connection = None
with Path(chrome_log).open("wb") as stderr:
    process = subprocess.Popen(
        args,
        stdout=subprocess.DEVNULL,
        stderr=stderr,
        start_new_session=True,
    )
try:
    connection = connect_debugger(process, deadline)
    command(connection, "Runtime.enable")
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"Chrome exited while running the canary ({process.returncode})")
        snapshot = evaluate(connection, r'''(() => {
          const root = document.documentElement;
          const data = root?.dataset || {};
          return {
            readyState: document.readyState,
            documentStatus: data.mechDocumentStatus || "",
            adapterStatus: data.mechComputeAdapterStatus || "",
            deviceStatus: data.mechComputeDeviceStatus || "",
            computeLifecycle: data.mechComputeLifecycle || "",
            computeBackend: data.mechComputeBackend || "",
            computeDispatches: data.mechComputeDispatches || "0",
            done: data.mechDone === "true",
            timedOut: data.mechTimedOut === "true",
            consoleError: data.mechConsoleError || "",
            pageError: data.mechPageError || "",
          };
        })()''') or {}
        milestones = snapshot
        if any((
            snapshot.get("done"),
            snapshot.get("timedOut"),
            snapshot.get("consoleError"),
            snapshot.get("pageError"),
        )):
            break
        time.sleep(0.1)
    html = evaluate(connection, "document.documentElement.outerHTML") or ""
    Path(dom_file).write_text(html)
finally:
    if connection is not None:
        try:
            connection.close()
        except OSError:
            pass
    stop(process)
    with Path(chrome_log).open("ab") as log:
        log.write(("\nMech browser milestones: " + json.dumps(milestones, sort_keys=True) + "\n").encode())
raise SystemExit(124)
PY
chrome_status="$?"
set -e

updates="$(sed -n 's/.*data-mech-updates="\([0-9][0-9]*\)".*/\1/p' "$dom_file" | head -1)"
expected_continuity_generation_changed="$continuity_edit"
expected_compute_state_resets=0
verified_compute_instances="$filter_count"
if [[ "$expected_compute_backend" == "wgpu" ]]; then
  expected_compute_state_resets=1
  verified_compute_instances="$((filter_count + 1))"
fi
if [[ "$chrome_status" -ne 0 && "$chrome_status" -ne 124 ]] \
  || ! grep -q 'data-mech-done="true"' "$dom_file" \
  || ! grep -q 'data-mech-cameras="4"' "$dom_file" \
  || ! grep -q 'data-mech-camera-ranges="4"' "$dom_file" \
  || ! grep -q 'data-mech-saw-no-camera="true"' "$dom_file" \
  || ! grep -q 'data-mech-saw-camera="true"' "$dom_file" \
  || ! grep -q 'data-mech-max-visible-cameras="1"' "$dom_file" \
  || ! grep -q 'data-mech-camera-geometry-stable="true"' "$dom_file" \
  || ! grep -q 'data-mech-camera-toggle-disabled="true"' "$dom_file" \
  || ! grep -q 'data-mech-camera-toggle-measurement-blocked="true"' "$dom_file" \
  || ! grep -q 'data-mech-camera-toggle-restored="true"' "$dom_file" \
  || ! grep -q 'data-mech-camera-toggle-pointer-submissions="4"' "$dom_file" \
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
  || ! grep -q "data-mech-verified-compute-instances=\"$verified_compute_instances\"" "$dom_file" \
  || ! grep -q 'data-mech-sampled-readback-efficient="true"' "$dom_file" \
  || ! grep -q "data-mech-continuity-generation-changed=\"$expected_continuity_generation_changed\"" "$dom_file" \
  || ! grep -q 'data-mech-continuity-resource-preserved="true"' "$dom_file" \
  || ! grep -q 'data-mech-continuity-active-buffer-preserved="true"' "$dom_file" \
  || ! grep -q 'data-mech-continuity-next-sample="[-0-9.,eE+]*"' "$dom_file" \
  || ! grep -q 'data-mech-busy-replacement-rejected="true"' "$dom_file" \
  || ! grep -q 'data-mech-busy-source-untouched="true"' "$dom_file" \
  || ! grep -q 'data-mech-busy-symbol-absent="true"' "$dom_file" \
  || ! grep -q 'data-mech-busy-logical-progress-unchanged="true"' "$dom_file" \
  || ! grep -q 'data-mech-stale-completion-rejected="true"' "$dom_file" \
  || ! grep -q 'data-mech-incompatible-generation-changed="true"' "$dom_file" \
  || ! grep -q 'data-mech-incompatible-resources-replaced="true"' "$dom_file" \
  || ! grep -q 'data-mech-incompatible-old-resource-disposed="true"' "$dom_file" \
  || ! grep -q 'data-mech-incompatible-state-reset="true"' "$dom_file" \
  || ! grep -q 'data-mech-incompatible-reset-diagnostic="true"' "$dom_file" \
  || ! grep -q "data-mech-compute-state-resets-verified=\"$expected_compute_state_resets\"" "$dom_file" \
  || ! grep -q 'data-mech-tracking-error-pixels="[0-9]' "$dom_file" \
  || ! grep -q 'data-mech-truth-x="[0-9]' "$dom_file" \
  || ! grep -q 'data-mech-truth-y="[0-9]' "$dom_file" \
  || ! grep -q 'data-mech-estimate-x="[0-9]' "$dom_file" \
  || ! grep -q 'data-mech-estimate-y="[0-9]' "$dom_file" \
  || ! grep -qE 'data-mech-parity-updates="[3-9][0-9][0-9]"' "$dom_file" \
  || ! grep -q 'data-mech-parity-output="[-0-9.,eE+]*"' "$dom_file" \
  || ! grep -q 'data-mech-parity-tracking-error="[0-9]' "$dom_file" \
  || [[ -z "$updates" || "$updates" -lt 376 ]] \
  || grep -qE 'data-mech-(console-error|page-error|timed-out)=' "$dom_file"; then
  echo "Served resident EKF browser smoke test failed" >&2
  python3 - "$dom_file" <<'PY' >&2 || true
from html import unescape
from pathlib import Path
import re
import sys

line = Path(sys.argv[1]).read_text(errors="replace").split("<head>", 1)[0]
for match in re.finditer(r'(data-mech-observed-[a-z0-9-]+)="([^"]*)"', line):
    print(f"{match.group(1)}: {unescape(match.group(2))}")
for name in (
    "data-mech-document-error",
    "data-mech-console-error",
    "data-mech-page-error",
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
parity_tracking_error="$(sed -n 's/.*data-mech-parity-tracking-error="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
turning_samples="$(sed -n 's/.*data-mech-turning-samples="\([0-9][0-9]*\)".*/\1/p' "$dom_file" | head -1)"
max_heading_step="$(sed -n 's/.*data-mech-max-heading-step="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
max_guide_deviation="$(sed -n 's/.*data-mech-max-guide-deviation="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
covariance_extent="$(sed -n 's/.*data-mech-covariance-extent="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
truth_x="$(sed -n 's/.*data-mech-truth-x="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
truth_y="$(sed -n 's/.*data-mech-truth-y="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
estimate_x="$(sed -n 's/.*data-mech-estimate-x="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
estimate_y="$(sed -n 's/.*data-mech-estimate-y="\([0-9.][0-9.]*\)".*/\1/p' "$dom_file" | head -1)"
parity_updates="$(sed -n 's/.*data-mech-parity-updates="\([0-9][0-9]*\)".*/\1/p' "$dom_file" | head -1)"
parity_output="$(sed -n 's/.*data-mech-parity-output="\([^"]*\)".*/\1/p' "$dom_file" | head -1)"
continuity_output="$(sed -n 's/.*data-mech-continuity-next-sample="\([^"]*\)".*/\1/p' "$dom_file" | head -1)"
if [[ -n "${MECH_EKF_RESULT_FILE:-}" ]]; then
  python3 - "$MECH_EKF_RESULT_FILE" "$compute_backend" "$parity_updates" \
    "$parity_output" "$parity_tracking_error" "$continuity_output" <<'PY'
import json
from pathlib import Path
import sys

path, backend, updates, output, tracking, continuity_output = sys.argv[1:]
values = [float(value) for value in output.split(",") if value]
continuity_values = [float(value) for value in continuity_output.split(",") if value]
if len(values) != 15:
    raise SystemExit(f"expected 15 EKF checkpoint values, got {len(values)}")
if len(continuity_values) != 15:
    raise SystemExit(
        f"expected 15 EKF continuity checkpoint values, got {len(continuity_values)}"
    )
Path(path).write_text(json.dumps({
    "backend": backend,
    "updates": int(updates),
    "output": values,
    "continuity_output": continuity_values,
    "tracking_error": float(tracking),
}, sort_keys=True))
PY
fi
printf 'EKF_E2E display_updates=%s requested_compute_backend=%s compute_backend=%s initial_compute_instances=%s verified_compute_instances=%s cameras=4 camera_toggle_disabled=true camera_measurement_blocked=true camera_toggle_restored=true max_visible_cameras=1 saw_no_camera=true square_sides=4 lap_complete=true smooth_turning=true turning_samples=%s max_heading_step=%s max_guide_deviation_pixels=%s truth_moved=true covariance_finite=true covariance_extent_pixels=%s paths_finite=true tracking_error_pixels=%s output_presentation=true console_errors=0 page_errors=0\n' "$updates" "$compute_backend" "$expected_compute_backend" "$filter_count" "$verified_compute_instances" "$turning_samples" "$max_heading_step" "$max_guide_deviation" "$covariance_extent" "$tracking_error_pixels"
