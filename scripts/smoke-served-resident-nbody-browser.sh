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
if ! grep -qx 'matrix' "$native_log" || ! grep -Eq '^\[-0\.[0-9]+\]$' "$native_log"; then
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
    let firstMercuryX;
    let firstMercuryY;
    let lastSunDisplayUpdate = -1;
    let sunFrameCount = 0;
    let maximumSunOffset = 0;
    let sunEverOffCenter = false;
    let minimumMercuryOffset = Number.POSITIVE_INFINITY;
    // Table-backed scene collections rebuild more structured data per frame.
    // Preserve the 600-turn physics proof while allowing slower CI runners to
    // complete it instead of weakening the number of observed simulation steps.
    const deadline = Date.now() + 30000;
    window.requestAnimationFrame = (callback) => originalSetTimeout(() => {
      if (root.dataset.mechDone === "true" || root.dataset.mechTimedOut === "true") return;
      callback(performance.now());
      const mercury = document.querySelector('[data-mech-scene-id="body-1"]');
      if (mercury && firstMercuryX === undefined) {
        firstMercuryX = mercury.getAttribute("cx");
        firstMercuryY = mercury.getAttribute("cy");
      }
      const display = document.querySelector('[data-mech-display-id="scene-orbit"]');
      const displayUpdates = Number(display?.dataset.mechDisplayUpdates || 0);
      const circles = document.querySelectorAll('[data-mech-scene-id^="body-"]');
      const orbitGuides = document.querySelectorAll('[data-mech-scene-id^="orbit-"]');
      const title = document.querySelector('[data-mech-scene-id="title"]');
      const scene = document.querySelector('[data-mech-rich-scene="true"]');
      const sceneRect = scene?.getBoundingClientRect();
      const host = document.querySelector('[data-mech-repl-host]');
      const expectedRadiusBands = [
        [0, 1],
        [20, 35],
        [30, 45],
        [35, 52],
        [45, 65],
        [85, 115],
        [120, 155],
        [175, 215],
        [220, 260],
        [215, 265],
      ];
      const bodyRadii = expectedRadiusBands.map((_, index) => {
        const circle = document.querySelector(`[data-mech-scene-id="body-${index}"]`);
        return circle
          ? Math.hypot(Number(circle.getAttribute("cx")) - 430, Number(circle.getAttribute("cy")) - 380)
          : Number.NaN;
      });
      const sceneGeometryCorrect = bodyRadii.every(
        (radius, index) => radius >= expectedRadiusBands[index][0] && radius < expectedRadiusBands[index][1]
      );
      const guidePoints = (guide) => (guide?.getAttribute("points") || "")
        .trim()
        .split(/\s+/)
        .filter(Boolean)
        .map((point) => point.split(",").map(Number));
      const bodyNames = ["mercury", "venus", "earth", "mars", "jupiter", "saturn", "uranus", "neptune", "pluto"];
      const guidesMatchBodies = bodyNames.every((name, index) => {
        const guide = document.querySelector(`[data-mech-scene-id="orbit-${name}"]`);
        const body = document.querySelector(`[data-mech-scene-id="body-${index + 1}"]`);
        const points = guidePoints(guide);
        if (guide?.tagName.toLowerCase() !== "polyline" || points.length < 97 || !body) return false;
        const bodyX = Number(body.getAttribute("cx"));
        const bodyY = Number(body.getAttribute("cy"));
        return Math.min(...points.map(([x, y]) => Math.hypot(x - bodyX, y - bodyY))) < 8;
      });
      const guideRadialRange = (name) => {
        const radii = guidePoints(document.querySelector(`[data-mech-scene-id="orbit-${name}"]`))
          .map(([x, y]) => Math.hypot(x - 430, y - 380));
        return radii.length > 0 ? [Math.min(...radii), Math.max(...radii)] : [Number.NaN, Number.NaN];
      };
      const neptuneGuideRange = guideRadialRange("neptune");
      const plutoGuideRange = guideRadialRange("pluto");
      const orbitGuidesPhysicallyDistinct =
        neptuneGuideRange[1] - neptuneGuideRange[0] < 5 &&
        plutoGuideRange[1] - plutoGuideRange[0] > 50 &&
        plutoGuideRange[1] > neptuneGuideRange[1] + 50;
      const sunOffset = bodyRadii[0];
      const mercuryOffset = bodyRadii[1];
      if (
        Number.isFinite(sunOffset) &&
        Number.isFinite(mercuryOffset) &&
        displayUpdates > 0 &&
        displayUpdates !== lastSunDisplayUpdate
      ) {
        lastSunDisplayUpdate = displayUpdates;
        sunFrameCount += 1;
        maximumSunOffset = Math.max(maximumSunOffset, sunOffset);
        sunEverOffCenter ||= sunOffset >= 0.001;
        minimumMercuryOffset = Math.min(minimumMercuryOffset, mercuryOffset);
      }
      const sunCentered = sunFrameCount > 0 && !sunEverOffCenter;
      // The independently bounded 0.295 AU perihelion maps above 23 px.
      const mercuryClearsSun = minimumMercuryOffset > 23;
      root.dataset.mechBodyRadii = bodyRadii.map((radius) => radius.toFixed(3)).join(",");
      root.dataset.mechSunCentered = String(sunCentered);
      root.dataset.mechSunFrameCount = String(sunFrameCount);
      root.dataset.mechMaximumSunOffset = maximumSunOffset.toExponential(3);
      root.dataset.mechMercuryClearsSun = String(mercuryClearsSun);
      root.dataset.mechMinimumMercuryOffset = minimumMercuryOffset.toFixed(3);
      const sceneVisible = Boolean(
        sceneRect &&
        sceneRect.width > 0 &&
        sceneRect.height > 0 &&
        sceneRect.top < window.innerHeight &&
        sceneRect.bottom > 0 &&
        sceneRect.left < window.innerWidth &&
        sceneRect.right > 0
      );
      root.dataset.mechObservedRendered = String(displayUpdates);
      if (root.dataset.mechPresentationRevealStarted === "true") {
        const content = document.querySelector('.content-shell, .content, #left-pane');
        const outputTab = document.querySelector('[data-mech-console-tab="output"]');
        const presentationRevealed =
          host?.dataset.mechPresentationView === "workspace" &&
          host?.dataset.mechOutputFullscreenActive === "false" &&
          host?.dataset.mechConsoleOpen === "true" &&
          outputTab?.getAttribute("aria-selected") === "true" &&
          content && getComputedStyle(content).display !== "none";
        root.dataset.mechPresentationRevealed = String(presentationRevealed);
        if (presentationRevealed) {
          root.dataset.mechDone = "true";
          globalThis.MechDocumentController?.dispose();
          return;
        }
      }
      if (
        mercury &&
        circles.length === 10 &&
        orbitGuides.length === 9 &&
        guidesMatchBodies &&
        orbitGuidesPhysicallyDistinct &&
        title?.textContent === "Solar-System Orbit Viewer" &&
        sceneGeometryCorrect &&
        sunCentered &&
        mercuryClearsSun &&
        sceneVisible &&
        sunFrameCount >= 600 &&
        displayUpdates >= 600
      ) {
          const outputPanel = document.querySelector('[data-mech-console-panel="output"]');
          if (root.dataset.mechPresentationRevealStarted !== "true") {
            const outputToggle = document.querySelector('button[data-mech-output-fullscreen]');
            const tabs = [...document.querySelectorAll('[data-mech-console-tab]')]
              .map(tab => tab.dataset.mechConsoleTab).join(",");
            root.dataset.mechRendered = String(displayUpdates);
            root.dataset.mechCircles = String(circles.length);
            root.dataset.mechOrbitGuides = String(orbitGuides.length);
            root.dataset.mechGuidesMatchBodies = String(guidesMatchBodies);
            root.dataset.mechOrbitGuidesPhysicallyDistinct = String(orbitGuidesPhysicallyDistinct);
            root.dataset.mechNeptuneGuideRange = neptuneGuideRange.map(value => value.toFixed(3)).join(",");
            root.dataset.mechPlutoGuideRange = plutoGuideRange.map(value => value.toFixed(3)).join(",");
            root.dataset.mechSceneTitle = title.textContent;
            root.dataset.mechSceneGeometryCorrect = String(sceneGeometryCorrect);
            root.dataset.mechSceneVisible = String(sceneVisible);
            root.dataset.mechSceneWidth = String(Math.round(sceneRect.width));
            root.dataset.mechSceneHeight = String(Math.round(sceneRect.height));
            root.dataset.mechMercuryMoved = String(
              mercury.getAttribute("cx") !== firstMercuryX ||
              mercury.getAttribute("cy") !== firstMercuryY
            );
            root.dataset.mechOutputPresentation = String(
              host?.dataset.mechPresentation === "output" &&
              host?.dataset.mechPresentationView === "output" &&
              host?.dataset.mechOutputFullscreenActive === "true" &&
              outputToggle?.getAttribute("aria-pressed") === "true" &&
              outputPanel?.hidden === false &&
              tabs === "output,console,errors"
            );
            root.dataset.mechRichScene = String(
              display?.querySelector('[data-mech-rich-scene="true"]') !== null
            );
            root.dataset.mechRichDisplayOperation = display?.dataset.mechDisplayOperation || "";
            root.dataset.mechRichDisplayUpdates = String(displayUpdates);
            root.dataset.mechRichCircles = String(circles.length);
            const input = document.querySelector('.repl-input');
            if (input) {
              input.value = "n-body-draft";
              const event = new KeyboardEvent("keydown", {
                key: "`", bubbles: true, cancelable: true,
              });
              input.dispatchEvent(event);
              root.dataset.mechBacktickCaptured = String(
                event.defaultPrevented && input.value === "n-body-draft"
              );
            }
            root.dataset.mechPresentationRevealStarted = "true";
            return;
          }
      }
      if (Date.now() >= deadline && root.dataset.mechDone !== "true") {
        root.dataset.mechTimedOut = "true";
        root.dataset.mechSceneGeometryCorrect = String(sceneGeometryCorrect);
        root.dataset.mechSunCentered = String(sunCentered);
        globalThis.MechDocumentController?.dispose();
      }
    }, 16);
  </script>'''
if html.count(marker) != 1:
    raise SystemExit("could not find the head boundary in generated n-body HTML")
path.write_text(html.replace(marker, harness + "\n  " + marker, 1))
PY
server_ready_deadline=$((SECONDS + server_ready_timeout_seconds))
while ((SECONDS < server_ready_deadline)); do
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
  python3 - "$page_url" "$chrome_profile" "$dom_file" "$chrome_log" <<'PY'
import sys

from tests.browser.harness import ChromeSession


page_url, profile, dom_file, chrome_log = sys.argv[1:]
browser = ChromeSession(
    None,
    profile,
    chrome_log,
    flags=[
        "--disable-gpu",
        "--run-all-compositor-stages-before-draw",
    ],
).start()
try:
    browser.navigate(page_url)
    browser.wait_for(
        "document.documentElement?.dataset.mechDone === 'true' || "
        "document.documentElement?.dataset.mechTimedOut === 'true'",
        "the n-body browser proof",
        timeout=90,
        interval=0.25,
    )
    browser.write_dom(dom_file)
finally:
    browser.close()
raise SystemExit(124)
PY
}

set +e
run_chrome
chrome_status="$?"
set -e

rendered="$(sed -n 's/.*data-mech-rendered="\([0-9][0-9]*\)".*/\1/p' "$dom_file" | head -1)"
sun_frames="$(sed -n 's/.*data-mech-sun-frame-count="\([0-9][0-9]*\)".*/\1/p' "$dom_file" | head -1)"
neptune_guide_range="$(sed -n 's/.*data-mech-neptune-guide-range="\([^"]*\)".*/\1/p' "$dom_file" | head -1)"
pluto_guide_range="$(sed -n 's/.*data-mech-pluto-guide-range="\([^"]*\)".*/\1/p' "$dom_file" | head -1)"
if [[ "$chrome_status" -ne 0 && "$chrome_status" -ne 124 ]] \
  || ! grep -q 'data-mech-done="true"' "$dom_file" \
  || ! grep -q 'data-mech-circles="10"' "$dom_file" \
  || ! grep -q 'data-mech-orbit-guides="9"' "$dom_file" \
  || ! grep -q 'data-mech-guides-match-bodies="true"' "$dom_file" \
  || ! grep -q 'data-mech-orbit-guides-physically-distinct="true"' "$dom_file" \
  || ! grep -q 'data-mech-scene-title="Solar-System Orbit Viewer"' "$dom_file" \
  || ! grep -q 'data-mech-scene-geometry-correct="true"' "$dom_file" \
  || ! grep -q 'data-mech-sun-centered="true"' "$dom_file" \
  || ! grep -q 'data-mech-mercury-clears-sun="true"' "$dom_file" \
  || ! grep -q 'data-mech-scene-visible="true"' "$dom_file" \
  || ! grep -q 'data-mech-scene-width="[1-9][0-9]*"' "$dom_file" \
  || ! grep -q 'data-mech-scene-height="[1-9][0-9]*"' "$dom_file" \
  || ! grep -q 'data-mech-mercury-moved="true"' "$dom_file" \
  || ! grep -q 'data-mech-output-presentation="true"' "$dom_file" \
  || ! grep -q 'data-mech-backtick-captured="true"' "$dom_file" \
  || ! grep -q 'data-mech-presentation-revealed="true"' "$dom_file" \
  || ! grep -q 'data-mech-rich-scene="true"' "$dom_file" \
  || ! grep -q 'data-mech-rich-display-operation="update"' "$dom_file" \
  || ! grep -q 'data-mech-rich-display-updates="[1-9][0-9]*"' "$dom_file" \
  || ! grep -q 'data-mech-rich-circles="10"' "$dom_file" \
  || [[ -z "$rendered" || "$rendered" -lt 600 ]] \
  || [[ -z "$sun_frames" || "$sun_frames" -lt 600 ]] \
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

printf 'NBODY_E2E native_energy=true display_updates=%s sun_frames=%s bodies=10 orbit_guides=9 guides_match_bodies=true neptune_guide_range=%s pluto_guide_range=%s title=true scene_geometry=true sun_centered_continuously=true mercury_clears_sun=true scene_visible=true rich_scene=true rich_operation=update mercury_moved=true output_presentation=true backtick_reveals_workspace=true console_errors=0 page_errors=0\n' "$rendered" "$sun_frames" "$neptune_guide_range" "$pluto_guide_range"
