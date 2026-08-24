#!/usr/bin/env bash
set -euo pipefail

# Exercise the shipped standalone document shells with the browser package that
# was embedded into the server binary.  Keep this separate from the FizzBuzz
# smoke: this script deliberately checks the richer presentation and console
# contracts rather than a single rendered program.

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

fixture="$repo_root/tests/fixtures/shims/all-slots.mec"
if [[ ! -f "$fixture" ]]; then
  echo "Rich document fixture is missing: $fixture" >&2
  exit 1
fi

# The binary must be self-contained.  Deleting the generated directory before
# serving catches an accidental runtime dependency on source-tree WASM assets.
rm -rf "$repo_root/src/wasm/pkg"

mkdir -p "$target_dir"
work_dir="$(mktemp -d "$target_dir/served-rich-document.XXXXXX")"
server_pid=""

stop_server() {
  if [[ -z "$server_pid" ]]; then
    return
  fi
  kill -INT "$server_pid" 2>/dev/null || true
  for _ in $(seq 1 50); do
    if ! kill -0 "$server_pid" 2>/dev/null; then
      break
    fi
    sleep 0.1
  done
  kill "$server_pid" 2>/dev/null || true
  wait "$server_pid" 2>/dev/null || true
  server_pid=""
}

cleanup() {
  local status="$?"
  stop_server
  if [[ "$status" -ne 0 ]]; then
    echo "Rich document browser artifacts retained at: $work_dir" >&2
  else
    rm -rf "$work_dir"
  fi
}
trap cleanup EXIT

port_for_test() {
  python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

wait_for_server() {
  local page_url="$1"
  local server_log="$2"
  for _ in $(seq 1 150); do
    if curl --fail --silent --output /dev/null "$page_url" 2>/dev/null; then
      return
    fi
    sleep 0.1
  done
  echo "Server did not respond at $page_url" >&2
  sed -n '1,300p' "$server_log" >&2 || true
  return 1
}

prepare_formatted_case() {
  local label="$1"
  local shim="$2"
  local stylesheet="$3"
  local case_dir="$work_dir/$label"
  mkdir -p "$case_dir"
  cp "$fixture" "$case_dir/all-slots.mec"
  cp "$repo_root/tests/fixtures/shims/hero.svg" "$case_dir/hero.svg"

  if ! "$MECH_BIN" --no-config format \
    "$fixture" \
    --html \
    --shim "$shim" \
    --stylesheet "$stylesheet" \
    --out "$case_dir/index.html" >"$case_dir/format.log" 2>&1; then
    echo "Could not format rich document case: $label" >&2
    sed -n '1,300p' "$case_dir/format.log" >&2 || true
    return 1
  fi
}

# Exercise the same formatted-document controller through a configured project
# route. The document imports sibling sources and declares an injected browser
# host, so this catches a regression where source pages bypass the projected
# resolver or host/grant authority used by `mech serve`.
prepare_configured_case() {
  local case_dir="$work_dir/configured"
  mkdir -p "$case_dir/package"
  cp "$fixture" "$case_dir/main.mec"
  cp "$repo_root/tests/fixtures/shims/hero.svg" "$case_dir/hero.svg"
  cat > "$case_dir/support.mec" <<'EOF'
value := 11
<+ value
EOF
  cat > "$case_dir/package/index.mec" <<'EOF'
value := 13
<+ value
EOF
  cat > "$case_dir/included.mec" <<'EOF'
{nested-included.mec}
included-value := 7
EOF
  cat > "$case_dir/nested-included.mec" <<'EOF'
nested-included-value := 10
EOF
  cat > "$case_dir/mech.mcfg" <<'EOF'
config := {
  hosts: [
    {
      name: "clock"
      provider: "time"
      settings: {}
    }
    {
      name: "repl"
      provider: "time"
      settings: {}
    }
  ]

  serve: {
    paths: ["."]
  }

  run: {
    paths: ["main.mec"]
    grants: [
      {
        target: "clock/clock"
        operations: ["read"]
        paths: ["second"]
      }
    ]
  }
}
EOF
  python3 - "$case_dir/main.mec" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
source = path.read_text()
original = "~answer := 41"
# Keep the live clock read independent from the mutable console fixture below.
# A clock tick legitimately recomputes its dependents between browser frames;
# making `answer` depend on it would race the later `answer = 7` console
# assertion and test the scheduler rather than document-console mutation.
replacement = """+> ./support
+> ./package
{included.mec}
@clock := time://clock/clock{:read(second)}
configured-answer := support/value + package/value + included-value + nested-included-value

~~~mech
configured-answer
~~~

~answer := 41"""
if source.count(original) != 1:
    raise SystemExit("configured rich fixture did not contain exactly one answer declaration")
path.write_text(source.replace(original, replacement, 1))
PY
}

run_browser_case() {
  local label="$1"
  local page_url="$2"
  local case_dir="$3"
  local chrome_profile="$case_dir/chrome-profile"
  local dom_file="$case_dir/chrome.dom"
  local screenshot_file="$case_dir/chrome.png"
  local chrome_log="$case_dir/chrome.stderr"

  python3 - \
    "$page_url" \
    "$chrome_profile" \
    "$dom_file" \
    "$screenshot_file" \
    "$chrome_log" \
    "$label" <<'PY'
import json
from pathlib import Path
import sys
import time

from tests.browser.harness import BrowserFailure, ChromeSession, visible_expression


page_url, profile, dom_path, screenshot_path, chrome_log, label = sys.argv[1:]


def fail(message):
    raise BrowserFailure(message)


browser_session = None
devtools = None
session_id = None


def evaluate(expression):
    return browser_session.evaluate(expression)


def evaluate_json(expression):
    return browser_session.evaluate_json(expression)


def wait_for(expression, description, timeout=35):
    return browser_session.wait_for(expression, description, timeout=timeout)


def capture_artifacts():
    if browser_session is None:
        return
    try:
        browser_session.write_dom(dom_path)
    except Exception as error:  # Diagnostics must not hide the original error.
        Path(dom_path).write_text(f"Could not collect DOM: {error!r}\n")
    try:
        browser_session.capture_screenshot(screenshot_path)
    except Exception as error:  # Diagnostics must not hide the original error.
        Path(screenshot_path + ".error").write_text(
            f"Could not collect screenshot: {error!r}\n",
        )


def stop_browser():
    global browser_session
    if browser_session is not None:
        browser_session.close()
        browser_session = None


def assert_desktop_contract():
    desktop = evaluate_json("""
(() => {
  const visible = (selector) => {
    const element = document.querySelector(selector);
    if (!element) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return !element.hidden && style.display !== "none" && style.visibility !== "hidden" &&
      rect.width > 0 && rect.height > 0;
  };
  const first = (selectors) => selectors.map((selector) => document.querySelector(selector)).find(Boolean);
  const content = first(["#left-pane", ".content-shell", ".main-content"]);
  const console = document.querySelector(".console-pane");
  const title = first(["#document-title", ".mech-document-content h1", ".main-content h1"]);
  const root = document.querySelector(".mech-root");
  const numericTocLink = [...document.querySelectorAll(".mech-toc a[href^='#'], .toc a[href^='#'], [data-mech-toc] a[href^='#']")]
    .find((link) => /^#\\d/.test(link.getAttribute("href") || ""));
  const numericToc = (() => {
    if (!numericTocLink) return null;
    const fragment = (numericTocLink.getAttribute("href") || "").slice(1);
    const target = document.getElementById(fragment);
    const event = new MouseEvent("click", { bubbles: true, cancelable: true });
    numericTocLink.dispatchEvent(event);
    return {
      fragment,
      targetExists: Boolean(target),
      clickPrevented: event.defaultPrevented,
    };
  })();
  const rectangle = (element) => {
    if (!element) return null;
    const rect = element.getBoundingClientRect();
    return { left: rect.left, right: rect.right, width: rect.width, height: rect.height };
  };
  return {
    status: document.documentElement.dataset.mechDocumentStatus,
    rootStatus: root?.dataset.mechDocumentStatus,
    errors: [
      document.documentElement.dataset.mechDocumentError,
      document.documentElement.dataset.mechWindowError,
      document.documentElement.dataset.mechUnhandledRejection,
    ].filter(Boolean),
    siteChromeAbsent:
      !document.querySelector(".site-header, .footer, .breadcrumbs, .mika-separator, .post-pagination") &&
      [...document.querySelectorAll("#toggle-repl, [data-mech-console-toggle]")]
        .every(control => control.id === "mech-smoke-custom-toggle"),
    titleVisible: Boolean(title && visible("#document-title, .mech-document-content h1, .main-content h1")),
    contentVisible: Boolean(content && visible("#left-pane, .content-shell, .main-content")),
    console: rectangle(console),
    consoleVisible: visible(".console-pane"),
    tabs: console?.querySelectorAll(".console-tab").length || 0,
    tabOrder: [...(console?.querySelectorAll('[data-mech-console-tab]') || [])]
      .map(tab => tab.dataset.mechConsoleTab),
    consoleTabActive: Boolean(document.querySelector("[data-mech-console-tab='console'][aria-selected='true']")),
    promptVisible: visible(".repl-prompt"),
    inputVisible: visible(".repl-input"),
    outputIsPlaceholder: /under construction/i.test(document.querySelector("[data-mech-output-panel]")?.textContent || ""),
    errorsIsPlaceholder: /under construction/i.test(document.querySelector("[data-mech-errors-panel]")?.textContent || ""),
    resizerVisible: visible("[data-mech-repl-host] > [data-mech-console-resizer]:not([data-mech-console-edge-handle])"),
    fullscreenVisible: visible("[data-mech-console-fullscreen]"),
    outputFullscreenVisible: visible("[data-mech-output-fullscreen]"),
    tocLinks: document.querySelectorAll(".mech-toc a[href], .toc a[href], [data-mech-toc] a[href]").length,
    numericToc,
    citationsVisible: visible(".mech-works-cited"),
    footnotesVisible: visible(".mech-footnotes"),
    blockOutput: [...document.querySelectorAll(".mech-block-output")]
      .some((element) => element.textContent.trim() && element.getBoundingClientRect().height > 0),
    inlineOutput: [...document.querySelectorAll(".mech-inline-mech-code")]
      .some((element) => /42/.test(element.textContent) && element.getBoundingClientRect().height > 0),
    variableHydrated: Boolean(document.querySelector("#mech-smoke-var .mech-var-placeholder")?.textContent.trim()),
    scrollWidth: document.documentElement.scrollWidth,
    viewportWidth: window.innerWidth,
    label: document.title,
  };
})()
""")
    if desktop["status"] != "ready" or desktop["rootStatus"] != "ready":
        fail(f"document did not become ready: {desktop!r}")
    if desktop["errors"]:
        fail(f"document reported browser errors: {desktop!r}")
    for name in (
        "siteChromeAbsent", "titleVisible", "contentVisible", "consoleVisible",
        "consoleTabActive", "promptVisible", "inputVisible", "resizerVisible",
        "fullscreenVisible", "outputFullscreenVisible", "citationsVisible", "footnotesVisible", "blockOutput",
        "inlineOutput", "variableHydrated",
    ):
        if not desktop[name]:
            fail(f"desktop rich-document contract failed for {name}: {desktop!r}")
    if desktop["tabs"] != 3:
        fail(f"expected exactly three console tabs: {desktop!r}")
    if desktop["tabOrder"] != ["output", "console", "errors"]:
        fail(f"console tabs did not follow Output, Console, Errors: {desktop!r}")
    if desktop["outputIsPlaceholder"] or desktop["errorsIsPlaceholder"]:
        fail(f"rich console still contains an unfinished placeholder: {desktop!r}")
    if desktop["tocLinks"] < 2:
        fail(f"table of contents is missing fixture section links: {desktop!r}")
    if not desktop["numericToc"] or not desktop["numericToc"]["targetExists"] or not desktop["numericToc"]["clickPrevented"]:
        fail(f"numeric table-of-contents fragments did not resolve through the document controller: {desktop!r}")
    if desktop["console"] is None or desktop["console"]["width"] < 300:
        fail(f"desktop console is too narrow: {desktop!r}")
    content = evaluate_json("""
(() => {
  const element = document.querySelector("#left-pane, .content-shell, .main-content");
  if (!element) return null;
  const rect = element.getBoundingClientRect();
  return { left: rect.left, right: rect.right, width: rect.width };
})()
""")
    if content is None or content["width"] < 600:
        fail(f"desktop content is too narrow: {content!r}, {desktop!r}")
    console = desktop["console"]
    if not (content["right"] <= console["left"] + 1 or console["right"] <= content["left"] + 1):
        fail(f"desktop content and console overlap: content={content!r}, console={console!r}")
    if desktop["scrollWidth"] > desktop["viewportWidth"] + 1:
        fail(f"desktop page overflows horizontally: {desktop!r}")

    if "blog" in label:
        if not evaluate(visible_expression(".hero")) or not evaluate(visible_expression(".mech-meta")):
            fail("blog shell did not render a visible hero and metadata")
    if "docs" in label:
        if not evaluate(visible_expression(".version-badge")):
            fail("docs shell did not render visible version metadata")
def assert_style_layer_contract():
    layers = evaluate_json("""
(async () => {
  const styles = Object.fromEntries(
    [...document.querySelectorAll('style[data-mech-style-layer]')]
      .map(style => [style.dataset.mechStyleLayer, style])
  );
  const token = document.querySelector(
    '[data-mech-source] .mech-number, [data-mech-source].mech-number'
  );
  const heading = document.querySelector('[data-mechdown] h2');
  const subheading = document.querySelector('[data-mechdown] h3:not(.mech-backmatter-heading)');
  const abstract = document.querySelector('[data-mechdown] .mech-abstract');
  const header = document.createElement('header');
  header.className = 'site-header';
  header.dataset.mechTestPageChrome = 'true';
  document.body.prepend(header);
  window.dispatchEvent(new CustomEvent('mech:styles-changed'));
  await new Promise(resolve => requestAnimationFrame(resolve));
  await new Promise(resolve => requestAnimationFrame(resolve));
  const consolePane = document.querySelector('.console-pane');
  const prompt = document.querySelector('.repl-prompt');
  if (
    Object.keys(styles).length !== 4 || !token || !heading || !header ||
    !consolePane || !prompt
  ) return null;

  const sourceMarkup = token.parentElement?.innerHTML || '';
  const sourceText = token.parentElement?.textContent || '';
  const sourceColor = getComputedStyle(token).color;
  const headingFont = getComputedStyle(heading).fontFamily;
  const headingDisplay = getComputedStyle(heading).display;
  const headingBefore = getComputedStyle(heading, '::before').content;
  const subheadingBefore = subheading
    ? getComputedStyle(subheading, '::before').content
    : null;
  const abstractStyle = abstract ? {
    borderWidth: parseFloat(getComputedStyle(abstract).borderTopWidth),
    paddingLeft: parseFloat(getComputedStyle(abstract).paddingLeft),
    background: getComputedStyle(abstract).backgroundColor,
  } : null;
  const headerPosition = getComputedStyle(header).position;
  const consoleDisplay = getComputedStyle(consolePane).display;
  const consolePosition = getComputedStyle(consolePane).position;
  const consoleHeight = consolePane.getBoundingClientRect().height;
  const consoleWidth = consolePane.getBoundingClientRect().width;
  const promptColor = getComputedStyle(prompt).color;
  const initialScrollY = window.scrollY;
  const initialInlineScrollBehavior = document.documentElement.style.scrollBehavior;

  styles.page.disabled = true;
  document.documentElement.style.scrollBehavior = 'auto';
  await new Promise(resolve => requestAnimationFrame(resolve));
  await new Promise(resolve => requestAnimationFrame(resolve));
  const pageOffBeforeScroll = {
    consoleTopStyle: getComputedStyle(consolePane).top,
    hostOffset: getComputedStyle(
      document.querySelector('[data-mech-repl-host]')
    ).getPropertyValue('--mech-repl-top-offset').trim(),
  };
  const headerHeight = header.getBoundingClientRect().height;
  window.scrollTo(0, Math.min(
    headerHeight + 64,
    Math.max(0, document.documentElement.scrollHeight - innerHeight),
  ));
  await new Promise(resolve => requestAnimationFrame(resolve));
  const consoleRect = consolePane.getBoundingClientRect();
  const pageOff = {
    headerPosition: getComputedStyle(header).position,
    headerBottom: header.getBoundingClientRect().bottom,
    sourceColor: getComputedStyle(token).color,
    consoleDisplay: getComputedStyle(consolePane).display,
    consolePosition: getComputedStyle(consolePane).position,
    consoleTop: consoleRect.top,
    consoleRight: consoleRect.right,
    consoleHeight: consoleRect.height,
    consoleWidth: consoleRect.width,
    viewportWidth: innerWidth,
    consoleBoxSizing: getComputedStyle(consolePane).boxSizing,
    consoleTopStyle: getComputedStyle(consolePane).top,
    promptColor: getComputedStyle(prompt).color,
  };
  styles.page.disabled = false;
  window.scrollTo(0, initialScrollY);
  await new Promise(resolve => requestAnimationFrame(resolve));
  document.documentElement.style.scrollBehavior = initialInlineScrollBehavior;

  styles.mechdown.disabled = true;
  const mechdownOff = {
    headingDisplay: getComputedStyle(heading).display,
    headingFont: getComputedStyle(heading).fontFamily,
    sourceColor: getComputedStyle(token).color,
  };
  styles.mechdown.disabled = false;

  styles.source.disabled = true;
  const sourceOff = {
    color: getComputedStyle(token).color,
    markup: token.parentElement?.innerHTML || '',
    text: token.parentElement?.textContent || '',
    connected: token.isConnected,
  };
  styles.source.disabled = false;

  styles.repl.disabled = true;
  const replOff = {
    consoleDisplay: getComputedStyle(consolePane).display,
    promptColor: getComputedStyle(prompt).color,
  };
  styles.repl.disabled = false;

  header.remove();
  window.dispatchEvent(new CustomEvent('mech:styles-changed'));
  await new Promise(resolve => requestAnimationFrame(resolve));

  return {
    sourceMarkup,
    sourceText,
    sourceColor,
    headingFont,
    headingDisplay,
    headingBefore,
    subheadingBefore,
    abstractStyle,
    headerPosition,
    consoleDisplay,
    consolePosition,
    consoleHeight,
    consoleWidth,
    promptColor,
    pageOffBeforeScroll,
    pageOff,
    mechdownOff,
    sourceOff,
    replOff,
  };
})()
""")
    if (
        layers is None or
        layers["headerPosition"] != "sticky" or
        layers["headingDisplay"] != "flex" or
        "section " not in layers["headingBefore"].lower() or
        "mechdown-section" not in layers["headingBefore"] or
        (
            layers["subheadingBefore"] is not None and
            (
                "mechdown-section" not in layers["subheadingBefore"] or
                "mechdown-subsection" not in layers["subheadingBefore"]
            )
        ) or
        (
            layers["abstractStyle"] is not None and
            (
                layers["abstractStyle"]["borderWidth"] < 1 or
                layers["abstractStyle"]["paddingLeft"] < 16 or
                layers["abstractStyle"]["background"] == "rgba(0, 0, 0, 0)"
            )
        ) or
        layers["pageOff"]["headerPosition"] == layers["headerPosition"] or
        layers["pageOffBeforeScroll"]["consoleTopStyle"] != "0px" or
        layers["pageOffBeforeScroll"]["hostOffset"] != "0px" or
        layers["pageOff"]["sourceColor"] != layers["sourceColor"] or
        layers["pageOff"]["consoleDisplay"] != layers["consoleDisplay"] or
        layers["pageOff"]["consolePosition"] != layers["consolePosition"] or
        layers["pageOff"]["headerBottom"] > 1 or
        abs(layers["pageOff"]["consoleTop"]) > 1 or
        layers["pageOff"]["consoleTopStyle"] != "0px" or
        layers["pageOff"]["consoleBoxSizing"] != "border-box" or
        layers["pageOff"]["consoleRight"] > layers["pageOff"]["viewportWidth"] + 1 or
        layers["pageOff"]["consoleHeight"] <= layers["consoleHeight"] or
        abs(layers["pageOff"]["consoleWidth"] - layers["consoleWidth"]) > 1 or
        layers["pageOff"]["promptColor"] != layers["promptColor"] or
        layers["mechdownOff"]["headingDisplay"] != "block" or
        layers["mechdownOff"]["headingFont"] == layers["headingFont"] or
        layers["mechdownOff"]["sourceColor"] != layers["sourceColor"] or
        layers["sourceOff"]["color"] == layers["sourceColor"] or
        layers["sourceOff"]["markup"] != layers["sourceMarkup"] or
        layers["sourceOff"]["text"] != layers["sourceText"] or
        not layers["sourceOff"]["connected"] or
        (
            layers["replOff"]["consoleDisplay"] == layers["consoleDisplay"] and
            layers["replOff"]["promptColor"] == layers["promptColor"]
        )
    ):
        fail(f"independent style-layer contract regressed: {layers!r}")


def assert_desktop_console_controls():
    state = evaluate_json("""
(() => {
  const root = document.querySelector(".mech-root");
  const pane = document.querySelector(".console-pane");
  return {
    rootOpen: root?.dataset.mechConsoleOpen,
    paneHidden: pane?.hidden,
    exteriorButtonAbsent: !document.querySelector("#toggle-repl, [data-mech-console-toggle]"),
  };
})()
""")
    if state["rootOpen"] != "true" or state["paneHidden"]:
        fail(f"desktop console did not begin in an accessible open state: {state!r}")
    if label == "custom":
        custom = evaluate_json("""
(() => {
  const toggle = document.querySelector('#mech-smoke-custom-toggle');
  const visible = element => {
    const rect = element?.getBoundingClientRect();
    return Boolean(element && getComputedStyle(element).display !== 'none' && rect.width && rect.height);
  };
  return {
    visible: visible(toggle),
    panels: Object.fromEntries(['console', 'output', 'errors'].map(name => [
      name,
      document.querySelector(`[data-mech-console-panel="${name}"]`)?.dataset.mechConsolePanel || null,
    ])),
  };
})()
""")
        if custom != {
            "visible": True,
            "panels": {"console": "console", "output": "output", "errors": "errors"},
        }:
            fail(f"custom REPL controls were not preserved alongside the canonical component: {custom!r}")
        evaluate("document.querySelector('#mech-smoke-custom-toggle')?.click()")
        wait_for(
            "document.querySelector('.mech-root')?.dataset.mechConsoleOpen === 'false'",
            "the custom component toggle closing the console",
        )
        evaluate("document.querySelector('#mech-smoke-custom-toggle')?.click()")
        wait_for(
            "document.querySelector('.mech-root')?.dataset.mechConsoleOpen === 'true'",
            "the custom component toggle reopening the console",
        )
    elif not state["exteriorButtonAbsent"]:
        fail(f"a shipped shim exposed an exterior Console button: {state!r}")

    evaluate("document.dispatchEvent(new KeyboardEvent('keydown', {key: '`', bubbles: true, cancelable: true}))")
    wait_for(
        "document.querySelector('.mech-root')?.dataset.mechConsoleOpen === 'false' && "
        "document.querySelector('.console-pane')?.hidden === true && "
        "getComputedStyle(document.querySelector('[data-mech-console-edge-handle]')).display !== 'none' && "
        "getComputedStyle(document.querySelector('[data-mech-console-edge-handle]')).opacity === '1'",
        "the desktop console closing with a visible edge affordance",
    )
    edge_before = evaluate_json("""
(() => {
  const edge = document.querySelector('[data-mech-console-edge-handle]');
  const rect = edge?.getBoundingClientRect();
  const style = edge && getComputedStyle(edge);
  return edge && rect && style ? {
    x: Math.min(innerWidth - 1, Math.max(0, rect.left + rect.width / 2)),
    y: rect.top + rect.height / 2,
    right: parseFloat(style.right),
    visiblePixels: innerWidth - rect.left,
    animated: style.transitionProperty.split(',').map(value => value.trim()).includes('right'),
  } : null;
})()
""")
    if (
        edge_before is None or
        edge_before["visiblePixels"] < 6 or
        not edge_before["animated"]
    ):
        fail(f"the collapsed console edge handle was not visibly reversible: {edge_before!r}")
    devtools.call(
        "Input.dispatchMouseEvent",
        {"type": "mouseMoved", "x": edge_before["x"], "y": edge_before["y"]},
        session_id,
    )
    time.sleep(0.15)
    edge_after = evaluate("parseFloat(getComputedStyle(document.querySelector('[data-mech-console-edge-handle]')).right)")
    if edge_after <= edge_before["right"]:
        fail(
            "the collapsed console edge handle did not pop out on hover: "
            f"before={edge_before!r}, after={edge_after!r}"
        )

    terminal_toggle = evaluate_json("""
(() => {
  const input = document.querySelector('.repl-input');
  if (!input) return null;
  input.value = 'terminal-draft';
  input.focus();
  const event = new KeyboardEvent('keydown', {key: '`', bubbles: true, cancelable: true});
  input.dispatchEvent(event);
  return { prevented: event.defaultPrevented, value: input.value };
})()
""")
    if terminal_toggle != {"prevented": True, "value": "terminal-draft"}:
        fail(f"backtick was not captured from the active terminal prompt: {terminal_toggle!r}")
    wait_for(
        "document.querySelector('.mech-root')?.dataset.mechConsoleOpen === 'true' && "
        "document.querySelector('.console-pane')?.hidden === false",
        "the desktop console reopening through its keyboard command",
    )


def assert_output_fullscreen_control():
    initial = evaluate_json("""
(() => {
  const toggle = document.querySelector('button[data-mech-output-fullscreen]');
  return toggle ? {
    pressed: toggle.getAttribute('aria-pressed'),
    label: toggle.getAttribute('aria-label'),
  } : null;
})()
""")
    if initial != {"pressed": "false", "label": "Enter fullscreen output"}:
        fail(f"output fullscreen control did not begin independently inactive: {initial!r}")

    evaluate("document.querySelector('button[data-mech-output-fullscreen]')?.click()")
    wait_for(
        "document.querySelector('[data-mech-repl-host]')?.dataset.mechOutputFullscreenActive === 'true' && "
        "document.querySelector('button[data-mech-output-fullscreen]')?.getAttribute('aria-pressed') === 'true'",
        "the output-only fullscreen control entering",
    )
    fullscreen = evaluate_json("""
(() => {
  const root = document.querySelector('[data-mech-repl-host]');
  const pane = document.querySelector('[data-mech-console-pane]');
  const output = document.querySelector('[data-mech-console-panel="output"]');
  const content = root?.querySelector(':scope > .content-shell, :scope > .content, :scope > #left-pane');
  const outputToggle = document.querySelector('button[data-mech-output-fullscreen]');
  const consoleToggle = document.querySelector('[data-mech-console-fullscreen]');
  const rect = output?.getBoundingClientRect();
  const visible = element => Boolean(element && getComputedStyle(element).display !== 'none' &&
    element.getBoundingClientRect().width > 0 && element.getBoundingClientRect().height > 0);
  return {
    outputOnly: [...document.querySelectorAll('[data-mech-console-panel]')].every(panel =>
      visible(panel) === (panel.dataset.mechConsolePanel === 'output')),
    fillsViewport: Boolean(rect) && rect.left <= 1 && rect.top <= 1 &&
      rect.right >= innerWidth - 1 && rect.bottom >= innerHeight - 1,
    contentHidden: !visible(content),
    outputExitVisible: visible(outputToggle) &&
      outputToggle.getAttribute('aria-label') === 'Exit fullscreen output',
    consoleFullscreenHidden: !visible(consoleToggle),
    distinctFromWorkspace: document.querySelector('[data-mech-repl-host]')?.dataset.mechConsoleMode === 'docked' &&
      document.body.classList.contains('output-fullscreen'),
  };
})()
""")
    if not all(fullscreen.values()):
        fail(f"output fullscreen did not remain an independent output-only surface: {fullscreen!r}")

    def render_fixed_surface(block_html):
        rendered = json.dumps({
            "name": None,
            "kind": "scene/test",
            "selectionToken": "selection:fullscreen-fill",
            "inlineHtml": "scene/test",
            "blockHtml": block_html,
        })
        return evaluate_json(f"""
(async () => {{
  const {{ WasmDocument }} = await import('/_mech/pkg/mech_wasm.js');
  const prototype = WasmDocument.prototype;
  window.__MECH_SMOKE_ORIGINAL_PROGRAM_OUTPUT__ ||=
    prototype.renderedProgramOutput;
  prototype.renderedProgramOutput = function() {{ return {rendered}; }};
  const controller = globalThis.MechDocumentController;
  controller.replaceSource(controller.source());
  return true;
}})()
""")

    def measure_fixed_surface(selector):
        return evaluate_json(f"""
(() => {{
  const panel = document.querySelector('[data-mech-output-panel]');
  const entry = panel?.querySelector(
    '[data-mech-output-region="document"] .mech-document-output-entry');
  const body = entry?.querySelector('[data-mech-output-fill="true"]');
  const surface = body?.querySelector({json.dumps(selector)});
  const panelRect = panel?.getBoundingClientRect();
  const entryRect = entry?.getBoundingClientRect();
  const bodyRect = body?.getBoundingClientRect();
  const surfaceRect = surface?.getBoundingClientRect();
  return panelRect && entryRect && bodyRect && surfaceRect ? {{
    entryFillsPanel: entryRect.height >= panelRect.height - 1,
    fillsBody: Math.abs(bodyRect.height - surfaceRect.height) <= 1,
    fillsUsefulViewport: surfaceRect.height >= innerHeight * 0.8,
    noNestedScroll:
      body.scrollHeight <= body.clientHeight + 1 &&
      body.scrollWidth <= body.clientWidth + 1 &&
      surface.scrollHeight <= surface.clientHeight + 1 &&
      surface.scrollWidth <= surface.clientWidth + 1,
  }} : null;
}})()
""")

    fill_geometry = {}
    try:
        render_fixed_surface('<canvas width="120" height="80"></canvas>')
        wait_for(
            "Boolean(document.querySelector('[data-mech-output-region=document] "
            ".mech-document-output-entry [data-mech-output-fill=true] canvas'))",
            "a canvas-backed fixed program Output",
        )
        fill_geometry["canvas"] = measure_fixed_surface("canvas")
        render_fixed_surface(
            '<svg class="mech-repl-scene" viewBox="0 0 120 80" '
            'width="120" height="80" role="img"></svg>',
        )
        wait_for(
            "Boolean(document.querySelector('[data-mech-output-region=document] "
            ".mech-document-output-entry [data-mech-output-fill=true] .mech-repl-scene'))",
            "an SVG-backed fixed program Output",
        )
        fill_geometry["svg"] = measure_fixed_surface(".mech-repl-scene")
    finally:
        evaluate_json("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  const original = window.__MECH_SMOKE_ORIGINAL_PROGRAM_OUTPUT__;
  if (!original) return false;
  WasmDocument.prototype.renderedProgramOutput = original;
  delete window.__MECH_SMOKE_ORIGINAL_PROGRAM_OUTPUT__;
  const controller = globalThis.MechDocumentController;
  controller.replaceSource(controller.source());
  return true;
})()
""")
    if any(surface is None or not all(surface.values()) for surface in fill_geometry.values()):
        fail(f"fullscreen fill geometry did not reach fixed program surfaces: {fill_geometry!r}")

    evaluate("""
(() => {
  window.dispatchEvent(new CustomEvent('mech:output', { detail: {
    stream: 'stdout', operation: 'create', display_id: 'fullscreen-long-text',
    content: {kind: 'text', data: {text: Array.from(
      {length: 240}, (_, index) => `text output line ${index + 1}`).join('\\n')}},
  }}));
})()
""")
    wait_for(
        "document.querySelector('[data-mech-display-id=\"fullscreen-long-text\"]') !== null",
        "long textual output in output fullscreen",
    )
    text_scroll = evaluate_json("""
(() => {
  const panel = document.querySelector('[data-mech-output-panel]');
  const style = panel && getComputedStyle(panel);
  return panel ? {
    overflow: style.overflowY,
    scrollable: panel.scrollHeight > panel.clientHeight,
  } : null;
})()
""")
    if (
        text_scroll is None or
        text_scroll["overflow"] not in {"auto", "scroll"} or
        not text_scroll["scrollable"]
    ):
        fail(f"fullscreen textual output did not retain the panel scroll surface: {text_scroll!r}")
    evaluate("""
window.dispatchEvent(new CustomEvent('mech:output', { detail: {
  stream: 'stdout', operation: 'remove', display_id: 'fullscreen-long-text',
}}))
""")

    captured = evaluate_json("""
(() => {
  const input = document.querySelector('.repl-input');
  if (!input) return null;
  input.value = 'output-fullscreen-draft';
  const event = new KeyboardEvent('keydown', {key: '`', bubbles: true, cancelable: true});
  input.dispatchEvent(event);
  return { prevented: event.defaultPrevented, value: input.value };
})()
""")
    if captured != {"prevented": True, "value": "output-fullscreen-draft"}:
        fail(f"backtick leaked into the prompt while leaving output fullscreen: {captured!r}")
    wait_for(
        "document.querySelector('[data-mech-repl-host]')?.dataset.mechOutputFullscreenActive === 'false' && "
        "document.querySelector('[data-mech-repl-host]')?.dataset.mechConsoleOpen === 'true' && "
        "document.querySelector(\"[data-mech-console-tab='output']\")?.getAttribute('aria-selected') === 'true' && "
        "getComputedStyle(document.querySelector('.content-shell, .content, #left-pane')).display !== 'none'",
        "backtick revealing the editor with Output selected",
    )

    evaluate("""
(() => {
  const pane = document.querySelector('[data-mech-console-pane]');
  const toggle = document.querySelector('button[data-mech-output-fullscreen]');
  const pending = {native: false, exitCalls: 0, resolve: null};
  window.__mechPendingOutputFullscreen = pending;
  Object.defineProperty(document, 'fullscreenElement', {
    configurable: true,
    get: () => pending.native ? pane : null,
  });
  Object.defineProperty(document, 'exitFullscreen', {
    configurable: true,
    value: async () => {
      pending.exitCalls += 1;
      pending.native = false;
      document.dispatchEvent(new Event('fullscreenchange'));
    },
  });
  Object.defineProperty(pane, 'requestFullscreen', {
    configurable: true,
    value: () => new Promise(resolve => {
      pending.resolve = () => {
        pending.native = true;
        document.dispatchEvent(new Event('fullscreenchange'));
        resolve();
      };
    }),
  });
  toggle.click();
})()
""")
    wait_for(
        "document.querySelector('[data-mech-repl-host]')?.dataset.mechOutputFullscreenActive === 'true' && "
        "typeof window.__mechPendingOutputFullscreen?.resolve === 'function'",
        "a pending output fullscreen request",
    )
    evaluate("document.querySelector('button[data-mech-output-fullscreen]')?.click()")
    wait_for(
        "document.querySelector('[data-mech-repl-host]')?.dataset.mechOutputFullscreenActive === 'false'",
        "canceling a pending output fullscreen request",
    )
    evaluate("window.__mechPendingOutputFullscreen.resolve()")
    wait_for(
        "window.__mechPendingOutputFullscreen?.exitCalls === 1 && "
        "document.fullscreenElement === null && "
        "document.querySelector('[data-mech-repl-host]')?.dataset.mechOutputFullscreenActive === 'false' && "
        "document.querySelector('[data-mech-repl-host]')?.dataset.mechConsoleMode === 'docked'",
        "the stale fullscreen completion being relinquished",
    )
    evaluate("""
(() => {
  const pane = document.querySelector('[data-mech-console-pane]');
  delete pane.requestFullscreen;
  delete document.exitFullscreen;
  delete document.fullscreenElement;
  delete window.__mechPendingOutputFullscreen;
})()
""")

    evaluate("""
(() => {
  const pane = document.querySelector('[data-mech-console-pane]');
  const outputToggle = document.querySelector('button[data-mech-output-fullscreen]');
  const consoleToggle = document.querySelector('button[data-mech-console-fullscreen]');
  const pending = {native: false, exitCalls: 0, resolves: []};
  window.__mechCollidingFullscreen = pending;
  Object.defineProperty(document, 'fullscreenElement', {
    configurable: true,
    get: () => pending.native ? pane : null,
  });
  Object.defineProperty(document, 'exitFullscreen', {
    configurable: true,
    value: async () => {
      pending.exitCalls += 1;
      pending.native = false;
      document.dispatchEvent(new Event('fullscreenchange'));
    },
  });
  Object.defineProperty(pane, 'requestFullscreen', {
    configurable: true,
    value: () => new Promise(resolve => {
      pending.resolves.push(() => {
        pending.native = true;
        document.dispatchEvent(new Event('fullscreenchange'));
        resolve();
      });
    }),
  });
  outputToggle.click();
  outputToggle.click();
  consoleToggle.click();
})()
""")
    wait_for(
        "window.__mechCollidingFullscreen?.resolves.length === 2 && "
        "document.querySelector('[data-mech-repl-host]')?.dataset.mechOutputFullscreenActive === 'false'",
        "a successor console request after canceling output fullscreen",
    )
    evaluate("window.__mechCollidingFullscreen.resolves[0]()")
    wait_for(
        "window.__mechCollidingFullscreen?.exitCalls === 0 && "
        "document.fullscreenElement === document.querySelector('[data-mech-console-pane]') && "
        "document.querySelector('[data-mech-repl-host]')?.dataset.mechConsoleMode === 'button'",
        "the stale output completion preserving its successor's fullscreen ownership",
    )
    evaluate("window.__mechCollidingFullscreen.resolves[1]()")
    evaluate("document.querySelector('button[data-mech-console-fullscreen]')?.click()")
    wait_for(
        "window.__mechCollidingFullscreen?.exitCalls === 1 && document.fullscreenElement === null",
        "the successor console owner exiting its own native fullscreen session",
    )
    evaluate("""
(() => {
  const pane = document.querySelector('[data-mech-console-pane]');
  delete pane.requestFullscreen;
  delete document.exitFullscreen;
  delete document.fullscreenElement;
  delete window.__mechCollidingFullscreen;
})()
""")

    devtools.call(
        "Emulation.setDeviceMetricsOverride",
        {"width": 700, "height": 700, "deviceScaleFactor": 1, "mobile": False},
        session_id,
    )
    evaluate("""
(() => {
  const pane = document.querySelector('[data-mech-console-pane]');
  Object.defineProperty(pane, 'requestFullscreen', {
    configurable: true,
    value: undefined,
  });
  document.querySelector('button[data-mech-output-fullscreen]')?.click();
})()
""")
    wait_for(
        "document.querySelector('[data-mech-repl-host]')?.dataset.mechOutputFullscreenActive === 'true'",
        "compact output fullscreen fallback",
    )
    compact_fallback = evaluate_json("""
(() => {
  const pane = document.querySelector('[data-mech-console-pane]');
  const rect = pane?.getBoundingClientRect();
  return rect ? {
    fillsWidth: rect.left <= 1 && rect.right >= innerWidth - 1,
    fillsHeight: rect.top <= 1 && rect.bottom >= innerHeight - 1,
  } : null;
})()
""")
    if compact_fallback is None or not all(compact_fallback.values()):
        fail(f"compact output fullscreen fallback remained a drawer: {compact_fallback!r}")
    evaluate("document.querySelector('button[data-mech-output-fullscreen]')?.click()")
    wait_for(
        "document.querySelector('[data-mech-repl-host]')?.dataset.mechOutputFullscreenActive === 'false'",
        "compact output fullscreen fallback exit",
    )
    evaluate("delete document.querySelector('[data-mech-console-pane]').requestFullscreen")
    devtools.call(
        "Emulation.setDeviceMetricsOverride",
        {"width": 1680, "height": 900, "deviceScaleFactor": 1, "mobile": False},
        session_id,
    )
    evaluate("document.querySelector(\"[data-mech-console-tab='console']\")?.click()")


def assert_fullscreen_accessibility():
    evaluate("""
(() => {
  const publish = detail => window.dispatchEvent(new CustomEvent('mech:output', { detail }));
  publish({
    stream: 'stdout', operation: 'create', display_id: 'unread-output',
    content: {
      kind: 'value',
      data: {
        kind: `record<${'planet-state<f64> '.repeat(30)}>`,
        text: 'new output activity',
      },
    },
  });
  publish({
    stream: 'stderr', operation: 'create', display_id: 'unread-error',
    content: {
      kind: 'value',
      data: {
        kind: `record<${'sensor-failure<f64> '.repeat(30)}>`,
        text: 'new error activity',
      },
    },
  });
})()
""")
    initial = evaluate_json("""
(() => {
  const toggle = document.querySelector("[data-mech-console-fullscreen]");
  const resizers = [...document.querySelectorAll('[data-mech-console-workspace-resizer]')];
  return {
    pressed: toggle?.getAttribute("aria-pressed"),
    label: toggle?.getAttribute("aria-label"),
    unreadCreated: ['output', 'errors'].every(name => {
      const tab = document.querySelector(`[data-mech-console-tab="${name}"]`);
      return tab?.dataset.mechConsoleUnread === 'true' &&
        /new activity/.test(tab.getAttribute('aria-label') || '') &&
        'mechConsoleBaseLabel' in tab.dataset;
    }),
    errorBadgeSynchronized: (() => {
      const tab = document.querySelector('[data-mech-console-tab="errors"]');
      const badge = tab?.querySelector('.mech-console-error-count');
      const panel = document.querySelector('[data-mech-errors-panel]');
      const count = panel?.querySelectorAll(
        ".mech-console-error:not(.mech-repl-diagnostic), " +
        ".mech-repl-diagnostic[data-mech-diagnostic-severity='error'], " +
        ".mech-repl-diagnostic[data-mech-diagnostic-severity='fatal']"
      ).length || 0;
      if (!tab || !badge || count !== 1) return false;
      const rect = badge.getBoundingClientRect();
      return !badge.hidden && badge.textContent === String(count) &&
        /1 error/.test(tab.getAttribute('aria-label') || '') &&
        Math.abs(rect.width - rect.height) <= 1 &&
        parseFloat(getComputedStyle(badge).borderRadius) >= rect.height / 2;
    })(),
    resizersInitialized: resizers.length === 2 && resizers.every(handle =>
      handle.hasAttribute('aria-valuemin') && handle.hasAttribute('aria-valuemax') &&
      handle.hasAttribute('aria-valuenow')),
    kindElided: (() => {
      const kind = document.querySelector(
        '[data-mech-display-id="unread-output"] [data-mech-kind-elided="true"]');
      const style = kind && getComputedStyle(kind);
      return Boolean(kind) && kind.textContent === kind.title &&
        kind.textContent.length > 96 && style.overflow === 'hidden' &&
        style.textOverflow === 'ellipsis' && style.maxWidth !== 'none';
    })(),
    errorKindElided: (() => {
      const kind = document.querySelector(
        '[data-mech-display-id="unread-error"] [data-mech-kind-elided="true"]');
      const style = kind && getComputedStyle(kind);
      return Boolean(kind) && kind.textContent === kind.title &&
        kind.textContent.length > 96 && style.overflow === 'hidden' &&
        style.textOverflow === 'ellipsis' && style.maxWidth !== 'none';
    })(),
  };
})()
""")
    if (
        initial["pressed"] != "false" or
        initial["label"] != "Enter fullscreen workspace" or
        not initial["unreadCreated"] or
        not initial["errorBadgeSynchronized"] or
        not initial["resizersInitialized"] or
        not initial["kindElided"] or
        not initial["errorKindElided"]
    ):
        fail(f"fullscreen control did not begin with a collapsed accessible state: {initial!r}")

    evaluate("document.querySelector('[data-mech-console-fullscreen]')?.click()")
    wait_for(
        "document.querySelector('[data-mech-console-fullscreen]')?.getAttribute('aria-pressed') === 'true' && "
        "document.querySelector('[data-mech-console-fullscreen]')?.getAttribute('aria-label') === 'Minimize console workspace'",
        "the fullscreen control entering an accessible active state",
    )
    if evaluate("document.fullscreenElement === document.querySelector('[data-mech-console-pane]')"):
        evaluate("document.exitFullscreen()")
        wait_for(
            "document.querySelector('[data-mech-console-fullscreen]')?.getAttribute('aria-pressed') === 'false' && "
            "document.querySelector('[data-mech-repl-host]')?.dataset.mechConsoleMode === 'docked'",
            "native fullscreen Escape/browser exit clearing button ownership",
        )
        evaluate("document.querySelector('[data-mech-console-fullscreen]')?.click()")
        wait_for(
            "document.querySelector('[data-mech-console-fullscreen]')?.getAttribute('aria-pressed') === 'true' && "
            "document.querySelector('[data-mech-repl-host]')?.dataset.mechConsoleMode === 'button'",
            "one click re-entering fullscreen after a native exit",
        )

    workspace = evaluate_json("""
(() => {
  const pane = document.querySelector('[data-mech-console-pane]');
  const consolePanel = document.querySelector('[data-mech-console-panel="console"]');
  const outputPanel = document.querySelector('[data-mech-console-panel="output"]');
  const errorsPanel = document.querySelector('[data-mech-console-panel="errors"]');
  const panels = document.querySelector('.console-panels');
  const tabs = [...document.querySelectorAll('[data-mech-console-tab]')];
  const column = document.querySelector('[data-mech-console-workspace-resizer="column"]');
  const row = document.querySelector('[data-mech-console-workspace-resizer="row"]');
  const mainHandle = document.querySelector(
    '[data-mech-repl-host] > [data-mech-console-resizer]:not([data-mech-console-edge-handle])');
  const edgeHandle = document.querySelector('[data-mech-console-edge-handle]');
  if (!pane || !panels || !consolePanel || !outputPanel || !errorsPanel || !column || !row) return null;
  const beforeConsoleWidth = consolePanel.getBoundingClientRect().width;
  const beforeOutputHeight = outputPanel.getBoundingClientRect().height;
  const drag = (handle, dx, dy, pointerId) => {
    const rect = handle.getBoundingClientRect();
    const x = rect.left + rect.width / 2;
    const y = rect.top + rect.height / 2;
    handle.dispatchEvent(new PointerEvent('pointerdown', {
      bubbles: true, cancelable: true, pointerId, button: 0, clientX: x, clientY: y,
    }));
    window.dispatchEvent(new PointerEvent('pointermove', {
      bubbles: true, pointerId, clientX: x + dx, clientY: y + dy,
    }));
    window.dispatchEvent(new PointerEvent('pointerup', {
      bubbles: true, pointerId, clientX: x + dx, clientY: y + dy,
    }));
  };
  drag(column, 48, 0, 301);
  drag(row, 0, 36, 302);
  const paneRect = pane.getBoundingClientRect();
  const consoleRect = consolePanel.getBoundingClientRect();
  const outputRect = outputPanel.getBoundingClientRect();
  const errorsRect = errorsPanel.getBoundingClientRect();
  const panelsRect = panels.getBoundingClientRect();
  return {
    fillsViewport: paneRect.left <= 1 && paneRect.top <= 1 &&
      paneRect.right >= innerWidth - 1 && paneRect.bottom >= innerHeight - 1,
    buttonFullscreenOwnsState:
      document.querySelector('[data-mech-repl-host]')?.dataset.mechConsoleMode === 'button',
    exteriorHandlesHidden: [mainHandle, edgeHandle].every(handle =>
      !handle || getComputedStyle(handle).display === 'none'),
    consoleLeft: consoleRect.left < outputRect.left,
    outputAboveErrors: outputRect.top < errorsRect.top && outputRect.bottom <= errorsRect.top + 1,
    rightAligned: Math.abs(outputRect.left - errorsRect.left) <= 1 &&
      Math.abs(outputRect.right - errorsRect.right) <= 1,
    allVisible: [consolePanel, outputPanel, errorsPanel].every(panel =>
      !panel.hidden && getComputedStyle(panel).display !== 'none' &&
      panel.getBoundingClientRect().width > 0 && panel.getBoundingClientRect().height > 0),
    tabsHidden: tabs.length === 3 && tabs.every(tab => getComputedStyle(tab).display === 'none'),
    panelsFillPane: Math.abs(panelsRect.top - paneRect.top) <= 1 &&
      Math.abs(panelsRect.bottom - paneRect.bottom) <= 1,
    resizersAccessible: [column, row].every(handle =>
      getComputedStyle(handle).display !== 'none' && handle.getAttribute('role') === 'separator' &&
      handle.tabIndex === 0 && handle.hasAttribute('aria-valuemin') &&
      handle.hasAttribute('aria-valuemax') && handle.hasAttribute('aria-valuenow')),
    unreadCleared: !document.querySelector('[data-mech-console-unread]'),
    unreadLabelsRestored: ['output', 'errors'].every(name => {
      const tab = document.querySelector(`[data-mech-console-tab="${name}"]`);
      const base = tab?.dataset.mechConsoleBaseLabel || '';
      const expected = name === 'errors' ? `${base}, 1 error` : base;
      return tab && !tab.dataset.mechConsoleUnread &&
        tab.getAttribute('aria-label') === expected;
    }),
    responsiveUnits: [
      pane.style.getPropertyValue('--mech-console-workspace-left'),
      pane.style.getPropertyValue('--mech-console-workspace-top'),
    ].every(value => value.endsWith('%')),
    labeled: [consolePanel, outputPanel, errorsPanel].every(panel =>
      getComputedStyle(panel, '::before').content.replaceAll('"', '') ===
        panel.dataset.mechConsoleLabel),
    delineated: [consolePanel, outputPanel, errorsPanel].every(panel => {
      const style = getComputedStyle(panel);
      return ['Top', 'Right', 'Bottom', 'Left'].every(side =>
        parseFloat(style[`border${side}Width`]) >= 1);
    }),
    resizeGrips: (() => {
      const columnGrip = getComputedStyle(column, '::after');
      const rowGrip = getComputedStyle(row, '::after');
      return parseFloat(columnGrip.height) <= 48 && parseFloat(columnGrip.width) <= 4 &&
        parseFloat(rowGrip.width) <= 48 && parseFloat(rowGrip.height) <= 4;
    })(),
    columnResized: consoleRect.width > beforeConsoleWidth + 20,
    rowResized: outputRect.height > beforeOutputHeight + 15,
  };
})()
""")
    if workspace is None or not all(workspace.values()):
        fail(f"fullscreen workspace did not expose or resize its three-pane layout: {workspace!r}")

    evaluate("""
(() => {
  const pane = document.querySelector('[data-mech-console-pane]');
  pane?.style.setProperty('--mech-console-workspace-left', '99%');
  pane?.style.setProperty('--mech-console-workspace-top', '99%');
})()
""")
    devtools.call(
        "Emulation.setDeviceMetricsOverride",
        {"width": 700, "height": 460, "deviceScaleFactor": 1, "mobile": False},
        session_id,
    )
    time.sleep(0.15)
    responsive = evaluate_json("""
(() => {
  const pane = document.querySelector('[data-mech-console-pane]');
  const panels = [...document.querySelectorAll('[data-mech-console-panel]')];
  const bounds = pane?.getBoundingClientRect();
  const workspace = pane?.querySelector(':scope > .console-panels');
  const workspaceBounds = workspace?.getBoundingClientRect();
  const consolePanel = document.querySelector('[data-mech-console-panel="console"]');
  const outputPanel = document.querySelector('[data-mech-console-panel="output"]');
  const separators = [...document.querySelectorAll('[data-mech-console-workspace-resizer]')];
  const ariaMatchesGeometry = Boolean(workspaceBounds) && separators.every(handle => {
    const column = handle.dataset.mechConsoleWorkspaceResizer === 'column';
    const total = column ? workspaceBounds.width : workspaceBounds.height;
    const minimumPixels = Math.min(column ? 180 : 120, Math.max(0, total / 2 - 4));
    const maximumPixels = Math.max(minimumPixels, total - minimumPixels - 8);
    const size = column
      ? consolePanel?.getBoundingClientRect().width || 0
      : outputPanel?.getBoundingClientRect().height || 0;
    const percentage = value => Math.round((value / total) * 100);
    return Number(handle.getAttribute('aria-valuemin')) === percentage(minimumPixels) &&
      Number(handle.getAttribute('aria-valuemax')) === percentage(maximumPixels) &&
      Number(handle.getAttribute('aria-valuenow')) === percentage(size);
  });
  return {
    contained: Boolean(bounds) && panels.every(panel => {
      const rect = panel.getBoundingClientRect();
      return rect.left >= bounds.left - 1 && rect.right <= bounds.right + 1 &&
        rect.top >= bounds.top - 1 && rect.bottom <= bounds.bottom + 1;
    }),
    ariaMatchesGeometry,
  };
})()
""")
    devtools.call(
        "Emulation.setDeviceMetricsOverride",
        {"width": 1680, "height": 900, "deviceScaleFactor": 1, "mobile": False},
        session_id,
    )
    time.sleep(0.15)
    if not responsive["contained"] or not responsive["ariaMatchesGeometry"]:
        fail(f"fullscreen workspace splits did not adapt to viewport pressure: {responsive!r}")

    evaluate("document.querySelector('[data-mech-console-fullscreen]')?.click()")
    wait_for(
        "document.querySelector('[data-mech-console-fullscreen]')?.getAttribute('aria-pressed') === 'false' && "
        "document.querySelector('[data-mech-console-fullscreen]')?.getAttribute('aria-label') === 'Enter fullscreen workspace' && "
        "document.querySelectorAll('[data-mech-console-panel]:not([hidden])').length === 1",
        "the fullscreen control restoring its accessible inactive state",
    )


def assert_toc_survives_console_pressure():
    toc = evaluate_json("""
(async () => {
  const root = document.querySelector('[data-mech-repl-host]');
  const pane = document.querySelector('[data-mech-console-pane]');
  const content = document.querySelector('.content-column');
  const layout = document.querySelector('.article-layout, .docs-layout');
  const toc = layout?.querySelector('.toc, [data-mech-toc]');
  const toggle = layout?.querySelector(':scope > .mech-toc-toggle');
  const main = layout?.querySelector('.main-content');
  if (!root || !pane || !content || !layout || !toc || !toggle || !main) return null;
  const oldRootSize = root.style.getPropertyValue('--mech-console-size');
  const oldPaneWidth = pane.style.width;
  const initialTocRect = toc.getBoundingClientRect();
  const initialMainRect = main.getBoundingClientRect();
  const initialSideBySide = getComputedStyle(toc).display !== 'none' &&
    initialTocRect.right <= initialMainRect.left + 1 &&
    Math.abs(initialTocRect.top - initialMainRect.top) < 80;
  const pressuredSize = Math.max(
    370,
    Math.floor(root.getBoundingClientRect().width - 72 - 840),
  );
  root.style.setProperty('--mech-console-size', `${pressuredSize}px`);
  pane.style.width = `${pressuredSize}px`;
  await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  const contentRect = content.getBoundingClientRect();
  const collapsed = {
    toggleVisible: getComputedStyle(toggle).display !== 'none' &&
      toggle.getBoundingClientRect().width > 0,
    tocHidden: getComputedStyle(toc).display === 'none',
    mainVisible: getComputedStyle(main).display !== 'none',
    expanded: toggle.getAttribute('aria-expanded') === 'false',
  };
  toggle.click();
  await new Promise(resolve => requestAnimationFrame(resolve));
  const tocRect = toc.getBoundingClientRect();
  const open = {
    classified: layout.classList.contains('is-toc-open'),
    expanded: toggle.getAttribute('aria-expanded') === 'true',
    visible: getComputedStyle(toc).display !== 'none' && tocRect.width > 0 && tocRect.height > 0,
    contentReplaced: getComputedStyle(main).display === 'none',
    unbounded: getComputedStyle(toc).maxHeight === 'none' &&
      getComputedStyle(toc).overflowY === 'visible',
    allLevelsVisible: [...toc.querySelectorAll('.toc-sub')].every(list =>
      getComputedStyle(list).display !== 'none'),
  };
  const link = toc.querySelector('.toc-sub a[href^="#"]') || toc.querySelector('a[href^="#"]');
  const target = link && document.getElementById(link.getAttribute('href').slice(1));
  let activations = 0;
  const originalScrollIntoView = target?.scrollIntoView;
  if (target) target.scrollIntoView = () => { activations += 1; };
  link?.click();
  await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  if (target && originalScrollIntoView) target.scrollIntoView = originalScrollIntoView;
  const selected = {
    tocClosed: !layout.classList.contains('is-toc-open') &&
      toggle.getAttribute('aria-expanded') === 'false',
    contentRestored: getComputedStyle(main).display !== 'none',
    oneNavigation: activations === 1,
  };
  const result = {
    initialSideBySide,
    contentUsesCompactRange: contentRect.width > 680 && contentRect.width < 900,
    collapsed,
    open,
    selected,
  };
  if (oldRootSize) root.style.setProperty('--mech-console-size', oldRootSize);
  else root.style.removeProperty('--mech-console-size');
  pane.style.width = oldPaneWidth;
  return result;
})()
""")
    if (
        toc is None or
        not toc["initialSideBySide"] or
        not toc["contentUsesCompactRange"] or
        not all(toc["collapsed"].values()) or
        not all(toc["open"].values()) or
        not all(toc["selected"].values())
    ):
        fail(f"the compact table of contents failed under console width pressure: {toc!r}")


def assert_toc_scrollspy_is_continuous_and_hierarchical():
    state = evaluate_json("""
(async () => {
  const toc = document.querySelector('.toc, [data-mech-toc]');
  const list = toc?.querySelector(':scope > ul');
  const topItems = [...(list?.children || [])].filter(item => item.matches('li'));
  const topLinks = topItems.map(item => item.querySelector(':scope > a[href^="#"]')).filter(Boolean);
  const shell = document.querySelector('.content-shell');
  if (!toc || topLinks.length < 2 || !shell) return null;
  const contained = /auto|scroll|overlay/.test(getComputedStyle(shell).overflowY) &&
    shell.scrollHeight > shell.clientHeight + 1;
  const frame = () => new Promise(resolve => requestAnimationFrame(resolve));
  const settle = async () => { await frame(); await frame(); };
  const scrollTo = async (top) => {
    if (contained) shell.scrollTo({ top, behavior: 'instant' });
    else window.scrollTo({ top, behavior: 'instant' });
    await settle();
  };
  const maximum = contained
    ? Math.max(0, shell.scrollHeight - shell.clientHeight)
    : Math.max(0, document.documentElement.scrollHeight - innerHeight);
  const snapshot = () => ({
    activeTop: topLinks.findIndex(link => link.classList.contains('active')),
    activeTopCount: topLinks.filter(link => link.classList.contains('active')).length,
    currentCount: toc.querySelectorAll('a[aria-current="location"]').length,
  });

  await scrollTo(0);
  const first = snapshot();
  const samples = [first];
  for (const fraction of [0.2, 0.45, 0.7]) {
    await scrollTo(maximum * fraction);
    samples.push(snapshot());
  }
  await scrollTo(maximum);
  const last = snapshot();
  samples.push(last);

  const nestedItem = topItems.find(item => item.querySelector(':scope > .toc-sub a[href^="#"]'));
  let hierarchy = null;
  if (nestedItem) {
    const topLink = nestedItem.querySelector(':scope > a[href^="#"]');
    const nestedLink = nestedItem.querySelector(':scope > .toc-sub a[href^="#"]');
    const target = nestedLink && document.getElementById(nestedLink.getAttribute('href').slice(1));
    if (topLink && nestedLink && target) {
      const shellRect = shell.getBoundingClientRect();
      const activationOffset = Math.min(
        (contained ? shell.clientHeight : innerHeight) * 0.35,
        280
      );
      const offset = (contained ? shell.scrollTop : window.scrollY) +
        target.getBoundingClientRect().top - (contained ? shellRect.top : 0) -
        activationOffset + 24;
      await scrollTo(Math.max(2, Math.min(maximum - 10, offset)));
      const sub = nestedItem.querySelector(':scope > .toc-sub');
      const style = getComputedStyle(sub);
      hierarchy = {
        parentActive: topLink.classList.contains('active'),
        childActive: nestedLink.classList.contains('active'),
        expanded: nestedItem.classList.contains('expanded') && style.display !== 'none',
        indented: parseFloat(style.marginLeft) > 0 && parseFloat(style.paddingLeft) > 0,
        delineated: parseFloat(style.borderLeftWidth) > 0 && style.borderLeftStyle === 'dotted',
        targetTop: target.getBoundingClientRect().top,
        activationLine: (contained ? shell.getBoundingClientRect().top : 0) + activationOffset,
        scrollTop: contained ? shell.scrollTop : window.scrollY,
      };
    }
  }
  await scrollTo(0);
  return {
    first,
    last,
    samples,
    hierarchy,
    maximum,
    topCount: topLinks.length,
  };
})()
""")
    if state is None or state["maximum"] <= 0:
        fail(f"TOC scrollspy fixture was not scrollable: {state!r}")
    if state["first"]["activeTop"] != 0 or state["last"]["activeTop"] != state["topCount"] - 1:
        fail(f"TOC did not own the first and final scroll positions: {state!r}")
    if any(sample["activeTopCount"] != 1 or sample["currentCount"] != 1 for sample in state["samples"]):
        fail(f"TOC scrollspy left a scroll position without one current section: {state!r}")
    active_indices = [sample["activeTop"] for sample in state["samples"]]
    if active_indices != sorted(active_indices):
        fail(f"TOC section progression moved backwards while scrolling down: {state!r}")
    if state["hierarchy"] is None or not all(
        state["hierarchy"][key]
        for key in ("parentActive", "childActive", "expanded", "indented", "delineated")
    ):
        fail(f"TOC nested section styling or active path regressed: {state!r}")


def assert_empty_toc_is_removed_and_content_is_centered():
    state = evaluate_json("""
(() => {
  const layout = document.querySelector('.article-layout, .docs-layout');
  const toc = layout?.querySelector('.toc, [data-mech-toc]');
  const main = layout?.querySelector('.main-content');
  if (!layout || !toc || !main) return null;
  const original = toc.innerHTML;
  toc.innerHTML = '<div class="toc-title">Contents</div><ul></ul>';
  window.dispatchEvent(new CustomEvent('mech:document-layout-refresh'));
  const layoutRect = layout.getBoundingClientRect();
  const mainRect = main.getBoundingClientRect();
  const result = {
    classified: layout.classList.contains('has-empty-toc'),
    tocHidden: toc.hidden && getComputedStyle(toc).display === 'none',
    contentCentered: Math.abs(
      (mainRect.left + mainRect.right) / 2 - (layoutRect.left + layoutRect.right) / 2
    ) <= 1,
  };
  toc.innerHTML = original;
  toc.hidden = false;
  window.dispatchEvent(new CustomEvent('mech:document-layout-refresh'));
  window.dispatchEvent(new CustomEvent('mech:document-layout-refresh'));
  window.dispatchEvent(new CustomEvent('mech:document-layout-refresh'));
  const link = toc.querySelector('a[href^="#"]');
  const target = link && document.getElementById(link.getAttribute('href').slice(1));
  let activations = 0;
  const originalScrollIntoView = target?.scrollIntoView;
  if (target) {
    target.scrollIntoView = () => { activations += 1; };
  }
  link?.click();
  if (target && originalScrollIntoView) {
    target.scrollIntoView = originalScrollIntoView;
  }
  result.singleActivationAfterRefresh = activations === 1;
  return result;
})()
""")
    if state is None or not all(state.values()):
        fail(f"empty table of contents remained in the document layout: {state!r}")


def submit(command):
    command_json = json.dumps(command)
    submitted = evaluate(f"""
(() => {{
  const input = document.querySelector(".repl-input");
  if (!input) return false;
  input.focus();
  input.value = {command_json};
  input.dispatchEvent(new KeyboardEvent("keydown", {{
    key: "Enter", bubbles: true, cancelable: true,
  }}));
  return true;
}})()
""")
    if not submitted:
        fail(f"could not submit browser REPL command: {command}")


def assert_controller_cooperative_lifecycle():
    result = evaluate_json("""
(async () => {
  const controller = globalThis.MechDocumentController;
  const terminal = response => Boolean(response) &&
    response.pending !== true &&
    response.hostPending !== true &&
    !response.hostRequest;
  const immediate = await controller.invoke('controller-probe := answer + 1');
  const afterImmediate = await controller.invoke(':whos controller-probe');
  const stepped = await controller.invoke(':step 256');
  const afterStep = await controller.invoke(':whos controller-probe');
  const documented = await controller.invoke(':docs');
  const afterDocs = await controller.invoke(':whos controller-probe');
  const commandSourceCounts = Object.fromEntries([
    ':step 256',
    ':docs',
  ].map(source => [source, [...document.querySelectorAll('.mech-repl-source .repl-code')]
    .filter(code => code.textContent.trim() === source).length]));
  const value = controller.renderedValue('controller-probe')?.inlineHtml || '';
  const cleaned = await controller.invoke(':clear controller-probe');
  return {
    terminal: [immediate, afterImmediate, stepped, afterStep, documented, afterDocs, cleaned]
      .every(terminal),
    commandSourceCounts,
    ready:
      document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'ready' &&
      document.querySelector('.repl-input')?.readOnly === false,
    value,
  };
})()
""")
    if result != {
        "terminal": True,
        "commandSourceCounts": {
            ":step 256": 1,
            ":docs": 1,
        },
        "ready": True,
        "value": "42",
    }:
        fail(f"public controller did not share the terminal cooperative lifecycle: {result!r}")


def assert_console_contract():
    resident = evaluate_json("""
(() => {
  const input = document.querySelector('.repl-input');
  if (!input) return null;
  return {
    capability: input.dataset.mechInteractiveEvaluation,
    label: input.getAttribute('aria-label'),
    placeholder: input.getAttribute('placeholder'),
    initialHint: document.querySelector('.mech-repl-transcript > .mech-repl-hint')?.textContent,
  };
})()
""")
    if resident != {
        "capability": "resident",
        "label": "Mech resident REPL input",
        "placeholder": None,
        "initialHint": "Enter submits · Ctrl+Enter adds a line · :help prints help",
    }:
        fail(f"the standard document console did not advertise resident evaluation: {resident!r}")

    evaluate("document.querySelector('[data-mech-console-tab=output]')?.click()")
    wait_for(
        "!document.querySelector('[data-mech-console-panel=output]')?.hidden && "
        "Boolean(document.querySelector('.mech-document-output-entry'))",
        "the document output metadata panel",
    )
    metadata = evaluate_json("""
(() => {
  const panel = document.querySelector('[data-mech-output-panel]');
  const entry = panel?.querySelector('.mech-document-output-entry');
  const name = entry?.querySelector('.mech-document-output-name');
  const kind = entry?.querySelector('.mech-output-kind');
  const body = entry?.querySelector('.mech-document-output-html');
  if (!panel || !entry || !kind || !body) return null;
  const probe = document.createElement('span');
  probe.style.color = 'var(--kind-annotation-color, #f09fca)';
  entry.append(probe);
  const expectedKindColor = getComputedStyle(probe).color;
  probe.remove();
  const panelRect = panel.getBoundingClientRect();
  const entryRect = entry.getBoundingClientRect();
  return {
    entryCount: panel.querySelectorAll('.mech-document-output-entry').length,
    nameAbsent: name === null,
    selectionToken: entry.dataset.mechSelectionToken,
    bodyText: body.textContent.trim(),
    kind: kind.textContent.trim(),
    kindColor: getComputedStyle(kind).color,
    expectedKindColor,
    contained:
      entryRect.left >= panelRect.left - 1 &&
      entryRect.right <= panelRect.right + 1 &&
      document.documentElement.scrollWidth <= window.innerWidth + 1,
    bodyOverflow: getComputedStyle(body).overflowX,
  };
})()
""")
    if (
        metadata is None or
        metadata["entryCount"] != 1 or
        not metadata["nameAbsent"] or
        not metadata["selectionToken"] or
        metadata["kind"] != "f64" or
        metadata["kindColor"] != metadata["expectedKindColor"] or
        not metadata["contained"] or
        metadata["bodyOverflow"] in {"auto", "scroll"}
    ):
        fail(f"document output metadata was not compact, typed, rose, contained, and scrollbar-free: {metadata!r}")
    evaluate("document.querySelector('[data-mech-console-tab=console]')?.click()")

    multiline = evaluate_json("""
(() => {
  const input = document.querySelector('.repl-input');
  const transcript = document.querySelector('.mech-repl-transcript');
  if (!input || !transcript) return null;
  const rowsBefore = transcript.children.length;
  input.value = ':whos answer';
  input.setSelectionRange(5, 5);
  input.dispatchEvent(new KeyboardEvent('keydown', {
    key: 'Enter', ctrlKey: true, bubbles: true, cancelable: true,
  }));
  const result = {
    value: input.value,
    caret: input.selectionStart,
    transcriptUnchanged: transcript.children.length === rowsBefore,
  };
  input.value = 'one\\ntwo\\nthree\\nfour';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  const style = getComputedStyle(input);
  result.grewForFourLines = input.getBoundingClientRect().height >= parseFloat(style.lineHeight) * 3.5;
  result.manualResizeHidden = style.resize === 'none';
  result.inputNeverScrolls = style.overflowY === 'hidden' &&
    input.scrollHeight <= input.clientHeight + 1;
  const spacer = document.createElement('div');
  spacer.style.cssText = `height:${Math.max(600, transcript.clientHeight * 2)}px;flex:none`;
  transcript.insertBefore(spacer, input.closest('.mech-repl-input-row'));
  transcript.scrollTop = transcript.scrollHeight;
  const bottom = transcript.scrollTop;
  const pageUp = new KeyboardEvent('keydown', {
    key: 'PageUp', bubbles: true, cancelable: true,
  });
  input.dispatchEvent(pageUp);
  result.pageUpScrollsTranscript = pageUp.defaultPrevented && transcript.scrollTop < bottom;
  const home = new KeyboardEvent('keydown', {
    key: 'Home', bubbles: true, cancelable: true,
  });
  input.dispatchEvent(home);
  result.homeScrollsTranscript = home.defaultPrevented && transcript.scrollTop === 0;
  const end = new KeyboardEvent('keydown', {
    key: 'End', bubbles: true, cancelable: true,
  });
  input.dispatchEvent(end);
  result.endScrollsTranscript = end.defaultPrevented && transcript.scrollTop > 0;
  transcript.scrollTop = 0;
  const pageDown = new KeyboardEvent('keydown', {
    key: 'PageDown', bubbles: true, cancelable: true,
  });
  input.dispatchEvent(pageDown);
  result.pageDownScrollsTranscript = pageDown.defaultPrevented && transcript.scrollTop > 0;
  spacer.remove();
  input.value = '';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  return result;
})()
""")
    if multiline != {
        "value": ":whos\n answer",
        "caret": 6,
        "transcriptUnchanged": True,
        "grewForFourLines": True,
        "manualResizeHidden": True,
        "inputNeverScrolls": True,
        "pageUpScrollsTranscript": True,
        "homeScrollsTranscript": True,
        "endScrollsTranscript": True,
        "pageDownScrollsTranscript": True,
    }:
        fail(f"Ctrl+Enter did not insert a multiline browser REPL draft: {multiline!r}")

    submit("answer + 1")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-result')].some((row) => "
        "/42/.test(row.textContent)) && "
        "document.querySelector('.mech-repl-transcript')?.lastElementChild?.classList.contains('mech-repl-active-prompt')",
        "the document-backed resident console and descending active prompt",
    )
    fixed_output = evaluate_json("""
(() => {
  const entry = document.querySelector('[data-mech-output-panel] .mech-document-output-entry');
  const body = entry?.querySelector('.mech-document-output-html');
  return entry && body ? {
    selectionToken: entry.dataset.mechSelectionToken,
    bodyText: body.textContent.trim(),
    nameAbsent: !entry.querySelector('.mech-document-output-name'),
  } : null;
})()
""")
    if fixed_output != {
        "selectionToken": metadata["selectionToken"],
        "bodyText": metadata["bodyText"],
        "nameAbsent": True,
    }:
        fail(
            "REPL evaluation replaced or renamed the fixed document Output pane: "
            f"before={metadata!r}, after={fixed_output!r}"
        )
    directed_lifecycle = evaluate_json("""
(() => {
  const publish = detail => window.dispatchEvent(new CustomEvent('mech:output', { detail }));
  const original = document.querySelector('[data-mech-output-panel] .mech-document-output-entry');
  const originalToken = original?.dataset.mechSelectionToken;
  const originalText = original?.querySelector('.mech-document-output-html')?.textContent.trim();
  publish({
    source: { name: 'document-directed', span: null },
    stream: 'stdout', operation: 'create', display_id: 'directed-smoke',
    content: { kind: 'text', data: { text: 'directed output' } },
  });
  const documentRegion = document.querySelector('[data-mech-output-region="document"]');
  const replRegion = document.querySelector('[data-mech-output-region="repl"]');
  const directed = {
    implicitHidden: !documentRegion?.querySelector('.mech-document-output-entry'),
    directedVisible: /directed output/.test(replRegion?.textContent || ''),
  };
  publish({
    stream: 'stdout', operation: 'clear', display_id: null,
    content: { kind: 'text', data: { text: '' } },
  });
  const restored = documentRegion?.querySelector('.mech-document-output-entry');
  return {
    ...directed,
    directedCleared: !/directed output/.test(replRegion?.textContent || ''),
    implicitRestored: restored?.dataset.mechSelectionToken === originalToken &&
      restored?.querySelector('.mech-document-output-html')?.textContent.trim() === originalText,
  };
})()
""")
    if directed_lifecycle is None or not all(directed_lifecycle.values()):
        fail(f"directed output did not yield back to implicit program output: {directed_lifecycle!r}")
    presentation = evaluate_json("""
(() => {
  const result = [...document.querySelectorAll('.mech-repl-result')]
    .find(row => /42/.test(row.textContent || ''));
  const kind = result?.querySelector('.mech-repl-result-kind');
  const value = result?.querySelector('.mech-repl-result-value');
  const number = value?.querySelector('.mech-number');
  const input = document.querySelector('.repl-input');
  if (!result || !kind || !value || !number || !input) return null;
  input.blur();
  result.click();
  const kindRect = kind.getBoundingClientRect();
  const valueRect = value.getBoundingClientRect();
  return {
    kind: kind.textContent.trim(),
    kindColored: getComputedStyle(kind).color !== getComputedStyle(value).color,
    kindAboveValue: kindRect.bottom <= valueRect.top,
    valueTokenized: number.textContent.trim() === '42',
    promptFocusedByResultClick: document.activeElement === input,
  };
})()
""")
    if (
        presentation is None or
        not presentation["kind"] or
        not presentation["kindColored"] or
        not presentation["kindAboveValue"] or
        not presentation["valueTokenized"] or
        not presentation["promptFocusedByResultClick"]
    ):
        fail(f"REPL values lost their kind, token formatting, or click-to-focus behavior: {presentation!r}")

    history = evaluate_json("""
(() => {
  const input = document.querySelector('.repl-input');
  if (!input) return null;
  input.value = 'draft first\\ndraft second\\ndraft third';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  input.setSelectionRange(input.value.length, input.value.length);
  const inside = new KeyboardEvent('keydown', {
    key: 'ArrowUp', bubbles: true, cancelable: true,
  });
  input.dispatchEvent(inside);
  const insideValue = input.value;
  input.setSelectionRange(3, 3);
  const recall = new KeyboardEvent('keydown', {
    key: 'ArrowUp', bubbles: true, cancelable: true,
  });
  input.dispatchEvent(recall);
  const recalledValue = input.value;
  const recalledAtTop = input.selectionStart === 0;
  const restore = new KeyboardEvent('keydown', {
    key: 'ArrowDown', bubbles: true, cancelable: true,
  });
  input.dispatchEvent(restore);
  const restoredValue = input.value;
  input.value = '';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  return {
    insideWasEditorNavigation: !inside.defaultPrevented && insideValue === 'draft first\\ndraft second\\ndraft third',
    recalledAtTop: recall.defaultPrevented && recalledAtTop && /answer \\+ 1/.test(recalledValue),
    restoredDraft: restore.defaultPrevented && restoredValue === 'draft first\\ndraft second\\ndraft third',
  };
})()
""")
    if history is None or not all(history.values()):
        fail(f"multiline editing and history navigation fought over arrow keys: {history!r}")

    background = evaluate_json("""
(() => {
  const input = document.querySelector('.repl-input');
  const consoleTab = document.querySelector('[data-mech-console-tab="console"]');
  const outputTab = document.querySelector('[data-mech-console-tab="output"]');
  if (!input || !consoleTab || !outputTab) return null;
  consoleTab.click();
  input.value = 'draft survives background output';
  input.focus();
  for (let update = 0; update < 5; update += 1) {
    window.dispatchEvent(new CustomEvent('mech:output', { detail: {
      operation: update === 0 ? 'replace' : 'update',
      stream: 'stdout',
      display_id: 'background-focus-smoke',
      content: { kind: 'text', data: { text: `frame ${update}` } },
    }}));
  }
  const result = {
    consoleSelected: consoleTab.getAttribute('aria-selected') === 'true',
    inputFocused: document.activeElement === input,
    draftPreserved: input.value === 'draft survives background output',
    outputUpdated: /frame 4/.test(
      document.querySelector('[data-mech-display-id="background-focus-smoke"]')?.textContent || ''
    ),
    unreadMarked: outputTab.dataset.mechConsoleUnread === 'true',
  };
  outputTab.click();
  result.unreadClearedOnExplicitFocus = !outputTab.hasAttribute('data-mech-console-unread');
  consoleTab.click();
  input.value = '';
  input.dispatchEvent(new Event('input', { bubbles: true }));
  window.dispatchEvent(new CustomEvent('mech:output', { detail: {
    operation: 'remove', stream: 'stdout', display_id: 'background-focus-smoke',
  }}));
  return result;
})()
""")
    if background is None or not all(background.values()):
        fail(f"background program output stole console focus or lost its unread state: {background!r}")
    if label == "configured":
        evaluate("""
(() => {
  const root = document.querySelector('.mech-root');
  if (!root || document.getElementById('mech-smoke-clock')) return false;
  const marker = document.createElement('span');
  marker.id = 'mech-smoke-clock';
  const value = document.createElement('span');
  value.className = 'mech-var-placeholder';
  value.dataset.mechVarName = 'clock-second';
  marker.append(value);
  root.append(marker);
  return true;
})()
""")
        submit("clock-second := @clock/second;")
        wait_for(
            "Boolean(document.querySelector('#mech-smoke-clock .mech-var-placeholder')?.textContent.trim())",
            "the configured clock variable after REPL replacement",
        )
        clock_before = evaluate_json(
            "document.querySelector('#mech-smoke-clock .mech-var-placeholder').textContent.trim()"
        )
        if clock_before is None:
            fail("the configured clock variable was not rendered before REPL replacement")
        wait_for(
            "document.querySelector('#mech-smoke-clock .mech-var-placeholder')?.textContent.trim() !== "
            + json.dumps(clock_before),
            "the live time input driver restarting after document REPL replacement",
            timeout=5,
        )
        evaluate("document.querySelector('#mech-smoke-clock')?.remove()")
        submit(":clear clock-second")
        wait_for(
            "[...document.querySelectorAll('.mech-repl-source .repl-code')].some((row) => row.textContent.trim() === ':clear clock-second') && "
            "document.querySelector('.repl-input')?.readOnly === false",
            "the configured driver probe returning to the baseline document",
        )
    result_count = evaluate_json(
        "document.querySelectorAll('.mech-repl-result').length"
    )
    submit("answer + 2; -- suppress this browser value")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-source .repl-code')].some((row) => "
        "/^answer\\s*\\+\\s*2\\s*;\\s*--\\s*suppress this browser value$/.test(row.textContent.trim()))",
        "the semicolon-terminated browser source echo",
    )
    suppressed_count = evaluate_json(
        "document.querySelectorAll('.mech-repl-result').length"
    )
    if suppressed_count != result_count:
        fail(
            "semicolon-terminated browser input rendered an automatic value: "
            f"before={result_count!r}, after={suppressed_count!r}"
        )
    evaluate("""
(() => {
  const panel = document.querySelector('[data-mech-errors-panel]');
  if (!panel) return false;
  let region = panel.querySelector('[data-mech-error-region=document]');
  if (!region) {
    region = document.createElement('div');
    region.dataset.mechErrorRegion = 'document';
    panel.append(region);
  }
  const marker = document.createElement('div');
  marker.dataset.mechDocumentErrorSmoke = 'true';
  marker.textContent = 'persistent document failure';
  region.append(marker);
  return true;
})()
""")
    submit(":step 1000001")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-transcript .mech-repl-diagnostic')].some((row) => "
        "/step count must be between 1 and 1000000/.test(row.textContent)) && "
        "!document.querySelector('[data-mech-error-region=repl]') && "
        "document.querySelector('[data-mech-console-tab=console]')?.getAttribute('aria-selected') === 'true'",
        "the portable resident step ceiling inline in the console",
    )
    prompt_ownership = evaluate_json("""
(() => {
  const sourceRows = [...document.querySelectorAll('.mech-repl-source')]
    .filter(row => row.querySelector('.repl-code')?.textContent.trim() === ':step 1000001');
  return {
    matchingSourceRows: sourceRows.length,
    sourcePromptCounts: sourceRows.map(row => row.querySelectorAll(':scope > .repl-prompt').length),
    activePrompts: document.querySelectorAll('.mech-repl-active-prompt').length,
  };
})()
""")
    if prompt_ownership != {
        "matchingSourceRows": 1,
        "sourcePromptCounts": [1],
        "activePrompts": 1,
    }:
        fail(f"browser submission duplicated a source or active prompt: {prompt_ownership!r}")
    submit(":clc")
    wait_for(
        "Boolean(document.querySelector('[data-mech-document-error-smoke=true]')) && "
        "document.querySelectorAll('.mech-repl-transcript .mech-repl-diagnostic').length === 0 && "
        "!document.querySelector('[data-mech-error-region=repl]')",
        "scoped inline resident diagnostic clearing",
    )
    evaluate("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  const original = WasmDocument.prototype.replInvoke;
  WasmDocument.prototype.replInvoke = function(source) {
    window.__MECH_SMOKE_DOCUMENT__ = this;
    const response = original.call(this, source);
    if (source === ':capabilities') {
      response.events = (response.events || []).filter(envelope =>
        envelope.event?.channel !== 'repl' || envelope.event?.event?.kind !== 'source_echo'
      );
      WasmDocument.prototype.replInvoke = original;
    }
    return response;
  };
})()
""")
    submit(":capabilities")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-source .repl-code')].some((row) => "
        "row.textContent.trim() === ':capabilities') && "
        "[...document.querySelectorAll('.mech-repl-response')].some((row) => "
        "/console:\\/\\//i.test(row.textContent) && /granted/i.test(row.textContent))",
        "a locally committed command whose portable source echo was omitted",
    )
    submit(":whos answer")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-symbols')].some((table) => /answer/.test(table.textContent)) && "
        "[...document.querySelectorAll('.mech-repl-symbols')].some((table) => /41/.test(table.textContent)) && "
        "[...document.querySelectorAll('.mech-repl-source .repl-code')].some((row) => row.textContent.trim() === ':whos answer')",
        "the running document answer row from :whos",
    )
    omitted_echo_ownership = evaluate_json("""
(() => {
  const commands = [':capabilities', ':whos answer'];
  const counts = Object.fromEntries(commands.map(command => [
    command,
    [...document.querySelectorAll('.mech-repl-source .repl-code')]
      .filter(code => code.textContent.trim() === command).length,
  ]));
  return {
    counts,
    activePrompts: document.querySelectorAll('.mech-repl-active-prompt').length,
  };
})()
""")
    if omitted_echo_ownership != {
        "counts": {":capabilities": 1, ":whos answer": 1},
        "activePrompts": 1,
    }:
        fail(
            "an omitted source echo leaked submission ownership into the next prompt: "
            f"{omitted_echo_ownership!r}"
        )
    repeated_enter_ownership = evaluate_json("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  const original = WasmDocument.prototype.replInvoke;
  const input = document.querySelector('.repl-input');
  if (!input) return null;
  const command = ':plan';
  const sourceCount = () => [...document.querySelectorAll('.mech-repl-source .repl-code')]
    .filter(code => code.textContent.trim() === command).length;
  const sourcesBefore = sourceCount();
  const responsesBefore = document.querySelectorAll('.mech-repl-response').length;
  let invocations = 0;
  WasmDocument.prototype.replInvoke = function(source) {
    if (source === command) invocations += 1;
    return original.call(this, source);
  };
  input.value = command;
  const enter = () => input.dispatchEvent(new KeyboardEvent('keydown', {
    key: 'Enter', bubbles: true, cancelable: true,
  }));
  enter();
  enter();
  WasmDocument.prototype.replInvoke = original;
  const matchingRows = [...document.querySelectorAll('.mech-repl-source')]
    .filter(row => row.querySelector('.repl-code')?.textContent.trim() === command);
  return {
    invocations,
    sourceDelta: sourceCount() - sourcesBefore,
    responseDelta:
      document.querySelectorAll('.mech-repl-response').length - responsesBefore,
    sourcePromptCounts:
      matchingRows.map(row => row.querySelectorAll(':scope > .repl-prompt').length),
    activePrompts: document.querySelectorAll('.mech-repl-active-prompt').length,
  };
})()
""")
    if repeated_enter_ownership != {
        "invocations": 1,
        "sourceDelta": 1,
        "responseDelta": 1,
        "sourcePromptCounts": [1],
        "activePrompts": 1,
    }:
        fail(
            "repeated Enter on a retired input duplicated command ownership: "
            f"{repeated_enter_ownership!r}"
        )
    evaluate("""
(() => {
  const root = document.querySelector('.mech-root');
  if (!root || document.getElementById('mech-smoke-large-var')) return false;
  const marker = document.createElement('span');
  marker.id = 'mech-smoke-large-var';
  const value = document.createElement('span');
  value.className = 'mech-var-placeholder';
  value.dataset.mechVarName = 'qq';
  marker.append(value);
  const surface = root.querySelector('.mech-document-content, #left-pane, .content-shell, .main-content') || root;
  surface.append(marker);
  return true;
})()
""")
    submit("qq := 1..1000;")
    wait_for(
        "Boolean(document.querySelector('#mech-smoke-large-var .mech-var-placeholder')?.textContent.trim())",
        "the large resident variable placeholder",
    )
    click_performance = evaluate_json("""
(() => {
  const value = document.querySelector('#mech-smoke-large-var .mech-var-placeholder');
  if (!value) return null;
  const rendersBefore = Number(window.__MECH_DOCUMENT_RENDERS__ || 0);
  const started = performance.now();
  value.click();
  return {
    elapsedMs: performance.now() - started,
    rerenderedDocument:
      Number(window.__MECH_DOCUMENT_RENDERS__ || 0) !== rendersBefore,
    printed:
      [...document.querySelectorAll('.mech-repl-result')]
        .some(row => /999/.test(row.textContent)),
  };
})()
""")
    if (
        click_performance is None or
        click_performance["elapsedMs"] >= 200 or
        click_performance["rerenderedDocument"] or
        not click_performance["printed"]
    ):
        fail(
            "clicking a large resident value recompiled, rerendered, or exceeded the 200ms UI budget: "
            f"{click_performance!r}"
        )
    popup_performance = evaluate_json("""
(() => {
  const root = document.querySelector('.mech-root');
  const value = document.querySelector('#mech-smoke-large-var .mech-var-placeholder');
  const pane = document.querySelector('[data-mech-console-pane]');
  const transcript = document.querySelector('.mech-repl-transcript');
  if (!root || !value || !pane || !transcript) return null;
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  value.scrollIntoView({ block: 'center' });
  const transcriptEntries = transcript.children.length;
  const rendersBefore = Number(window.__MECH_DOCUMENT_RENDERS__ || 0);
  const started = performance.now();
  value.click();
  const elapsedMs = performance.now() - started;
  const popup = document.querySelector('.mech-inline-popup[data-mech-repl-popup]');
  const popupRect = popup?.getBoundingClientRect();
  const valueRect = value.getBoundingClientRect();
  const header = popup?.querySelector('.mech-inline-popup__header');
  if (header && popupRect) {
    const pointerId = 91;
    header.dispatchEvent(new PointerEvent('pointerdown', {
      bubbles: true, cancelable: true, pointerId, button: 0,
      clientX: popupRect.left + 8, clientY: popupRect.top + 8,
    }));
    window.dispatchEvent(new PointerEvent('pointermove', {
      bubbles: true, pointerId,
      clientX: popupRect.left + 48, clientY: popupRect.top + 38,
    }));
    window.dispatchEvent(new PointerEvent('pointerup', {
      bubbles: true, pointerId,
      clientX: popupRect.left + 48, clientY: popupRect.top + 38,
    }));
  }
  const movedRect = popup?.getBoundingClientRect();
  const closeButton = popup?.querySelector('.mech-inline-popup__close');
  const result = {
    elapsedMs,
    rerenderedDocument:
      Number(window.__MECH_DOCUMENT_RENDERS__ || 0) !== rendersBefore,
    rendered: /999/.test(popup?.textContent || ''),
    closedThroughControl:
      root.dataset.mechConsoleOpen === 'false' && pane.hidden,
    transcriptClean: transcript.children.length === transcriptEntries,
    positionedByValue:
      Boolean(popupRect) && Math.abs(popupRect.top - valueRect.top) < 80,
    draggable:
      Boolean(popupRect && movedRect) &&
      (Math.abs(movedRect.left - popupRect.left) > 10 ||
        Math.abs(movedRect.top - popupRect.top) > 10),
    focusMoved: document.activeElement === closeButton,
  };
  if (popup) {
    popup.style.left = '100000px';
    popup.style.top = '100000px';
    window.dispatchEvent(new Event('resize'));
    const clampedRect = popup.getBoundingClientRect();
    result.reclamped =
      clampedRect.left >= 0 && clampedRect.top >= 0 &&
      clampedRect.right <= window.innerWidth &&
      clampedRect.bottom <= window.innerHeight;
  }
  closeButton?.click();
  result.dismissed = !document.querySelector('.mech-inline-popup[data-mech-repl-popup]');
  result.focusRestored = document.activeElement === value;

  value.click();
  result.reopenPopupCreated = Boolean(
    document.querySelector('.mech-inline-popup[data-mech-repl-popup]')
  );
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  result.reopened =
    root.dataset.mechConsoleOpen === 'true' && !pane.hidden &&
    !document.querySelector('.mech-inline-popup[data-mech-repl-popup]');
  return result;
})()
""")
    if (
        popup_performance is None or
        popup_performance["elapsedMs"] >= 200 or
        popup_performance["rerenderedDocument"] or
        not popup_performance["rendered"] or
        not popup_performance["closedThroughControl"] or
        not popup_performance["transcriptClean"] or
        not popup_performance["positionedByValue"] or
        not popup_performance["draggable"] or
        not popup_performance["focusMoved"] or
        not popup_performance["reclamped"] or
        not popup_performance["dismissed"] or
        not popup_performance["focusRestored"] or
        not popup_performance["reopenPopupCreated"] or
        not popup_performance["reopened"]
    ):
        fail(
            "closed-console selection did not open a clean, anchored, draggable value popup: "
            f"{popup_performance!r}"
        )
    popup_identity = evaluate_json("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  const root = document.querySelector('.mech-root');
  const pane = document.querySelector('[data-mech-console-pane]');
  const outputs = [...document.querySelectorAll(
    '.mech-inline-mech-code[id], .mech-block-output[id], ' +
    '.mech-document-output-entry[data-mech-output-id]'
  )];
  if (!root || !pane || outputs.length < 2) return null;
  const original = WasmDocument.prototype.replSelectOutput;
  WasmDocument.prototype.replSelectOutput = function(outputId) {
    return {
      identity: `resident-output:${outputId}`,
      response: { events: [] },
      rendered: {
        name: 'same-name', kind: 'f64', inlineHtml: String(outputId),
        blockHtml: `<span>${outputId}</span>`,
      },
    };
  };
  const [first, second] = outputs;
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  first.click();
  first.click();
  second.click();
  first.click();
  const popups = [...document.querySelectorAll('[data-mech-repl-popup]')];
  const identities = popups.map(popup => popup.dataset.mechReplPopupIdentity);
  const titles = popups.map(popup =>
    popup.querySelector('.mech-inline-popup__title')?.textContent,
  );
  const result = {
    consoleClosed: root.dataset.mechConsoleOpen === 'false' && pane.hidden,
    onePerIdentity: popups.length === 2 && new Set(identities).size === 2,
    nameDidNotCollapseIdentity: titles.every(title => title === 'same-name'),
  };
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: 'Escape', bubbles: true, cancelable: true,
  }));
  result.escapeClosedOnlyTopmost =
    document.querySelectorAll('[data-mech-repl-popup]').length === 1;
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  result.openingConsoleDismissedAll =
    root.dataset.mechConsoleOpen === 'true' && !pane.hidden &&
    !document.querySelector('[data-mech-repl-popup]');
  WasmDocument.prototype.replSelectOutput = original;
  return result;
})()
""")
    if popup_identity is None or not all(popup_identity.values()):
        fail(
            "closed-console popups were not keyed one-per-runtime identity: "
            f"{popup_identity!r}"
        )
    evaluate("document.dispatchEvent(new KeyboardEvent('keydown', {key: '`', bubbles: true, cancelable: true}))")
    evaluate("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  window.__MECH_ORIGINAL_SELECT_SYMBOL__ = WasmDocument.prototype.replSelectSymbol;
  WasmDocument.prototype.replSelectSymbol = function() {
    return {
      response: { events: [{ event: {
        channel: 'repl',
        event: { kind: 'source_echo', payload: { source: 'qq' } },
      } }] },
      rendered: null,
    };
  };
})()
""")
    quiet_selection = evaluate_json("""
(() => {
  const value = document.querySelector('#mech-smoke-large-var .mech-var-placeholder');
  const transcript = document.querySelector('.mech-repl-transcript');
  if (!value || !transcript) return null;
  const before = transcript.children.length;
  value.click();
  return {
    transcriptClean: transcript.children.length === before,
    popupAbsent: !document.querySelector('.mech-inline-popup[data-mech-repl-popup]'),
  };
})()
""")
    evaluate("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  WasmDocument.prototype.replSelectSymbol = function() {
    throw new Error('synthetic closed selection failure');
  };
})()
""")
    closed_failure = evaluate_json("""
(() => {
  const value = document.querySelector('#mech-smoke-large-var .mech-var-placeholder');
  value?.click();
  const popup = document.querySelector('.mech-inline-popup[data-mech-repl-popup]');
  return {
    visible: /synthetic closed selection failure/.test(popup?.textContent || ''),
    focused: document.activeElement === popup?.querySelector('.mech-inline-popup__close'),
  };
})()
""")
    evaluate("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  if (window.__MECH_ORIGINAL_SELECT_SYMBOL__) {
    WasmDocument.prototype.replSelectSymbol = window.__MECH_ORIGINAL_SELECT_SYMBOL__;
    delete window.__MECH_ORIGINAL_SELECT_SYMBOL__;
  }
  document.querySelector('.mech-inline-popup__close')?.click();
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
})()
""")
    if (
        quiet_selection is None or
        not quiet_selection["transcriptClean"] or
        not quiet_selection["popupAbsent"] or
        closed_failure is None or
        not closed_failure["visible"] or
        not closed_failure["focused"]
    ):
        fail(
            "closed-console quiet or failure selection lifecycle regressed: "
            f"quiet={quiet_selection!r}, failure={closed_failure!r}"
        )
    submit(":whos ans")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-symbols tbody tr')].some((row) => "
        "/ans/.test(row.firstElementChild?.textContent || '') && /999|…/.test(row.lastElementChild?.textContent || ''))",
        "explicit pending ans inspection before source materialization",
    )
    historical_ans = evaluate_json("""
(() => {
  const rows = [...document.querySelectorAll('.mech-repl-symbols tbody tr')];
  const row = rows.findLast(candidate =>
    candidate.firstElementChild?.textContent.trim() === 'ans');
  const answer = document.querySelector('#mech-smoke-var .mech-var-placeholder');
  if (!row || !answer) return null;
  row.dataset.mechSmokeHistoricalAns = 'true';
  const token = row.dataset.mechSelectionToken;
  answer.click();
  const before = document.querySelectorAll('.mech-repl-result').length;
  row.firstElementChild.click();
  return {
    token: Boolean(token),
    boundToToken: row.firstElementChild.dataset.mechReplBound === 'true',
    resultCountBefore: before,
  };
})()
""")
    if historical_ans is None or not historical_ans["token"] or not historical_ans["boundToToken"]:
        fail(f"historical :whos ans row did not retain a selection token: {historical_ans!r}")
    wait_for(
        f"document.querySelectorAll('.mech-repl-result').length > {historical_ans['resultCountBefore']}",
        "the historical ans row restoring its captured value after ans changed",
    )
    submit("ans[100]")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-result')].at(-1)?.textContent.includes('100')",
        "the historical :whos row selecting its captured array rather than current ans",
    )
    submit(":whos qq")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-symbols tbody tr')].some((row) => "
        "/qq/.test(row.textContent) && /…\\]/.test(row.lastElementChild?.textContent || ''))",
        "an inline, elided :whos value",
    )
    whos_layout = evaluate_json("""
(() => {
  const rows = [...document.querySelectorAll('.mech-repl-symbols tbody tr')];
  const row = rows.find(candidate => /qq/.test(candidate.firstElementChild?.textContent || ''));
  const table = row?.closest('table');
  const value = row?.lastElementChild;
  const transcript = table?.closest('.mech-repl-transcript');
  if (!row || !table || !value || !transcript) return null;
  const cells = [...table.querySelectorAll('th, td')];
  return {
    value: value.textContent,
    inline: !value.textContent.includes('\\n') && getComputedStyle(value).whiteSpace === 'nowrap',
    borderless: cells.every(cell => {
      const style = getComputedStyle(cell);
      return ['Top', 'Right', 'Bottom', 'Left'].every(side =>
        parseFloat(style[`border${side}Width`]) === 0);
    }),
    contained: table.getBoundingClientRect().right <= transcript.getBoundingClientRect().right + 1,
  };
})()
""")
    if (
        whos_layout is None or
        "…]" not in whos_layout["value"] or
        not whos_layout["inline"] or
        not whos_layout["borderless"] or
        not whos_layout["contained"]
    ):
        fail(f":whos was not inline, elided, borderless, and contained: {whos_layout!r}")
    whos_click = evaluate_json("""
(() => {
  const rows = [...document.querySelectorAll('.mech-repl-symbols tbody tr')];
  const row = rows.find(candidate => candidate.firstElementChild?.textContent.trim() === 'qq');
  const name = row?.firstElementChild;
  if (!name) return null;
  window.__MECH_WHOS_RESULT_COUNT__ = document.querySelectorAll('.mech-repl-result').length;
  window.__MECH_WHOS_QQ_ECHO_COUNT__ = [...document.querySelectorAll('.mech-repl-source .repl-code')]
    .filter(code => code.textContent.trim() === 'qq').length;
  const contract = {
    bound: name.dataset.mechReplBound === 'true',
    role: name.getAttribute('role'),
    keyboard: name.tabIndex === 0,
    pointer: getComputedStyle(name).cursor,
  };
  name.click();
  return contract;
})()
""")
    if (
        whos_click is None or
        not whos_click["bound"] or
        whos_click["role"] != "button" or
        not whos_click["keyboard"] or
        whos_click["pointer"] != "pointer"
    ):
        fail(f":whos names were not interactive resident-value controls: {whos_click!r}")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-source .repl-code')]"
        ".filter(code => code.textContent.trim() === 'qq').length > "
        "(window.__MECH_WHOS_QQ_ECHO_COUNT__ || 0) && "
        "document.querySelectorAll('.mech-repl-result').length > "
        "(window.__MECH_WHOS_RESULT_COUNT__ || 0)",
        "clicking a :whos name into the console as ans",
    )
    submit("ans[100]")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-source .repl-code')]"
        ".some(code => code.textContent.trim() === 'ans[100]') && "
        "/100/.test([...document.querySelectorAll('.mech-repl-result')].at(-1)?.textContent || '')",
        "the :whos selection becoming the resident ans value",
    )
    evaluate("document.querySelector('#mech-smoke-large-var')?.remove()")
    evaluate("document.querySelector('#mech-smoke-var .mech-var-placeholder')?.click()")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-source .repl-code')].some((row) => row.textContent.trim() === 'answer') && "
        "document.querySelector('.mech-repl-transcript')?.lastElementChild?.classList.contains('mech-repl-active-prompt')",
        "clicking a document variable into the descending REPL transcript",
    )
    submit("ans + 1")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-result')].some((row) => /42/.test(row.textContent))",
        "the clicked document variable being bound as ans",
    )
    evaluate("document.querySelector('.mech-inline-mech-code')?.click()")
    submit("ans")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-result')].some((row) => /42/.test(row.textContent))",
        "inline document output binding ans",
    )
    submit(":step 256")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-info')].some((row) => "
        "/Advanced 256 resident step/.test(row.textContent))",
        "cooperative browser stepping completing across bounded animation-frame chunks",
    )
    submit(":step 1000000")
    wait_for(
        "document.querySelector('.repl-input')?.readOnly === true && "
        "document.querySelector('.repl-input')?.disabled === false && "
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'busy'",
        "the busy browser input remaining focusable for interruption",
    )
    evaluate("""
(() => {
  const input = document.querySelector('.repl-input');
  input?.dispatchEvent(new KeyboardEvent('keydown', {
    key: 'c', ctrlKey: true, bubbles: true, cancelable: true,
  }));
})()
""")
    wait_for(
        "document.querySelector('.repl-input')?.readOnly === false && "
        "document.querySelector('.repl-input')?.disabled === false && "
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'ready' && "
        "[...document.querySelectorAll('.mech-repl-transcript .mech-repl-diagnostic')].some((row) => /request interrupted/.test(row.textContent))",
        "Ctrl+C interrupting a cooperative browser step",
    )
    submit(":step 1000000")
    wait_for(
        "document.querySelector('.repl-input')?.readOnly === true && "
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'busy'",
        "cooperative ownership before a fallible interrupt response",
    )
    evaluate("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  const original = WasmDocument.prototype.replInterrupt;
  WasmDocument.prototype.replInterrupt = function(...args) {
    original.apply(this, args);
    WasmDocument.prototype.replInterrupt = original;
    throw new Error('synthetic interrupt response failure');
  };
  const input = document.querySelector('.repl-input');
  input?.dispatchEvent(new KeyboardEvent('keydown', {
    key: 'c', ctrlKey: true, bubbles: true, cancelable: true,
  }));
})()
""")
    wait_for(
        "document.querySelector('.repl-input')?.readOnly === false && "
        "document.querySelector('.repl-input')?.disabled === false && "
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'ready' && "
        "[...document.querySelectorAll('.mech-repl-transcript .mech-repl-error')].some((row) => "
        "/synthetic interrupt response failure/.test(row.textContent))",
        "ownership revocation surviving a fallible WASM interrupt response",
    )
    ownership_probe = evaluate("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  const original = WasmDocument.prototype.replFinishHostRequest;
  if (typeof original !== 'function') return null;
  window.__MECH_HOST_FINISH_CALLS__ = [];
  window.__MECH_THROW_NEXT_HOST_FINISH__ = true;
  window.__MECH_ORIGINAL_FINISH_HOST_REQUEST__ = original;
  WasmDocument.prototype.replFinishHostRequest = function(requestId) {
    window.__MECH_HOST_FINISH_CALLS__.push(requestId);
    const response = original.call(this, requestId);
    if (window.__MECH_THROW_NEXT_HOST_FINISH__) {
      window.__MECH_THROW_NEXT_HOST_FINISH__ = false;
      throw new Error('synthetic host finalization response failure');
    }
    return response;
  };
  return { installed: true };
})()
""")
    if ownership_probe != {"installed": True}:
        fail(f"could not instrument browser host finalization ownership: {ownership_probe!r}")
    submit(":docs browser-smoke/latency")
    wait_for(
        "document.querySelector('.repl-input')?.readOnly === true && "
        "document.querySelector('.repl-input')?.disabled === false && "
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'busy'",
        "documentation fetch marking the REPL busy without disabling Ctrl+C",
    )
    stale_host_request_id = evaluate_json(
        "document.querySelector('.mech-root')?.dataset.mechHostRequestId || null"
    )
    if not stale_host_request_id:
        fail("pending documentation request did not expose its ownership id")
    busy_selection = evaluate_json("""
(() => {
  const value = document.querySelector('#mech-smoke-var .mech-var-placeholder');
  if (!value) return null;
  const sourceRows = document.querySelectorAll('.mech-repl-source').length;
  const resultRows = document.querySelectorAll('.mech-repl-result').length;
  value.click();
  return {
    sourceRows,
    sourceRowsAfter: document.querySelectorAll('.mech-repl-source').length,
    resultRows,
    resultRowsAfter: document.querySelectorAll('.mech-repl-result').length,
  };
})()
""")
    if (
        busy_selection is None or
        busy_selection["sourceRowsAfter"] != busy_selection["sourceRows"] or
        busy_selection["resultRowsAfter"] != busy_selection["resultRows"]
    ):
        fail(f"a reflective click mutated the REPL during documentation loading: {busy_selection!r}")
    evaluate("""
(() => {
  const input = document.querySelector('.repl-input');
  input?.dispatchEvent(new KeyboardEvent('keydown', {
    key: 'c', ctrlKey: true, bubbles: true, cancelable: true,
  }));
})()
""")
    wait_for(
        "document.querySelector('.repl-input')?.readOnly === false && "
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'ready' && "
        "!document.querySelector('.mech-root')?.dataset.mechHostRequestId && "
        "[...document.querySelectorAll('.mech-repl-transcript .mech-repl-diagnostic')].some((row) => /documentation request interrupted/i.test(row.textContent))",
        "Ctrl+C canceling an awaiting documentation request",
    )
    submit(":docs browser-smoke/latency-next")
    wait_for(
        "document.querySelector('.repl-input')?.readOnly === true && "
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'busy' && "
        "window.__MECH_DOCUMENTATION_RELEASES__?.has('latency-next')",
        "the replacement documentation request taking host ownership",
    )
    evaluate("window.__MECH_DOCUMENTATION_RELEASES__.get('latency-next')?.()")
    wait_for(
        "Boolean(document.querySelector('[data-mech-documentation-topic=\"browser-smoke/latency-next\"]')) && "
        "document.querySelector('.repl-input')?.readOnly === false && "
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'ready' && "
        "[...document.querySelectorAll('.mech-repl-transcript .mech-repl-error')].some((row) => "
        "/synthetic host finalization response failure/.test(row.textContent))",
        "a newer documentation request releasing ownership before fallible finalization",
    )
    if evaluate("Boolean(document.querySelector('[data-mech-documentation-topic=\"browser-smoke/latency\"]'))"):
        fail("a canceled documentation response was accepted by a newer host request")
    evaluate("window.__MECH_DOCUMENTATION_RELEASES__.get('latency')?.()")
    evaluate("new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))")
    stale_finalizer = evaluate_json("""
(() => ({
  calls: [...(window.__MECH_HOST_FINISH_CALLS__ || [])],
  staleAppended: Boolean(document.querySelector(
    '[data-mech-documentation-topic="browser-smoke/latency"]'
  )),
  ready: document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'ready',
}))()
""")
    if (
        stale_finalizer is None or
        stale_host_request_id in stale_finalizer["calls"] or
        stale_finalizer["staleAppended"] or
        not stale_finalizer["ready"]
    ):
        fail(
            "a revoked documentation finalizer entered the current host response path: "
            f"request={stale_host_request_id!r}, state={stale_finalizer!r}"
        )
    evaluate("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  if (window.__MECH_ORIGINAL_FINISH_HOST_REQUEST__) {
    WasmDocument.prototype.replFinishHostRequest =
      window.__MECH_ORIGINAL_FINISH_HOST_REQUEST__;
    delete window.__MECH_ORIGINAL_FINISH_HOST_REQUEST__;
  }
})()
""")
    submit(":docs browser-smoke/rejected")
    wait_for(
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'ready' && "
        "[...document.querySelectorAll('.mech-repl-transcript .mech-repl-diagnostic')].some((row) => /missing|resident activation failed/.test(row.textContent))",
        "a semantically rejected documentation fragment returning control",
    )
    if evaluate("Boolean(document.querySelector('[data-mech-documentation-topic=\"browser-smoke/rejected\"]'))"):
        fail("semantically rejected documentation was appended to the output DOM")
    submit(":docs browser-smoke/recovered")
    wait_for(
        "Boolean(document.querySelector('[data-mech-documentation-topic=\"browser-smoke/recovered\"]')) && "
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'ready'",
        "accepted documentation reusing only the uncommitted runtime namespace",
    )
    documentation_namespace = evaluate_json("""
(() => {
  const rows = [...document.querySelectorAll('.mech-repl-documentation')];
  const ids = rows.flatMap(row => [...row.querySelectorAll('[id]')].map(element => element.id));
  const brokenReferences = rows.flatMap(row =>
    [...row.querySelectorAll('[href^="#"]')]
      .map(link => link.getAttribute('href').slice(1))
      .filter(id => !row.querySelector(`#${CSS.escape(id)}`)));
  return {
    ids,
    brokenReferences,
    globallyUnique: ids.every(id => document.querySelectorAll(`#${CSS.escape(id)}`).length === 1),
    outputAddressesPreserved: [...document.querySelectorAll(
      '.mech-repl-documentation .mech-inline-mech-code[id], ' +
      '.mech-repl-documentation .mech-block-output[id]'
    )].every(element => /^\\d+:\\d+$/.test(element.dataset.mechOutputAddress || '')),
  };
})()
""")
    if (
        documentation_namespace is None or
        not documentation_namespace["ids"] or
        len(documentation_namespace["ids"]) != len(set(documentation_namespace["ids"])) or
        not documentation_namespace["globallyUnique"] or
        documentation_namespace["brokenReferences"] or
        not documentation_namespace["outputAddressesPreserved"]
    ):
        fail(f"accepted documentation did not receive a complete local DOM namespace: {documentation_namespace!r}")
    submit(":help")
    wait_for(
        "Boolean(document.querySelector('.mech-repl-help')) && "
        "Boolean(document.querySelector('.mech-repl-logo')) && "
        "/┌─────────┐/.test(document.querySelector('.mech-repl-logo')?.textContent || '') && "
        "/:load/.test(document.querySelector('.mech-repl-help')?.textContent || '') && "
        "[...document.querySelectorAll('.mech-repl-help .mech-repl-row-muted')].some((row) => /:load/.test(row.textContent)) && "
        "document.querySelectorAll('.mech-repl-help th').length === 2",
        "the shared command registry with unavailable commands muted and no host column",
    )
    help_layout = evaluate_json("""
(() => {
  const table = document.querySelector('.mech-repl-help');
  const logo = document.querySelector('.mech-repl-logo');
  if (!table || !logo) return null;
  return {
    borderless: [...table.querySelectorAll('th, td')].every(cell => {
      const style = getComputedStyle(cell);
      return ['Top', 'Right', 'Bottom', 'Left'].every(side =>
        parseFloat(style[`border${side}Width`]) === 0);
    }),
    unavailableReasonLeaked: /unavailable:/.test(table.textContent),
    logoColored: getComputedStyle(logo).color ===
      getComputedStyle(document.querySelector('.repl-prompt')).color,
    logoPreservesBoxDrawing: getComputedStyle(logo).whiteSpace === 'pre',
    commandOutputIndented: parseFloat(
      getComputedStyle(table.closest('.mech-repl-response')).marginLeft) > 0,
  };
})()
""")
    if (
        help_layout is None or
        not help_layout["borderless"] or
        help_layout["unavailableReasonLeaked"] or
        not help_layout["logoColored"] or
        not help_layout["logoPreservesBoxDrawing"] or
        not help_layout["commandOutputIndented"]
    ):
        fail(f"browser help retained table rules or the removed Host column text: {help_layout!r}")
    console_instance = "repl-console" if label == "configured" else "repl"
    console_context = f"console://{console_instance}/output"
    submit(":capabilities")
    wait_for(
        f"[...document.querySelectorAll('.mech-repl-response')].some((row) => row.textContent.includes({json.dumps(console_context)}))",
        "the effective generated console namespace in browser capabilities",
    )
    submit(f'@out := {console_context}{{:write(line)}}\n@out/line <- "browser-output"\n@out/line <- "continued"')
    wait_for(
        "/browser-output\\s*continued/.test(document.querySelector('[data-mech-output-region=repl]')?.textContent || '') && "
        "document.querySelectorAll('[data-mech-output-region=repl] [data-mech-display-id]').length === 1 && "
        "document.querySelector('[data-mech-output-region=document]')?.children.length === 0",
        "explicit program output replacing the implicit final-statement projection",
    )
    display_id = evaluate_json(
        "document.querySelector('[data-mech-output-region=repl] [data-mech-display-id]')?.dataset.mechDisplayId || null"
    )
    if not display_id:
        fail("resident program output did not expose its stable display id")
    evaluate("""
(() => {
  const panel = document.querySelector('[data-mech-output-panel]');
  if (!panel?.parentElement) return false;
  window.__MECH_DETACHED_OUTPUT_PANEL__ = {
    panel,
    parent: panel.parentElement,
    next: panel.nextSibling,
  };
  panel.remove();
  return true;
})()
""")
    submit('@out/line <- "while-absent"')
    wait_for(
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'ready' && "
        "[...document.querySelectorAll('.mech-repl-source .repl-code')].some((row) => "
        "/while-absent/.test(row.textContent))",
        "the retained output turn completing while its pane is absent",
    )
    submit(":outputs")
    wait_for(
        f"[...document.querySelectorAll('.mech-repl-response')].some((row) => "
        f"row.textContent.includes({json.dumps(display_id)}) && "
        "/active/.test(row.textContent))",
        "the absent-pane output reaching the retained artifact journal",
    )
    evaluate("""
(() => {
  const saved = window.__MECH_DETACHED_OUTPUT_PANEL__;
  if (!saved) return false;
  saved.parent.insertBefore(saved.panel, saved.next);
  delete window.__MECH_DETACHED_OUTPUT_PANEL__;
  return true;
})()
""")
    submit(f":output {display_id}")
    wait_for(
        f"document.querySelector('[data-mech-console-tab=output]')?.getAttribute('aria-selected') === 'true' && "
        f"/browser-output[\\s\\S]*continued[\\s\\S]*while-absent/.test(document.querySelector('[data-mech-display-id=\"{display_id}\"]')?.textContent || '')",
        "focus reconstruction for retained output published while its pane was absent",
    )
    evaluate("""
(() => {
  const scene = JSON.stringify({
    width: 120,
    height: 80,
    background: '#080b12',
    circles: [
      { id: 'body-0', x: 40, y: 40, radius: 8, fill: '#ffd166', stroke: 'none', stroke_width: 0, opacity: 1 },
      { id: 'body-1', x: 80, y: 40, radius: 4, fill: '#4cc9f0', stroke: 'none', stroke_width: 0, opacity: 1 },
    ],
    lines: [],
  });
  window.dispatchEvent(new CustomEvent('mech:output', {
    detail: {
      source: { host: { name: 'scene://browser-smoke/frame', span: null } },
      stream: 'stdout',
      display_id: 'scene-browser-smoke',
      operation: 'create',
      content: {
        kind: 'scene',
        data: { representations: { representations: [
          { media_type: 'application/vnd.mech.scene+json', data: { encoding: 'text', value: scene } },
          { media_type: 'text/plain', data: { encoding: 'text', value: 'scene 120×80 (2 circles, 0 lines)' } },
        ] } },
      },
    },
  }));
  return true;
})()
""")
    wait_for(
        "document.querySelectorAll('[data-mech-display-id=scene-browser-smoke] [data-mech-rich-scene=true] circle').length === 2",
        "portable rich Scene output rendering as SVG in the Output pane",
    )
    root_isolation = evaluate_json("""
(() => {
  const root = document.querySelector('.mech-root');
  const localOutput = root?.querySelector('[data-mech-output-panel]');
  const localErrors = root?.querySelector('[data-mech-errors-panel]');
  if (!root || !localOutput || !localErrors) return null;
  const outputAnchor = document.createComment('mech-output-anchor');
  const errorsAnchor = document.createComment('mech-errors-anchor');
  localOutput.before(outputAnchor);
  localErrors.before(errorsAnchor);
  const foreign = document.createElement('aside');
  foreign.id = 'mech-smoke-foreign-panels';
  foreign.innerHTML = `
    <section data-mech-document-output></section>
    <section data-mech-document-errors></section>
  `;
  document.body.append(foreign);
  localOutput.remove();
  localErrors.remove();
  const send = (stream) => window.dispatchEvent(new CustomEvent('mech:output', {
    detail: {
      source: { host: { name: 'browser-smoke', span: null } },
      stream,
      display_id: null,
      operation: 'create',
      content: { kind: 'text', data: { text: 'must stay root-local' } },
    },
  }));
  send('stdout');
  send('stderr');
  const result = {
    foreignOutputChildren:
      foreign.querySelector('[data-mech-document-output]').children.length,
    foreignErrorChildren:
      foreign.querySelector('[data-mech-document-errors]').children.length,
  };
  outputAnchor.replaceWith(localOutput);
  errorsAnchor.replaceWith(localErrors);
  foreign.remove();
  return result;
})()
""")
    if root_isolation != {
        "foreignOutputChildren": 0,
        "foreignErrorChildren": 0,
    }:
        fail(f"document controller escaped its selected root: {root_isolation!r}")
    evaluate("""
(() => {
  const send = (stream, operation, text) => window.dispatchEvent(new CustomEvent('mech:output', {
    detail: {
      source: { host: { name: 'browser-smoke', span: null } },
      stream,
      display_id: 'cross-stream-smoke',
      operation,
      content: { kind: 'text', data: { text } },
    },
  }));
  send('stdout', 'create', 'stdout owner');
  send('stderr', 'replace', 'stderr owner');
  return true;
})()
""")
    wait_for(
        "document.querySelectorAll('[data-mech-display-id=cross-stream-smoke]').length === 1 && "
        "Boolean(document.querySelector('[data-mech-error-region=program] [data-mech-display-id=cross-stream-smoke]')) && "
        "!document.querySelector('[data-mech-output-region=repl] [data-mech-display-id=cross-stream-smoke]')",
        "global display identity moving from stdout to stderr",
    )
    evaluate("""
(() => {
  const panel = document.querySelector('[data-mech-errors-panel]');
  const parent = panel?.parentElement;
  const next = panel?.nextSibling;
  panel?.remove();
  window.dispatchEvent(new CustomEvent('mech:output', {
    detail: {
      source: { host: { name: 'browser-smoke', span: null } },
      stream: 'stderr',
      display_id: 'cross-stream-smoke',
      operation: 'remove',
      content: { kind: 'text', data: { text: '' } },
    },
  }));
  if (parent && panel) parent.insertBefore(panel, next);
  return true;
})()
""")
    wait_for(
        "!document.querySelector('[data-mech-display-id=cross-stream-smoke]')",
        "display removal while the event's destination pane is absent",
    )
    evaluate("document.querySelector('[data-mech-console-tab=console]')?.click()")
    repl_event_rejection = evaluate_json("""
(() => {
  const controller = window.__MECH_SMOKE_DOCUMENT__;
  const transcript = document.querySelector('.mech-repl-transcript');
  const panels = [...document.querySelectorAll('[data-mech-console-panel]')];
  if (!controller || !transcript || panels.length !== 3) return null;
  const before = JSON.stringify({
    transcript: transcript.innerHTML,
    panels: panels.map(panel => ({
      name: panel.dataset.mechConsolePanel,
      className: panel.className,
      html: panel.innerHTML,
    })),
  });
  let rejection = '';
  try {
    controller.replPublishProgramEvent({
      channel: 'repl',
      event: { kind: 'clear', payload: 'interaction' },
    });
  } catch (error) {
    rejection = String(error);
  }
  const after = JSON.stringify({
    transcript: transcript.innerHTML,
    panels: panels.map(panel => ({
      name: panel.dataset.mechConsolePanel,
      className: panel.className,
      html: panel.innerHTML,
    })),
  });
  return {
    rejectedAtSessionBoundary:
      /program producers cannot publish REPL control events/.test(rejection),
    presentationUnchanged: before === after,
  };
})()
""")
    if repl_event_rejection is None or not all(repl_event_rejection.values()):
        fail(
            "the served WASM boundary accepted or presented a program-owned REPL event: "
            f"{repl_event_rejection!r}"
        )
    evaluate("""
(() => window.__MECH_SMOKE_DOCUMENT__?.replPublishProgramEvent({
  channel: 'diagnostic',
  event: {
    id: 'program-browser-smoke',
    owner: 'interaction',
    severity: 'error',
    phase: 'execute',
    code: 'ProgramBrowserSmoke',
    message: 'persistent program diagnostic',
    source: null,
    notes: [],
    related: [],
  },
}))()
""")
    submit(":outputs")
    wait_for(
        "Boolean(document.querySelector('[data-mech-error-region=program-diagnostics] "
        ".mech-program-diagnostic[data-mech-diagnostic-id=program-browser-smoke]')) && "
        "document.querySelector('[data-mech-console-tab=errors]')?.dataset.mechConsoleUnread === 'true' && "
        "document.querySelector('[data-mech-console-tab=console]')?.getAttribute('aria-selected') === 'true'",
        "a program-owned diagnostic routing into Errors without stealing Console focus",
    )
    evaluate("""
(() => {
  const region = document.querySelector('[data-mech-error-region=program]');
  if (!region) return false;
  const marker = document.createElement('article');
  marker.dataset.mechProgramStderrSmoke = 'true';
  marker.textContent = 'program stderr lifecycle marker';
  region.append(marker);
  return true;
})()
""")
    evaluate("""
(() => {
  window.dispatchEvent(new CustomEvent('mech:output', {
    detail: {
      source: { host: { name: 'browser-smoke', span: null } },
      stream: 'stdout',
      display_id: null,
      operation: 'clear',
      content: { kind: 'text', data: { text: '' } },
    },
  }));
  return true;
})()
""")
    wait_for(
        "!document.querySelector('[data-mech-output-region=repl] [data-mech-display-id]') && "
        "(document.querySelector('[data-mech-error-region=program]')?.children.length || 0) === 0 && "
        "Boolean(document.querySelector('[data-mech-error-region=program-diagnostics] "
        ".mech-program-diagnostic[data-mech-diagnostic-id=program-browser-smoke]'))",
        "program stream clearing without deleting program diagnostics",
    )
    evaluate("document.querySelector('[data-mech-console-tab=output]')?.click()")
    wait_for(
        "document.querySelector('[data-mech-console-tab=output]')?.getAttribute('aria-selected') === 'true' && "
        "!document.querySelector('[data-mech-console-panel=output]')?.hidden && "
        "Boolean(document.querySelector('[data-mech-output-panel] [data-mech-selection-token]'))",
        "the Output console tab restoring the fixed implicit program result after stream clearing",
    )
    submit(":clear")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-info')].some((row) => /Resident workspace cleared/.test(row.textContent)) && "
        "document.querySelector('#mech-smoke-var .mech-var-placeholder')?.dataset.mechValueAvailable === 'false' && "
        "document.querySelector('#mech-smoke-var .mech-var-placeholder')?.textContent.trim() === '—' && "
        "(document.querySelector('[data-mech-output-region=document]')?.children.length || 0) === 0",
        "the document-backed resident workspace clear",
    )
    submit(":clc")
    wait_for(
        "document.querySelector('.mech-repl-transcript')?.children.length === 1 && "
        "document.querySelector('.mech-repl-transcript')?.lastElementChild?.classList.contains('mech-repl-active-prompt')",
        "the cleared browser console retaining its active bottom prompt",
    )
    typed_backtick = evaluate_json("""
(() => {
  const root = document.querySelector('.mech-root');
  const input = document.querySelector('.repl-input');
  if (!root || !input) return null;
  const before = root.dataset.mechConsoleOpen;
  input.focus();
  input.value = 'terminal-draft';
  const event = new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  });
  input.dispatchEvent(event);
  return {
    consoleOpen: root.dataset.mechConsoleOpen,
    closed: before === 'true' && root.dataset.mechConsoleOpen === 'false',
    prevented: event.defaultPrevented,
    value: input.value,
  };
})()
""")
    if typed_backtick != {
        "consoleOpen": "false",
        "closed": True,
        "prevented": True,
        "value": "terminal-draft",
    }:
        fail(f"the focused REPL did not capture backtick as its console command: {typed_backtick!r}")
    toggled = evaluate_json("""
(() => {
  const root = document.querySelector('.mech-root');
  if (!root) return null;
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  return {
    reopened: root.dataset.mechConsoleOpen,
    consoleActive: document.querySelector('[data-mech-console-tab=console]')?.getAttribute('aria-selected') === 'true',
  };
})()
""")
    if toggled != {"reopened": "true", "consoleActive": True}:
        fail(f"backtick did not reopen the browser REPL: {toggled!r}")


def assert_console_tab_isolation():
    state = evaluate_json("""
(() => {
  const root = document.querySelector('.mech-root');
  const outputTab = document.querySelector('[data-mech-console-tab=output]');
  if (!root || !outputTab) return null;

  const foreign = document.querySelector('#mech-smoke-unrelated-controls');
  if (!foreign) return null;
  const foreignTab = foreign.querySelector('[data-tab="output"]');
  const foreignPanel = foreign.querySelector('[data-panel="output"]');
  const foreignResize = foreign.querySelector('.resize-handle');
  const foreignButton = foreign.querySelector('[data-mech-like-but-not-owned-control]');
  const pane = document.querySelector('[data-mech-console-pane]');
  if (!foreignTab || !foreignPanel || !foreignResize || !foreignButton || !pane) return null;

  outputTab.click();
  const before = {
    consoleSize: root.style.getPropertyValue('--mech-console-size'),
    consoleOpen: root.dataset.mechConsoleOpen,
    outputAria: outputTab.getAttribute('aria-selected'),
    foreignPanelDisplay: getComputedStyle(foreignPanel).display,
    foreignResizePosition: getComputedStyle(foreignResize).position,
    foreignResizeCursor: getComputedStyle(foreignResize).cursor,
  };
  const rect = foreignResize.getBoundingClientRect();
  const startX = rect.left + Math.max(1, rect.width / 2);
  const startY = rect.top + Math.max(1, rect.height / 2);
  foreignResize.dispatchEvent(new PointerEvent('pointerdown', {
    bubbles: true, cancelable: true, pointerId: 74, clientX: startX, clientY: startY,
  }));
  window.dispatchEvent(new PointerEvent('pointermove', {
    bubbles: true, pointerId: 74, clientX: startX - 48, clientY: startY,
  }));
  window.dispatchEvent(new PointerEvent('pointerup', {
    bubbles: true, pointerId: 74, clientX: startX - 48, clientY: startY,
  }));
  foreignButton.click();

  return {
    foreignHidden: foreignPanel.hidden,
    foreignPanelActive: foreignPanel.classList.contains('foreign-panel-active'),
    foreignTabActive: foreignTab.classList.contains('foreign-tab-active'),
    foreignAria: foreignTab.getAttribute('aria-selected'),
    foreignPointerEvents: Number(foreign.dataset.mechSmokePointerEvents || '0'),
    foreignButtonEvents: Number(foreign.dataset.mechSmokeButtonEvents || '0'),
    consoleSizeUnchanged: root.style.getPropertyValue('--mech-console-size') === before.consoleSize,
    consoleOpenUnchanged: root.dataset.mechConsoleOpen === before.consoleOpen,
    consoleTabAriaUnchanged: outputTab.getAttribute('aria-selected') === before.outputAria,
    outputActive: !document.querySelector('[data-mech-console-panel=output]')?.hidden,
    foreignPanelDisplay: before.foreignPanelDisplay,
    foreignResizePosition: before.foreignResizePosition,
    foreignResizeCursor: before.foreignResizeCursor,
  };
})()
""")
    if state is None or not state["outputActive"]:
        fail(f"document console Output tab did not activate: {state!r}")
    if (
        state["foreignHidden"] or
        not state["foreignPanelActive"] or
        not state["foreignTabActive"] or
        state["foreignAria"] != "true" or
        state["foreignPointerEvents"] != 1 or
        state["foreignButtonEvents"] != 1 or
        state["foreignPanelDisplay"] != "block" or
        state["foreignResizePosition"] != "static" or
        state["foreignResizeCursor"] == "ew-resize" or
        not state["consoleSizeUnchanged"] or
        not state["consoleOpenUnchanged"] or
        not state["consoleTabAriaUnchanged"]
    ):
        fail(f"document console captured an unrelated custom-shim control: {state!r}")


def assert_right_console_resize_direction():
    state = evaluate_json("""
(() => {
  const root = document.querySelector('.mech-root');
  const pane = document.querySelector('[data-mech-console-pane]');
  const handle = root?.querySelector(':scope > [data-mech-console-resizer]:not([data-mech-console-edge-handle])');
  if (!root || !pane || !handle) return null;
  let pointerId = 80;
  const drag = (...deltaXs) => {
    const rect = handle.getBoundingClientRect();
    const startX = rect.left + Math.max(1, rect.width / 2);
    const startY = rect.top + Math.max(1, rect.height / 2);
    pointerId += 1;
    handle.dispatchEvent(new PointerEvent('pointerdown', {
      bubbles: true, cancelable: true, pointerId, clientX: startX, clientY: startY,
    }));
    const states = [];
    for (const deltaX of deltaXs) {
      window.dispatchEvent(new PointerEvent('pointermove', {
        bubbles: true, pointerId, clientX: startX + deltaX, clientY: startY,
      }));
      states.push({
        open: root.dataset.mechConsoleOpen,
        fullscreen: root.dataset.mechConsoleMode !== 'docked',
        mode: root.dataset.mechConsoleMode,
        handleVisible: (() => {
          const rect = handle.getBoundingClientRect();
          return getComputedStyle(handle).display !== 'none' &&
            rect.width > 0 && rect.right > 0 && rect.left < innerWidth;
        })(),
        fillsViewport: (() => {
          const paneRect = pane.getBoundingClientRect();
          return paneRect.left <= 1 && paneRect.top <= 1 &&
            paneRect.right >= innerWidth - 1 && paneRect.bottom >= innerHeight - 1;
        })(),
        fallbackWorkspaceComplete: (() => {
          const paneRect = pane.getBoundingClientRect();
          const panels = [...pane.querySelectorAll('[data-mech-console-panel]')];
          return panels.length === 3 && panels.every(panel => {
            const rect = panel.getBoundingClientRect();
            return !panel.hidden && getComputedStyle(panel).display !== 'none' &&
              rect.width > 0 && rect.height > 0 &&
              rect.left >= paneRect.left - 1 && rect.right <= paneRect.right + 1 &&
              rect.top >= paneRect.top - 1 && rect.bottom <= paneRect.bottom + 1 &&
              getComputedStyle(panel, '::before').content.replaceAll('"', '') ===
                panel.dataset.mechConsoleLabel;
          });
        })(),
      });
    }
    const finalDelta = deltaXs.at(-1) || 0;
    window.dispatchEvent(new PointerEvent('pointerup', {
      bubbles: true, pointerId, clientX: startX + finalDelta, clientY: startY,
    }));
    return states;
  };
  const before = pane.getBoundingClientRect().width;
  const rect = handle.getBoundingClientRect();
  const cancelX = rect.left + Math.max(1, rect.width / 2);
  const cancelY = rect.top + Math.max(1, rect.height / 2);
  handle.dispatchEvent(new PointerEvent('pointerdown', {
    bubbles: true, cancelable: true, pointerId: 701, button: 0,
    clientX: cancelX, clientY: cancelY,
  }));
  window.dispatchEvent(new PointerEvent('pointermove', {
    bubbles: true, pointerId: 702, clientX: cancelX - 80, clientY: cancelY,
  }));
  const ignoredForeignPointer = Math.abs(pane.getBoundingClientRect().width - before) <= 1;
  window.dispatchEvent(new PointerEvent('pointercancel', {
    bubbles: true, pointerId: 701, clientX: cancelX, clientY: cancelY,
  }));
  window.dispatchEvent(new PointerEvent('pointermove', {
    bubbles: true, pointerId: 701, clientX: cancelX - 80, clientY: cancelY,
  }));
  const cancelledCleanly =
    Math.abs(pane.getBoundingClientRect().width - before) <= 1 &&
    !document.body.classList.contains('is-resizing') &&
    !document.body.hasAttribute('data-mech-resize-axis');
  drag(-48);
  const widened = pane.getBoundingClientRect().width;

  drag(widened);
  const collapsed =
    root.dataset.mechConsoleOpen === 'false' &&
    pane.hidden;
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  const reopened = root.dataset.mechConsoleOpen === 'true';

  const reopenedWidth = pane.getBoundingClientRect().width;
  const transitions = drag(
    -root.getBoundingClientRect().width,
    reopenedWidth - Math.max(500, Math.min(900, root.getBoundingClientRect().width * 0.6)),
  );
  const entered = transitions[0];
  const exited = transitions[1];
  const fullscreen =
    entered?.fullscreen === true &&
    entered?.mode === 'drag' &&
    entered?.handleVisible === true &&
    entered?.fillsViewport === true &&
    entered?.fallbackWorkspaceComplete === true;
  const returned =
    exited?.fullscreen === false &&
    exited?.mode === 'docked' &&
    exited?.open === 'true';
  return {
    before, widened, collapsed, reopened, fullscreen, returned,
    ignoredForeignPointer, cancelledCleanly,
  };
})()
""")
    if (
        state is None or
        state["widened"] <= state["before"] or
        not state["collapsed"] or
        not state["reopened"] or
        not state["fullscreen"] or
        not state["returned"] or
        not state["ignoredForeignPointer"] or
        not state["cancelledCleanly"]
    ):
        fail(f"right-console drag thresholds did not widen, collapse, fullscreen, and return: {state!r}")


def assert_layout_persistence():
    expected = evaluate_json("""
(() => {
  const root = document.querySelector('[data-mech-repl-host]');
  const pane = document.querySelector('[data-mech-console-pane]');
  const handle = root?.querySelector(
    ':scope > [data-mech-console-resizer]:not([data-mech-console-edge-handle])');
  if (!root || !pane || !handle || root.dataset.mechConsoleMode !== 'docked') return null;
  const rect = handle.getBoundingClientRect();
  const x = rect.left + rect.width / 2;
  const y = rect.top + rect.height / 2;
  const initialWidth = pane.getBoundingClientRect().width;
  const maximumWidth = Math.floor(root.getBoundingClientRect().width * 0.8);
  const targetWidth = Math.max(initialWidth, maximumWidth - 16);
  handle.dispatchEvent(new PointerEvent('pointerdown', {
    bubbles: true, cancelable: true, pointerId: 811, button: 0, clientX: x, clientY: y,
  }));
  window.dispatchEvent(new PointerEvent('pointermove', {
    bubbles: true, pointerId: 811, clientX: x - (targetWidth - initialWidth), clientY: y,
  }));
  window.dispatchEvent(new PointerEvent('pointerup', {
    bubbles: true, pointerId: 811, clientX: x - (targetWidth - initialWidth), clientY: y,
  }));
  const spacer = document.createElement('div');
  spacer.id = 'mech-late-layout-spacer';
  spacer.style.height = '900px';
  (document.querySelector('.content-column') || document.body).append(spacer);
  const shell = document.querySelector('.content-shell');
  const shellStyle = shell ? getComputedStyle(shell) : null;
  const owner = shell && ['auto', 'scroll', 'overlay'].includes(shellStyle.overflowY) &&
    shell.scrollHeight > shell.clientHeight + 1 ? shell : window;
  const maximumScroll = owner === window
    ? Math.max(0, document.documentElement.scrollHeight - innerHeight)
    : Math.max(0, owner.scrollHeight - owner.clientHeight);
  const styleOwner = owner === window ? document.documentElement : owner;
  const scrollBehavior = styleOwner.style.scrollBehavior;
  styleOwner.style.scrollBehavior = 'auto';
  owner.scrollTo(0, maximumScroll);
  styleOwner.style.scrollBehavior = scrollBehavior;
  return {
    width: pane.getBoundingClientRect().width,
    owner: owner === window ? 'window' : 'content-shell',
    scrollY: owner === window ? window.scrollY : owner.scrollTop,
    maximumScroll,
  };
})()
""")
    if expected is None or expected["width"] <= 0 or expected["maximumScroll"] <= 0:
        fail(f"could not establish a persistent REPL size and page position: {expected!r}")
    late_layout_script = devtools.call(
        "Page.addScriptToEvaluateOnNewDocument",
        {"source": f"""
for (const [key, value] of Object.entries(localStorage)) {{
  if (!key.startsWith('mech:document-layout:v1:')) continue;
  const layout = JSON.parse(value);
  layout.page = {{ owner: {json.dumps(expected['owner'])}, x: 0, y: {expected['scrollY']} }};
  localStorage.setItem(key, JSON.stringify(layout));
}}
document.addEventListener('DOMContentLoaded', () => {{
  setTimeout(() => {{
    if (document.getElementById('mech-late-layout-spacer')) return;
    const spacer = document.createElement('div');
    spacer.id = 'mech-late-layout-spacer';
    spacer.style.height = '900px';
    (document.querySelector('.content-column') || document.body).append(spacer);
  }}, 450);
}}, {{ once: true }});
"""},
        session_id,
    ).get("identifier")
    devtools.call("Page.navigate", {"url": page_url}, session_id)
    wait_for(
        "document.documentElement?.dataset.mechDocumentStatus === 'ready' && "
        "Boolean(document.querySelector('.repl-input'))",
        "the document reloading for layout persistence coverage",
        timeout=45,
    )
    try:
        expected_owner = json.dumps(expected["owner"])
        wait_for(
            f"Math.abs((document.querySelector('[data-mech-console-pane]')?.getBoundingClientRect().width || 0) - {expected['width']}) <= 2 && "
            "(() => { "
            "const shell = document.querySelector('.content-shell'); "
            "const style = shell ? getComputedStyle(shell) : null; "
            "const owner = shell && ['auto', 'scroll', 'overlay'].includes(style.overflowY) && "
            "shell.scrollHeight > shell.clientHeight + 1 ? shell : window; "
            f"return (owner === window ? 'window' : 'content-shell') === {expected_owner} && "
            f"Math.abs((owner === window ? window.scrollY : owner.scrollTop) - {expected['scrollY']}) <= 2; "
            "})()",
            "the saved REPL opening size and page position restoring after refresh",
        )
    except AssertionError:
        restored = evaluate_json("""
(() => ({
  width: document.querySelector('[data-mech-console-pane]')?.getBoundingClientRect().width || 0,
  windowScrollY: window.scrollY,
  shellScrollY: document.querySelector('.content-shell')?.scrollTop || 0,
  shellMaximumScroll: Math.max(0, (document.querySelector('.content-shell')?.scrollHeight || 0) -
    (document.querySelector('.content-shell')?.clientHeight || 0)),
  persisted: Object.entries(localStorage).find(([key]) =>
    key.startsWith('mech:document-layout:v1:')) || null,
}))()
""")
        fail(f"layout persistence mismatch: expected={expected!r}, restored={restored!r}")
    if late_layout_script:
        devtools.call(
            "Page.removeScriptToEvaluateOnNewDocument",
            {"identifier": late_layout_script},
            session_id,
        )
    wait_for(
        "!document.documentElement.dataset.mechPagePositionRestore",
        "the refreshed layout restore reaching a stable mapping",
    )
    devtools.call(
        "Emulation.setDeviceMetricsOverride",
        {"width": 1100, "height": 900, "deviceScaleFactor": 1, "mobile": False},
        session_id,
    )
    wait_for(
        "(document.querySelector('[data-mech-console-pane]')?.getBoundingClientRect().width || Infinity) <= "
        "(document.querySelector('[data-mech-repl-host]')?.getBoundingClientRect().width || 0) * 0.8 + 2",
        "the restored console size re-clamping after viewport narrowing",
    )
    if label == "default":
        desktop_position = evaluate_json("""
(() => {
  const shell = document.querySelector('.content-shell');
  if (!shell) return null;
  if (!document.getElementById('mech-late-layout-spacer')) {
    const spacer = document.createElement('div');
    spacer.id = 'mech-late-layout-spacer';
    spacer.style.height = '900px';
    (document.querySelector('.content-column') || document.body).append(spacer);
  }
  const maximum = Math.max(0, shell.scrollHeight - shell.clientHeight);
  const y = Math.min(320, maximum);
  let origin = 0;
  for (let element = shell; element; element = element.offsetParent) origin += element.offsetTop;
  shell.scrollTo(0, y);
  return { y: shell.scrollTop, maximum, origin };
})()
""")
        if desktop_position is None or desktop_position["y"] < 200:
            fail(f"could not establish desktop content-shell persistence: {desktop_position!r}")
        wait_for(
            "(() => { "
            "const entry = Object.entries(localStorage).find(([key]) => "
            "key.startsWith('mech:document-layout:v1:')); "
            "if (!entry) return false; "
            "const page = JSON.parse(entry[1]).page; "
            f"return page?.owner === 'content-shell' && page?.coordinateSpace === 'content-shell' && "
            f"Math.abs(page.y - {desktop_position['y']}) <= 2; "
            "})()",
            "desktop scrolling persisting canonical content coordinates",
        )
        desktop_layout = evaluate_json("""
Object.entries(localStorage).find(([key]) => key.startsWith('mech:document-layout:v1:')) || null
""")
        if desktop_layout is None:
            fail("desktop scrolling did not produce a persisted layout entry")
        desktop_transfer_script = devtools.call(
            "Page.addScriptToEvaluateOnNewDocument",
            {"source": f"localStorage.setItem({json.dumps(desktop_layout[0])}, {json.dumps(desktop_layout[1])});"},
            session_id,
        ).get("identifier")
        devtools.call(
            "Emulation.setDeviceMetricsOverride",
            {"width": 650, "height": 900, "deviceScaleFactor": 1, "mobile": False},
            session_id,
        )
        devtools.call("Page.navigate", {"url": page_url}, session_id)
        wait_for(
            "document.documentElement?.dataset.mechDocumentStatus === 'ready' && "
            "getComputedStyle(document.querySelector('.content-shell')).overflowY === 'visible'",
            "the mobile document reloading for cross-owner persistence",
            timeout=45,
        )
        if desktop_transfer_script:
            devtools.call(
                "Page.removeScriptToEvaluateOnNewDocument",
                {"identifier": desktop_transfer_script},
                session_id,
            )
        mobile_expected = evaluate_json(f"""
(() => {{
  const shell = document.querySelector('.content-shell');
  if (!document.getElementById('mech-late-layout-spacer')) {{
    const spacer = document.createElement('div');
    spacer.id = 'mech-late-layout-spacer';
    spacer.style.height = '900px';
    (document.querySelector('.content-column') || document.body).append(spacer);
  }}
  let origin = 0;
  for (let element = shell; element; element = element.offsetParent) origin += element.offsetTop;
  return {{ y: origin + {desktop_position['y']}, origin }};
}})()
""")
        wait_for(
            f"Math.abs(window.scrollY - {mobile_expected['y']}) <= 2",
            "a desktop content-shell offset translating onto the mobile window",
        )
        wait_for(
            "!document.documentElement.dataset.mechPagePositionRestore",
            "the mobile canonical restore reaching a stable mapping",
        )
        if mobile_expected["origin"] <= desktop_position["origin"] + 20:
            fail(
                "responsive persistence coverage did not cross the compact-header origin: "
                f"desktop={desktop_position!r}, mobile={mobile_expected!r}"
            )
        delayed_shell_script = devtools.call(
            "Page.addScriptToEvaluateOnNewDocument",
            {"source": f"""
localStorage.setItem({json.dumps(desktop_layout[0])}, {json.dumps(desktop_layout[1])});
document.addEventListener('DOMContentLoaded', () => {{
  const shell = document.querySelector('.content-shell');
  const parent = shell?.parentNode;
  if (!shell || !parent) return;
  const next = shell.nextSibling;
  shell.remove();
  const temporaryRange = document.createElement('div');
  temporaryRange.id = 'mech-shellless-scroll-range';
  temporaryRange.style.height = '2000px';
  document.body.append(temporaryRange);
  window.__MECH_DELAYED_SHELL__ = {{ shell, parent, next, temporaryRange }};
  document.documentElement.dataset.mechDelayedShell = 'missing';
}}, {{ once: true }});
"""},
            session_id,
        ).get("identifier")
        devtools.call("Page.navigate", {"url": page_url}, session_id)
        wait_for(
            "document.documentElement?.dataset.mechDocumentStatus === 'ready' && "
            "document.documentElement?.dataset.mechDelayedShell === 'missing' && "
            "document.documentElement?.dataset.mechPagePositionRestore === 'waiting-anchor' && "
            f"Math.max(0, document.documentElement.scrollHeight - innerHeight) >= {desktop_position['y']}",
            "an observed canonical restore attempt waiting on a missing reachable anchor",
            timeout=45,
        )
        if delayed_shell_script:
            devtools.call(
                "Page.removeScriptToEvaluateOnNewDocument",
                {"identifier": delayed_shell_script},
                session_id,
            )
        evaluate("""
(() => {
  const held = window.__MECH_DELAYED_SHELL__;
  if (!held) return;
  held.temporaryRange.remove();
  held.parent.insertBefore(
    held.shell,
    held.next?.parentNode === held.parent ? held.next : null,
  );
  if (!held.shell.querySelector('#mech-late-layout-spacer')) {
    const spacer = document.createElement('div');
    spacer.id = 'mech-late-layout-spacer';
    spacer.style.height = '900px';
    (held.shell.querySelector('.content-column') || held.shell).append(spacer);
  }
  document.documentElement.dataset.mechDelayedShell = 'restored';
})()
""")
        delayed_shell_expected = evaluate_json(f"""
(() => {{
  const shell = document.querySelector('.content-shell');
  let origin = 0;
  for (let element = shell; element; element = element.offsetParent) origin += element.offsetTop;
  return {{ y: origin + {desktop_position['y']}, origin }};
}})()
""")
        wait_for(
            f"Math.abs(window.scrollY - {delayed_shell_expected['y']}) <= 2",
            "canonical restoration waiting for its delayed content-shell anchor",
        )
        wait_for(
            "!document.documentElement.dataset.mechPagePositionRestore",
            "the delayed canonical restore reaching a stable mapping",
        )
        mobile_position = evaluate_json("""
(() => {
  const shell = document.querySelector('.content-shell');
  let origin = 0;
  for (let element = shell; element; element = element.offsetParent) origin += element.offsetTop;
  const maximum = Math.max(0, document.documentElement.scrollHeight - innerHeight);
  const contentMaximum = Math.max(0, maximum - origin);
  // Deliberately move away from the restored desktop offset so this must
  // emit a real window scroll event before persistence can pass.
  const target = Math.min(140, contentMaximum);
  const scrollBehavior = document.documentElement.style.scrollBehavior;
  document.documentElement.style.scrollBehavior = 'auto';
  window.scrollTo(0, origin + target);
  document.documentElement.style.scrollBehavior = scrollBehavior;
  return { y: target, maximum, origin };
})()
""")
        if mobile_position is None or mobile_position["y"] < 100:
            fail(f"could not establish mobile window persistence: {mobile_position!r}")
        wait_for(
            f"Math.abs(window.scrollY - {mobile_position['origin'] + mobile_position['y']}) <= 2",
            "the deliberate mobile window scroll completing",
        )
        try:
            wait_for(
                "(() => { "
                "const entry = Object.entries(localStorage).find(([key]) => "
                "key.startsWith('mech:document-layout:v1:')); "
                "if (!entry) return false; "
                "const page = JSON.parse(entry[1]).page; "
                f"return page?.owner === 'window' && page?.coordinateSpace === 'content-shell' && "
                f"Math.abs(page.y - {mobile_position['y']}) <= 2; "
                "})()",
                "mobile scrolling persisting canonical content coordinates",
            )
        except AssertionError:
            mobile_persistence = evaluate_json("""
(() => {
  const shell = document.querySelector('.content-shell');
  let origin = 0;
  for (let element = shell; element; element = element.offsetParent) origin += element.offsetTop;
  const entry = Object.entries(localStorage).find(([key]) =>
    key.startsWith('mech:document-layout:v1:')) || null;
  return {
    windowY: window.scrollY,
    origin,
    contentY: window.scrollY - origin,
    shellY: shell?.scrollTop || 0,
    shellOverflowY: shell ? getComputedStyle(shell).overflowY : null,
    page: entry ? JSON.parse(entry[1]).page : null,
  };
})()
""")
            fail(
                "mobile canonical persistence mismatch: "
                f"expected={mobile_position!r}, actual={mobile_persistence!r}"
            )
        mobile_layout = evaluate_json("""
Object.entries(localStorage).find(([key]) => key.startsWith('mech:document-layout:v1:')) || null
""")
        if mobile_layout is None:
            fail("mobile scrolling did not produce a persisted layout entry")
        mobile_transfer_script = devtools.call(
            "Page.addScriptToEvaluateOnNewDocument",
            {"source": f"localStorage.setItem({json.dumps(mobile_layout[0])}, {json.dumps(mobile_layout[1])});"},
            session_id,
        ).get("identifier")
        devtools.call(
            "Emulation.setDeviceMetricsOverride",
            {"width": 1100, "height": 900, "deviceScaleFactor": 1, "mobile": False},
            session_id,
        )
        devtools.call("Page.navigate", {"url": page_url}, session_id)
        wait_for(
            "document.documentElement?.dataset.mechDocumentStatus === 'ready' && "
            "/auto|scroll|overlay/.test(getComputedStyle(document.querySelector('.content-shell')).overflowY)",
            "the desktop document reloading for reverse cross-owner persistence",
            timeout=45,
        )
        if mobile_transfer_script:
            devtools.call(
                "Page.removeScriptToEvaluateOnNewDocument",
                {"identifier": mobile_transfer_script},
                session_id,
            )
        evaluate("""
(() => {
  if (document.getElementById('mech-late-layout-spacer')) return;
  const spacer = document.createElement('div');
  spacer.id = 'mech-late-layout-spacer';
  spacer.style.height = '900px';
  (document.querySelector('.content-column') || document.body).append(spacer);
})()
""")
        wait_for(
            f"Math.abs(document.querySelector('.content-shell').scrollTop - {mobile_position['y']}) <= 2",
            "a mobile window offset translating back onto the desktop content shell",
        )
        wait_for(
            "!document.documentElement.dataset.mechPagePositionRestore",
            "the canonical reverse restore reaching a stable mapping",
        )

        devtools.call(
            "Emulation.setDeviceMetricsOverride",
            {"width": 650, "height": 900, "deviceScaleFactor": 1, "mobile": False},
            session_id,
        )
        wait_for(
            "getComputedStyle(document.querySelector('.content-shell')).overflowY === 'visible'",
            "the explicit-window persistence source switching to its mobile layout",
        )
        shellless_position = evaluate_json("""
(() => {
  const shell = document.querySelector('.content-shell');
  if (!shell) return null;
  shell.remove();
  const temporaryRange = document.createElement('div');
  temporaryRange.id = 'mech-shellless-scroll-range';
  temporaryRange.style.height = '2000px';
  document.body.append(temporaryRange);
  const scrollBehavior = document.documentElement.style.scrollBehavior;
  document.documentElement.style.scrollBehavior = 'auto';
  window.scrollTo(0, 260);
  document.documentElement.style.scrollBehavior = scrollBehavior;
  return { x: window.scrollX, y: window.scrollY };
})()
""")
        if shellless_position is None or shellless_position["y"] < 200:
            fail(f"could not establish shell-less window persistence: {shellless_position!r}")
        wait_for(
            "(() => { "
            "const entry = Object.entries(localStorage).find(([key]) => "
            "key.startsWith('mech:document-layout:v1:')); "
            "if (!entry) return false; "
            "const page = JSON.parse(entry[1]).page; "
            f"return page?.owner === 'window' && page?.coordinateSpace === 'window' && "
            f"Math.abs(page.y - {shellless_position['y']}) <= 2; "
            "})()",
            "a real shell-less scroll persisting explicit window coordinates",
        )
        shellless_layout = evaluate_json("""
Object.entries(localStorage).find(([key]) => key.startsWith('mech:document-layout:v1:')) || null
""")
        if shellless_layout is None:
            fail("shell-less scrolling did not produce a persisted layout entry")
        delayed_window_script = devtools.call(
            "Page.addScriptToEvaluateOnNewDocument",
            {"source": f"""
localStorage.setItem({json.dumps(shellless_layout[0])}, {json.dumps(shellless_layout[1])});
document.addEventListener('DOMContentLoaded', () => {{
  const shell = document.querySelector('.content-shell');
  const parent = shell?.parentNode;
  if (!shell || !parent) return;
  const next = shell.nextSibling;
  shell.remove();
  const temporaryRange = document.createElement('div');
  temporaryRange.id = 'mech-shellless-scroll-range';
  temporaryRange.style.height = '2000px';
  document.body.append(temporaryRange);
  window.__MECH_DELAYED_SHELL__ = {{ shell, parent, next, temporaryRange }};
  document.documentElement.dataset.mechDelayedShell = 'missing';
}}, {{ once: true }});
"""},
            session_id,
        ).get("identifier")
        devtools.call(
            "Emulation.setDeviceMetricsOverride",
            {"width": 1100, "height": 900, "deviceScaleFactor": 1, "mobile": False},
            session_id,
        )
        devtools.call("Page.navigate", {"url": page_url}, session_id)
        wait_for(
            "document.documentElement?.dataset.mechDocumentStatus === 'ready' && "
            "document.documentElement?.dataset.mechDelayedShell === 'missing' && "
            "document.documentElement?.dataset.mechPagePositionRestore === 'waiting-owner' && "
            f"Math.abs(window.scrollY - {shellless_position['y']}) <= 2",
            "an explicit window restore remaining live after reaching its shell-less target",
            timeout=45,
        )
        if delayed_window_script:
            devtools.call(
                "Page.removeScriptToEvaluateOnNewDocument",
                {"identifier": delayed_window_script},
                session_id,
            )
        explicit_window_expected = evaluate_json(f"""
(() => {{
  const held = window.__MECH_DELAYED_SHELL__;
  if (!held) return null;
  held.temporaryRange.remove();
  if (!held.shell.querySelector('#mech-late-layout-spacer')) {{
    const spacer = document.createElement('div');
    spacer.id = 'mech-late-layout-spacer';
    spacer.style.height = '900px';
    (held.shell.querySelector('.content-column') || held.shell).append(spacer);
  }}
  held.parent.insertBefore(
    held.shell,
    held.next?.parentNode === held.parent ? held.next : null,
  );
  let origin = 0;
  for (let element = held.shell; element; element = element.offsetParent) origin += element.offsetTop;
  document.documentElement.dataset.mechDelayedShell = 'restored';
  return {{ y: Math.max(0, {shellless_position['y']} - origin), origin }};
}})()
""")
        if explicit_window_expected is None:
            fail("could not reinsert the explicit-window restore anchor")
        wait_for(
            f"Math.abs(document.querySelector('.content-shell').scrollTop - {explicit_window_expected['y']}) <= 2 && "
            "document.documentElement.dataset.mechPagePositionRestore === 'settling'",
            "a reached window coordinate reprojecting onto the returned shell",
        )
        wait_for(
            "!document.documentElement.dataset.mechPagePositionRestore",
            "the reprojected explicit-window restore reaching a stable mapping",
        )

        legacy_target = 180
        legacy_origin = evaluate_json("""
(() => {
  const shell = document.querySelector('.content-shell');
  let origin = 0;
  for (let element = shell; element; element = element.offsetParent) origin += element.offsetTop;
  return origin;
})()
""")
        legacy_restore_script = devtools.call(
            "Page.addScriptToEvaluateOnNewDocument",
            {"source": f"""
(() => {{
  for (const [key, value] of Object.entries(localStorage)) {{
    if (!key.startsWith('mech:document-layout:v1:')) continue;
    const layout = JSON.parse(value);
    layout.page = {{ x: 0, y: {legacy_origin + legacy_target} }};
    localStorage.setItem(key, JSON.stringify(layout));
  }}
}})()
document.addEventListener('DOMContentLoaded', () => {{
  setTimeout(() => {{
    if (document.getElementById('mech-late-layout-spacer')) return;
    const spacer = document.createElement('div');
    spacer.id = 'mech-late-layout-spacer';
    spacer.style.height = '900px';
    (document.querySelector('.content-column') || document.body).append(spacer);
  }}, 450);
}}, {{ once: true }});
"""},
            session_id,
        ).get("identifier")
        devtools.call("Page.navigate", {"url": page_url}, session_id)
        wait_for(
            "document.documentElement?.dataset.mechDocumentStatus === 'ready' && "
            "/auto|scroll|overlay/.test(getComputedStyle(document.querySelector('.content-shell')).overflowY)",
            "the desktop document reloading for legacy ownerless persistence",
            timeout=45,
        )
        if legacy_restore_script:
            devtools.call(
                "Page.removeScriptToEvaluateOnNewDocument",
                {"identifier": legacy_restore_script},
                session_id,
            )
        wait_for(
            f"Math.abs(document.querySelector('.content-shell').scrollTop - {legacy_target}) <= 2",
            "a legacy ownerless window offset using its defined fallback",
        )
        wait_for(
            "!document.documentElement.dataset.mechPagePositionRestore",
            "the legacy ownerless restore reaching a stable mapping",
        )
    devtools.call(
        "Emulation.setDeviceMetricsOverride",
        {"width": 1680, "height": 900, "deviceScaleFactor": 1, "mobile": False},
        session_id,
    )
    time.sleep(0.15)
    evaluate("document.querySelector('#mech-late-layout-spacer')?.remove()")
    evaluate("document.querySelector('.content-shell')?.scrollTo(0, 0); window.scrollTo(0, 0)")
    time.sleep(0.1)


def assert_real_pointer_capture_cleanup():
    pointer = evaluate_json("""
(() => {
  const root = document.querySelector('.mech-root');
  const handle = root?.querySelector(':scope > [data-mech-console-resizer]:not([data-mech-console-edge-handle])');
  if (!handle) return null;
  const rect = handle.getBoundingClientRect();
  window.__MECH_REAL_POINTER__ = { pointerId: null, got: 0, lost: 0 };
  window.__MECH_REAL_POINTER_HANDLE__ = handle;
  handle.addEventListener('pointerdown', event => {
    window.__MECH_REAL_POINTER__.pointerId = event.pointerId;
  });
  handle.addEventListener('lostpointercapture', () => {
    window.__MECH_REAL_POINTER__.lost += 1;
  });
  handle.addEventListener('gotpointercapture', () => {
    window.__MECH_REAL_POINTER__.got += 1;
  });
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
})()
""")
    if pointer is None:
        fail("could not locate the real pointer-capture resize handle")
    devtools.call(
        "Input.dispatchMouseEvent",
        {
            "type": "mousePressed", "x": pointer["x"], "y": pointer["y"],
            "button": "left", "buttons": 1, "clickCount": 1,
        },
        session_id,
    )
    devtools.call(
        "Input.dispatchMouseEvent",
        {
            "type": "mouseMoved", "x": pointer["x"] + 1, "y": pointer["y"],
            "button": "left", "buttons": 1,
        },
        session_id,
    )
    wait_for(
        "window.__MECH_REAL_POINTER__?.got === 1 && "
        "window.__MECH_REAL_POINTER_HANDLE__?.hasPointerCapture("
        "window.__MECH_REAL_POINTER__?.pointerId)",
        "an active native pointer capture on the resize handle",
    )
    evaluate("""
(() => {
  const handle = window.__MECH_REAL_POINTER_HANDLE__;
  const pointerId = window.__MECH_REAL_POINTER__?.pointerId;
  if (handle && pointerId != null && handle.hasPointerCapture(pointerId)) {
    handle.releasePointerCapture(pointerId);
  }
})()
""")
    release_state = evaluate_json("""
(() => ({
  lost: window.__MECH_REAL_POINTER__?.lost,
  captured: window.__MECH_REAL_POINTER_HANDLE__?.hasPointerCapture(
    window.__MECH_REAL_POINTER__?.pointerId),
  resizing: document.body.classList.contains('is-resizing'),
  axis: document.body.dataset.mechResizeAxis || null,
}))()
""")
    pending_loss = {
        "lost": 0, "captured": False, "resizing": True, "axis": "width",
    }
    delivered_loss = {
        "lost": 1, "captured": False, "resizing": False, "axis": None,
    }
    if release_state not in (pending_loss, delivered_loss):
        fail(
            "releasePointerCapture left native ownership or inconsistent resize state: "
            f"{release_state!r}"
        )
    devtools.call(
        "Input.dispatchMouseEvent",
        {
            "type": "mouseMoved", "x": pointer["x"] + 2, "y": pointer["y"],
            "button": "left", "buttons": 1,
        },
        session_id,
    )
    wait_for(
        "window.__MECH_REAL_POINTER__?.lost === 1 && "
        "!document.body.classList.contains('is-resizing') && "
        "!document.body.hasAttribute('data-mech-resize-axis')",
        "the native lostpointercapture path cleaning the resize session",
    )
    listeners = devtools.call(
        "Runtime.evaluate",
        {
            "expression": "(getEventListeners(window.__MECH_REAL_POINTER_HANDLE__).lostpointercapture || []).length",
            "returnByValue": True,
            "includeCommandLineAPI": True,
        },
        session_id,
    )
    listener_count = listeners.get("result", {}).get("value")
    if listener_count != 1:
        fail(
            "native pointer-capture cleanup retained its production loss listener: "
            f"{listener_count!r}"
        )
    devtools.call(
        "Input.dispatchMouseEvent",
        {
            "type": "mouseReleased", "x": pointer["x"] + 2, "y": pointer["y"],
            "button": "left", "buttons": 0, "clickCount": 1,
        },
        session_id,
    )


def assert_mobile_contract():
    evaluate("""
(() => {
  const root = document.querySelector('[data-mech-repl-host]');
  const pane = document.querySelector('[data-mech-console-pane]');
  if (root) root.style.setProperty('--mech-console-size', '1200px');
  if (pane) pane.style.width = '1200px';
})()
""")
    devtools.call(
        "Emulation.setDeviceMetricsOverride",
        {"width": 800, "height": 900, "deviceScaleFactor": 1, "mobile": False},
        session_id,
    )
    time.sleep(0.3)
    mobile = evaluate_json("""
(() => {
  const root = document.querySelector(".mech-root");
  const visible = (element) => {
    if (!element) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return !element.hidden && style.display !== "none" && style.visibility !== "hidden" &&
      rect.width > 0 && rect.height > 0;
  };
  const content = document.querySelector(".content-shell, .main-content");
  const toggle = document.querySelector("[data-mech-console-edge-handle]");
  return {
    contentVisible: visible(content),
    controlVisible: visible(toggle),
    scrollWidth: document.documentElement.scrollWidth,
    viewportWidth: window.innerWidth,
    rootOpen: root?.dataset.mechConsoleOpen !== "false",
  };
})()
""")
    if not mobile["contentVisible"] or not mobile["controlVisible"]:
        fail(f"mobile content or console control is not reachable: {mobile!r}")
    if mobile["scrollWidth"] > mobile["viewportWidth"] + 1:
        fail(f"mobile page overflows horizontally: {mobile!r}")
    def toggle_mobile_console():
        return evaluate("""
(() => {
  const root = document.querySelector(".mech-root");
  const edge = document.querySelector("[data-mech-console-edge-handle]");
  if (!edge) return "";
  edge.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, clientX: 1, clientY: 1 }));
  window.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, clientX: 1, clientY: 1 }));
  return root?.dataset.mechConsoleOpen || "";
})()
""")

    # A narrow shell may deliberately start closed, but it must pass through
    # both states through an actual user-facing control.
    first_state = toggle_mobile_console()
    expected_first = "false" if mobile["rootOpen"] else "true"
    if first_state != expected_first:
        fail(
            "mobile console did not toggle through its visible control: "
            f"initial={mobile['rootOpen']!r}, next={first_state!r}"
        )
    if first_state == "true" and not evaluate(visible_expression(".console-pane")):
        fail("mobile console was marked open but is not visible")

    second_state = toggle_mobile_console()
    expected_second = "true" if first_state == "false" else "false"
    if second_state != expected_second:
        fail(f"mobile console did not reach its opposite state: {second_state!r}")

    if second_state != "true":
        reopened = toggle_mobile_console()
        if reopened != "true":
            fail(f"mobile console did not reopen through its visible control: {reopened!r}")
    if not evaluate(visible_expression(".console-pane")):
        fail("mobile console was marked open but is not visible")
    pane_geometry = evaluate_json("""
(() => {
  const pane = document.querySelector('[data-mech-console-pane]');
  if (!pane) return null;
  const rect = pane.getBoundingClientRect();
  const rules = [];
  const collect = list => {
    for (const rule of list || []) {
      if (rule.cssRules) collect(rule.cssRules);
      if (rule.selectorText?.includes(':not([data-mech-console-mode="docked"])')) {
        rules.push(rule);
      }
    }
  };
  for (const sheet of document.styleSheets) {
    try { collect(sheet.cssRules); } catch (_error) {}
  }
  return {
    width: rect.width,
    right: rect.right,
    viewportWidth: innerWidth,
    expectedMaximum: Math.min(innerWidth * 0.94, 520),
    dynamicHeightImportant: rules.some(rule =>
      rule.style.getPropertyValue('height') === '100dvh' &&
      rule.style.getPropertyPriority('height') === 'important'),
  };
})()
""")
    if (
        pane_geometry is None or
        not pane_geometry["dynamicHeightImportant"] or
        pane_geometry["width"] > pane_geometry["expectedMaximum"] + 1 or
        pane_geometry["right"] > pane_geometry["viewportWidth"] + 1
    ):
        fail(f"mobile console retained an overflowing desktop width: {pane_geometry!r}")


def assert_terminal_runtime_mutations_retired(probe):
    result = evaluate_json(f"""
(async () => {{
  const {{ WasmDocument }} = await import('/_mech/pkg/mech_wasm.js');
  let pointerCalls = 0;
  WasmDocument.prototype.scenePointerInput = function() {{ pointerCalls += 1; }};
  const displayId = 'terminal-output-{probe}';
  window.dispatchEvent(new CustomEvent('mech:output', {{ detail: {{
    stream: 'stdout', operation: 'create', display_id: displayId,
    content: {{ kind: 'text', data: {{ text: 'must not publish after {probe}' }} }},
  }}}}));
  const wrapper = document.createElement('div');
  wrapper.dataset.mechDisplayId = 'scene-terminal-{probe}';
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.dataset.mechScenePointerSurface = '';
  svg.setAttribute('viewBox', '0 0 100 100');
  svg.style.cssText = 'position:fixed;left:0;top:0;width:100px;height:100px';
  wrapper.append(svg);
  document.body.append(wrapper);
  svg.dispatchEvent(new PointerEvent('pointerdown', {{
    bubbles: true, button: 0, pointerId: 71, clientX: 50, clientY: 50,
  }}));
  window.dispatchEvent(new PointerEvent('pointerup', {{
    bubbles: true, button: 0, pointerId: 71, clientX: 50, clientY: 50,
  }}));
  wrapper.remove();
  const outputTab = document.querySelector('[data-mech-console-tab="output"]');
  const errorsTab = document.querySelector('[data-mech-console-tab="errors"]');
  outputTab?.click();
  const selectedOutput = outputTab?.getAttribute('aria-selected') === 'true';
  errorsTab?.click();
  return {{
    outputAbsent: !document.querySelector(`[data-mech-display-id="${{displayId}}"]`),
    pointerSilent: pointerCalls === 0,
    controlsRemainInteractive:
      selectedOutput && errorsTab?.getAttribute('aria-selected') === 'true',
  }};
}})()
""")
    if result is None or not all(result.values()):
        fail(f"{probe} runtime retained mutation events or lost component controls: {result!r}")


def assert_repl_termination():
    submit(":quit")
    wait_for(
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'terminated' && "
        "document.querySelector('.repl-input')?.disabled === true && "
        "[...document.querySelectorAll('.mech-repl-info')].some((row) => /REPL session terminated/.test(row.textContent))",
        "browser REPL termination disabling further input",
    )
    terminated_state = evaluate_json("""
(() => {
  const input = document.querySelector('.repl-input');
  const transcript = document.querySelector('.mech-repl-transcript');
  if (!input || !transcript) return null;
  const sourceRows = document.querySelectorAll('.mech-repl-source').length;
  const frameRequests = Number(window.__MECH_RAF_REQUESTS__ || 0);
  input.disabled = false;
  input.value = '1 + 1';
  input.dispatchEvent(new KeyboardEvent('keydown', {
    key: 'Enter', bubbles: true, cancelable: true,
  }));
  return {
    sourceRows,
    sourceRowsAfter: document.querySelectorAll('.mech-repl-source').length,
    frameRequests,
    transcriptRows: transcript.children.length,
  };
})()
""")
    if (
        terminated_state is None or
        terminated_state["sourceRowsAfter"] != terminated_state["sourceRows"]
    ):
        fail(f"terminated browser UI accepted a forced follow-up request: {terminated_state!r}")
    time.sleep(0.25)
    frame_requests_after = evaluate_json("Number(window.__MECH_RAF_REQUESTS__ || 0)")
    if frame_requests_after != terminated_state["frameRequests"]:
        fail(
            "the document animation loop continued after :quit: "
            f"before={terminated_state['frameRequests']!r}, after={frame_requests_after!r}"
        )
    assert_terminal_runtime_mutations_retired("stopped")
    direct_exports = evaluate("""
(async () => {
  const { WasmDocument, WasmRepl } = await import('/_mech/pkg/mech_wasm.js');
  const busyRepl = new WasmRepl();
  busyRepl.submit('~counter := 0\\ncounter += 1\\n');
  const sourceBeforeStep = busyRepl.source();
  const started = busyRepl.step(1000n);
  const blockedSubmit = busyRepl.submit('other := 1\\n');
  const sourceAfterSubmit = busyRepl.source();
  const blockedStep = busyRepl.step(2n);
  const sourceAfterStep = busyRepl.source();
  const busy = (response) =>
    response?.pending === true &&
    response?.remaining === 1000 &&
    response?.terminated === false &&
    (response.events || []).some((envelope) =>
      envelope.event?.channel === 'diagnostic' &&
      envelope.event?.event?.code === 'ReplBusy');
  const interrupted = busyRepl.interrupt();
  const accepted = busyRepl.submit('other := 1\\n');
  const sourceAfterInterrupt = busyRepl.source();
  const restarted = busyRepl.step(2n);
  const stopped = busyRepl.shutdown();

  let resetOwnership = null;
  const configuredDocument = Boolean(document.querySelector(
    '[data-mech-var-name="configured-answer"]'
  ));
  if (!configuredDocument) {
    const sourceKey =
      document.querySelector('.mech-root')?.dataset.mechSourceUrlKey ||
      document.documentElement.dataset.mechSourceUrlKey || '';
    const encoded = sourceKey
      ? await (await fetch(`/code/${sourceKey}`)).text()
      : document.querySelector('[data-mech-document-code]')?.textContent?.trim();
    if (!encoded) throw new Error('direct reset smoke could not locate the encoded document');
    const resetDocument = WasmDocument.fromEncoded(encoded);
    const oldStep = resetDocument.replInvoke(':step 1000');
    resetDocument.reset(encoded);
    const newStep = resetDocument.replInvoke(':step 1000');
    let staleStepRejected = false;
    try {
      resetDocument.replContinueStep(1, oldStep.stepRequestId);
    } catch (_) {
      staleStepRejected = true;
    }
    const currentStep = resetDocument.replContinueStep(1, newStep.stepRequestId);
    resetDocument.replInterrupt();
    const oldHost = resetDocument.replInvoke(':docs browser-smoke/old-owner');
    resetDocument.replInterrupt();
    const newHost = resetDocument.replInvoke(':docs browser-smoke/new-owner');
    let staleHostRejected = false;
    try {
      resetDocument.replFinishHostRequest(oldHost.hostRequestId);
    } catch (_) {
      staleHostRejected = true;
    }
    const currentHost = resetDocument.replFinishHostRequest(newHost.hostRequestId);
    resetDocument.stop();
    resetOwnership = {
      distinctStepIds:
        typeof oldStep?.stepRequestId === 'string' &&
        typeof newStep?.stepRequestId === 'string' &&
        oldStep.stepRequestId !== newStep.stepRequestId,
      staleStepRejected,
      currentStepAdvanced:
        currentStep?.pending === true && currentStep?.remaining === 999,
      distinctHostIds:
        typeof oldHost?.hostRequestId === 'string' &&
        typeof newHost?.hostRequestId === 'string' &&
        oldHost.hostRequestId !== newHost.hostRequestId,
      staleHostRejected,
      currentHostFinished: currentHost?.hostPending === false,
    };
  }

  const repl = new WasmRepl();
  const quit = repl.invoke(':quit');
  let staleTerminatedContinuationRejected = false;
  try {
    repl.continueStep(1, 'retired-request');
  } catch (_) {
    staleTerminatedContinuationRejected = true;
  }
  const responses = {
    invoke: repl.invoke('1 + 1'),
    submit: repl.submit('1 + 1'),
    setQuiet: repl.setQuiet(true),
    reset: repl.reset(),
    step: repl.step(1n),
    interrupt: repl.interrupt(),
    clearOutputs: repl.clearOutputs(),
    clearDiagnostics: repl.clearDiagnostics(),
    shutdown: repl.shutdown(),
  };
  const rejected = (response) =>
    response?.terminated === true &&
    (response.events || []).some((envelope) =>
      envelope.event?.channel === 'diagnostic' &&
      envelope.event?.event?.code === 'ReplTerminated');
  return {
    busyState: {
      started: started?.pending === true && started?.remaining === 1000,
      blockedSubmit: busy(blockedSubmit),
      blockedStep: busy(blockedStep),
      sourcePreserved:
        sourceAfterSubmit === sourceBeforeStep &&
        sourceAfterStep === sourceBeforeStep,
      interrupted: interrupted?.pending === false,
      accepted:
        accepted?.pending === false &&
        sourceAfterInterrupt.includes('other := 1'),
      shutdownDuringStep:
        restarted?.pending === true &&
        stopped?.pending === false &&
        stopped?.terminated === true,
    },
    resetOwnership,
    quitTerminated: quit?.terminated === true,
    staleTerminatedContinuationRejected,
    rejected: Object.fromEntries(
      Object.entries(responses).map(([name, response]) => [name, rejected(response)]),
    ),
  };
})()
""")
    busy_state = direct_exports.get("busyState", {}) if direct_exports else {}
    failed_busy_checks = sorted(name for name, value in busy_state.items() if not value)
    if failed_busy_checks or len(busy_state) != 7:
        fail(f"direct WASM REPL bypassed cooperative busy state: {direct_exports!r}")
    reset_ownership = direct_exports.get("resetOwnership") if direct_exports else None
    if label == "configured":
        if reset_ownership is not None:
            fail(f"configured direct reset coverage was unexpectedly detached: {direct_exports!r}")
    else:
        reset_ownership = reset_ownership or {}
        failed_reset_checks = sorted(name for name, value in reset_ownership.items() if not value)
        if failed_reset_checks or len(reset_ownership) != 6:
            fail(f"direct WASM document reused stale operation ownership: {direct_exports!r}")
    if not direct_exports or not direct_exports.get("quitTerminated"):
        fail(f"direct WASM REPL did not enter terminal state: {direct_exports!r}")
    if not direct_exports.get("staleTerminatedContinuationRejected"):
        fail(f"terminated WASM REPL accepted a retired continuation: {direct_exports!r}")
    rejected = direct_exports.get("rejected", {})
    missed = sorted(name for name, value in rejected.items() if not value)
    if missed or len(rejected) != 9:
        fail(f"mutating WASM exports bypassed terminal guard: {direct_exports!r}")


def assert_fatal_error_is_visible():
    devtools.call("Page.navigate", {"url": page_url}, session_id)
    wait_for(
        "document.documentElement?.dataset.mechDocumentStatus === 'ready' && "
        "Boolean(document.querySelector('.repl-input'))",
        "the document reloading for fatal-error visibility coverage",
        timeout=45,
    )
    evaluate("""
(() => {
  if (document.querySelector('.mech-root')?.dataset.mechConsoleOpen !== 'false') {
    document.dispatchEvent(new KeyboardEvent('keydown', {
      key: '`', bubbles: true, cancelable: true,
    }));
  }
})()
""")
    wait_for(
        "document.querySelector('.mech-root')?.dataset.mechConsoleOpen === 'false'",
        "the console closing before a fatal runtime failure",
    )
    evaluate("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  WasmDocument.prototype.frame = function() {
    throw new Error('synthetic fatal frame failure');
  };
})()
""")
    wait_for(
        "document.documentElement?.dataset.mechDocumentStatus === 'error' && "
        "document.querySelector('.mech-root')?.dataset.mechConsoleOpen === 'true' && "
        "document.querySelector('[data-mech-console-tab=errors]')?.getAttribute('aria-selected') === 'true' && "
        "/synthetic fatal frame failure/.test(document.querySelector('[data-mech-errors-panel]')?.textContent || '')",
        "a fatal document failure forcing a visible Errors surface",
        timeout=15,
    )
    assert_terminal_runtime_mutations_retired("failed")


def assert_stop_invalidates_pending_ownership():
    devtools.call("Page.navigate", {"url": page_url}, session_id)
    wait_for(
        "document.documentElement?.dataset.mechDocumentStatus === 'ready' && "
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'ready' && "
        "Boolean(document.querySelector('.repl-input'))",
        "the document reloading for stop ownership coverage",
        timeout=45,
    )
    evaluate("""
(() => {
  const root = document.querySelector('.mech-root');
  if (root?.dataset.mechConsoleOpen !== 'false') {
    document.dispatchEvent(new KeyboardEvent('keydown', {
      key: '`', bubbles: true, cancelable: true,
    }));
  }
})()
""")
    shutdown_inspector = evaluate_json("""
(() => {
  const value = document.querySelector('#mech-smoke-var .mech-var-placeholder');
  window.__MECH_SHUTDOWN_INSPECTOR_ANCHOR__ = value;
  value?.click();
  const popup = document.querySelector('.mech-inline-popup[data-mech-repl-popup]');
  return {
    consoleClosed: document.querySelector('.mech-root')?.dataset.mechConsoleOpen === 'false',
    visible: Boolean(popup?.isConnected),
    focused: document.activeElement === popup?.querySelector('.mech-inline-popup__close'),
  };
})()
""")
    if (
        shutdown_inspector is None or
        not shutdown_inspector["consoleClosed"] or
        not shutdown_inspector["visible"] or
        not shutdown_inspector["focused"]
    ):
        fail(f"could not prepare the closed inspector shutdown regression: {shutdown_inspector!r}")
    submit(":docs browser-smoke/latency")
    wait_for(
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'busy' && "
        "Boolean(document.querySelector('.mech-root')?.dataset.mechHostRequestId) && "
        "window.__MECH_DOCUMENTATION_RELEASES__?.has('latency')",
        "documentation ownership before document stop",
    )
    fullscreen_exit = evaluate_json("""
(async () => {
  const pane = document.querySelector('[data-mech-console-pane]');
  const toggle = document.querySelector('button[data-mech-output-fullscreen]');
  const pending = { native: false, resolveExit: null };
  window.__MECH_DISPOSED_FULLSCREEN_EXIT__ = pending;
  Object.defineProperty(document, 'fullscreenElement', {
    configurable: true,
    get: () => pending.native ? pane : null,
  });
  Object.defineProperty(pane, 'requestFullscreen', {
    configurable: true,
    value: async () => {
      pending.native = true;
      document.dispatchEvent(new Event('fullscreenchange'));
    },
  });
  Object.defineProperty(document, 'exitFullscreen', {
    configurable: true,
    value: () => new Promise(resolve => {
      pending.resolveExit = () => {
        pending.native = false;
        document.dispatchEvent(new Event('fullscreenchange'));
        resolve();
      };
    }),
  });
  toggle?.click();
  await Promise.resolve();
  await Promise.resolve();
  toggle?.click();
  await Promise.resolve();
  return {
    native: pending.native,
    exitPending: typeof pending.resolveExit === 'function',
  };
})()
""")
    if fullscreen_exit != {"native": True, "exitPending": True}:
        fail(f"could not prepare pending fullscreen exit ownership: {fullscreen_exit!r}")
    evaluate("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  window.__MECH_DISPOSED_POINTER_CALLS__ = 0;
  WasmDocument.prototype.scenePointerInput = function() {
    window.__MECH_DISPOSED_POINTER_CALLS__ += 1;
  };
})()
""")
    stopped = evaluate_json("""
(() => {
  const renders = Number(window.__MECH_DOCUMENT_RENDERS__ || 0);
  const pointerCalls = Number(window.__MECH_DISPOSED_POINTER_CALLS__ || 0);
  window.dispatchEvent(new Event('beforeunload'));
  document.documentElement.dataset.mechDocumentStatus = 'error';
  const root = document.querySelector('.mech-root');
  root.dataset.mechDocumentStatus = 'error';
  root.dataset.mechOutputFullscreenActive = 'disposed-sentinel';
  return { renders, pointerCalls };
})()
""")
    evaluate("""
(() => {
  window.__MECH_DOCUMENTATION_RELEASES__.get('latency')?.();
  window.__MECH_DISPOSED_FULLSCREEN_EXIT__?.resolveExit?.();
  window.dispatchEvent(new CustomEvent('mech:output', { detail: {
    stream: 'stdout', operation: 'create', display_id: 'disposed-output-probe',
    content: { kind: 'text', data: { text: 'must not publish after disposal' } },
  }}));
  const wrapper = document.createElement('div');
  wrapper.dataset.mechDisplayId = 'scene-disposal-probe';
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.dataset.mechScenePointerSurface = '';
  svg.setAttribute('viewBox', '0 0 100 100');
  svg.style.cssText = 'position:fixed;left:0;top:0;width:100px;height:100px';
  wrapper.append(svg);
  document.body.append(wrapper);
  svg.dispatchEvent(new PointerEvent('pointerdown', {
    bubbles: true, button: 0, pointerId: 73, clientX: 50, clientY: 50,
  }));
  window.dispatchEvent(new PointerEvent('pointerup', {
    bubbles: true, button: 0, pointerId: 73, clientX: 50, clientY: 50,
  }));
  wrapper.remove();
  return new Promise(resolve => setTimeout(
    () => requestAnimationFrame(() => requestAnimationFrame(resolve)),
    0,
  ));
})()
""")
    stopped_after = evaluate_json("""
(() => ({
  rootStatus: document.querySelector('.mech-root')?.dataset.mechDocumentStatus,
  documentStatus: document.documentElement.dataset.mechDocumentStatus,
  consoleStatus: document.querySelector('.mech-root')?.dataset.mechConsoleStatus,
  hostRequestId: document.querySelector('.mech-root')?.dataset.mechHostRequestId || null,
  renders: Number(window.__MECH_DOCUMENT_RENDERS__ || 0),
  disposedOutput: Boolean(document.querySelector(
    '[data-mech-output-panel] [data-mech-display-id="disposed-output-probe"]'
  )),
  pointerCalls: Number(window.__MECH_DISPOSED_POINTER_CALLS__ || 0),
  appended: Boolean(document.querySelector(
    '[data-mech-documentation-topic="browser-smoke/latency"]'
  )),
  inspectorPresent: Boolean(document.querySelector(
    '.mech-inline-popup[data-mech-repl-popup]'
  )),
  anchorFocused:
    document.activeElement === window.__MECH_SHUTDOWN_INSPECTOR_ANCHOR__,
  fullscreenState:
    document.querySelector('.mech-root')?.dataset.mechOutputFullscreenActive,
}))()
""")
    disposed_api = evaluate_json("""
(async () => {
  const controller = globalThis.MechDocumentController;
  const results = {};
  for (const [name, call] of [
    ['source', () => controller.source()],
    ['renderedValue', () => controller.renderedValue('answer')],
    ['replaceSource', () => controller.replaceSource('answer := 1')],
  ]) {
    try { call(); results[name] = 'accepted'; }
    catch (error) { results[name] = error?.code || error?.name || String(error); }
  }
  try { await controller.invoke('1 + 1'); results.invoke = 'accepted'; }
  catch (error) { results.invoke = error?.code || error?.name || String(error); }
  controller.dispose();
  controller.dispose();
  return results;
})()
""")
    if (
        stopped_after["rootStatus"] != "error" or
        stopped_after["documentStatus"] != "error" or
        stopped_after["consoleStatus"] != "terminated" or
        stopped_after["hostRequestId"] is not None or
        stopped_after["renders"] != stopped["renders"] or
        stopped_after["disposedOutput"] or
        stopped_after["pointerCalls"] != stopped["pointerCalls"] or
        stopped_after["appended"] or
        stopped_after["inspectorPresent"] or
        stopped_after["anchorFocused"] or
        stopped_after["fullscreenState"] != "disposed-sentinel"
    ):
        fail(f"stale async ownership changed a stopped/fatal document: {stopped_after!r}")
    if set(disposed_api.values()) != {"MECH_DOCUMENT_DISPOSED"}:
        fail(f"disposed controller APIs were not terminal and explicit: {disposed_api!r}")


try:
    browser_session = ChromeSession(
        None,
        profile,
        chrome_log,
        flags=[
            "--disable-gpu",
            "--run-all-compositor-stages-before-draw",
            "--hide-scrollbars",
        ],
        window_size=(1680, 900),
    ).start()
    devtools = browser_session.devtools
    session_id = browser_session.session_id
    devtools.call(
        "Emulation.setDeviceMetricsOverride",
        {"width": 1680, "height": 900, "deviceScaleFactor": 1, "mobile": False},
        session_id,
    )
    if label == "custom":
        devtools.call(
            "Page.addScriptToEvaluateOnNewDocument",
            {"source": """
(() => {
  const install = () => {
    const root = document.querySelector('.mech-root');
    if (!root) return;
    if (!document.getElementById('mech-smoke-custom-toggle')) {
      const toggle = document.createElement('button');
      toggle.id = 'mech-smoke-custom-toggle';
      toggle.type = 'button';
      toggle.dataset.mechConsoleToggle = '';
      toggle.textContent = 'Custom console control';
      Object.assign(toggle.style, { position: 'fixed', left: '8px', bottom: '8px', zIndex: '1200' });
      root.prepend(toggle);
    }
  };
  new MutationObserver(install).observe(document, { childList: true, subtree: true });
  document.addEventListener('DOMContentLoaded', install, { once: true });
  install();
})();
"""},
            session_id,
        )
    # Keep real variable placeholders in the DOM before the controller starts.
    # Shipped templates intentionally do not hard-code a document's symbols, so
    # this uses browser automation instead of test-only production behavior to
    # verify the resident root-variable placeholder contract.
    devtools.call(
        "Page.addScriptToEvaluateOnNewDocument",
        {"source": """
(() => {
  const nativeRequestAnimationFrame = window.requestAnimationFrame.bind(window);
  window.__MECH_RAF_REQUESTS__ = 0;
  window.requestAnimationFrame = callback => {
    window.__MECH_RAF_REQUESTS__ += 1;
    return nativeRequestAnimationFrame(callback);
  };
  window.__MECH_DOCUMENT_RENDERS__ = 0;
  window.__MECH_DOCUMENTATION_RELEASES__ = new Map();
  window.addEventListener('mech:document-rendered', () => {
    window.__MECH_DOCUMENT_RENDERS__ += 1;
  });
  const nativeFetch = window.fetch.bind(window);
  window.fetch = (input, init) => {
    const url = typeof input === 'string' ? input : input?.url || String(input);
    if (url.includes('raw.githubusercontent.com/mech-machines/browser-smoke/main/docs/latency.mec')) {
      return new Promise(resolve => {
        window.__MECH_DOCUMENTATION_RELEASES__.set('latency', () => resolve(new Response(
          'Browser smoke documentation evaluates {answer}.',
          { status: 200, headers: { 'content-type': 'text/plain' } },
        )));
      });
    }
    if (url.includes('raw.githubusercontent.com/mech-machines/browser-smoke/main/docs/latency-next.mec')) {
      return new Promise((resolve, reject) => {
        window.__MECH_DOCUMENTATION_RELEASES__.set('latency-next', () => resolve(new Response(
          'Accepted Documentation\\n----------------------\\nAccepted documentation evaluates {answer}.\\n\\n',
          { status: 200, headers: { 'content-type': 'text/plain' } },
        )));
        init?.signal?.addEventListener(
          'abort',
          () => reject(new DOMException('documentation fetch aborted', 'AbortError')),
          { once: true },
        );
      });
    }
    if (url.includes('raw.githubusercontent.com/mech-machines/browser-smoke/main/docs/rejected.mec')) {
      return Promise.resolve(new Response(
        'broken := missing + 1\\nRejected documentation evaluates {answer}.',
        { status: 200, headers: { 'content-type': 'text/plain' } },
      ));
    }
    if (url.includes('raw.githubusercontent.com/mech-machines/browser-smoke/main/docs/recovered.mec')) {
      return Promise.resolve(new Response(
        'Recovered documentation evaluates {answer}.',
        { status: 200, headers: { 'content-type': 'text/plain' } },
      ));
    }
    return nativeFetch(input, init);
  };
  window.addEventListener('mech:console-ready', () => {
    document.documentElement.dataset.mechSmokeConsoleReady = 'true';
  });
  const install = () => {
    const root = document.querySelector('.mech-root');
    if (!root) return;
    if (!document.getElementById('mech-smoke-var')) {
      const marker = document.createElement('span');
      marker.id = 'mech-smoke-var';
      marker.textContent = '{{VAR:answer}}';
      root.append(marker);
    }
    if (!document.getElementById('mech-smoke-unrelated-controls')) {
      const foreign = document.createElement('section');
      foreign.id = 'mech-smoke-unrelated-controls';
      foreign.innerHTML = `
        <div class="resize-handle" aria-label="Unrelated resize handle"></div>
        <button data-mech-like-but-not-owned-control type="button">Unrelated control</button>
        <button class="console-tab foreign-tab-active" data-tab="output" aria-selected="true">Unrelated tab</button>
        <section class="console-panel foreign-panel-active" data-panel="output">Unrelated panel</section>
      `;
      foreign.querySelector('.resize-handle').addEventListener('pointerdown', () => {
        foreign.dataset.mechSmokePointerEvents = String(
          Number(foreign.dataset.mechSmokePointerEvents || '0') + 1,
        );
      });
      foreign.querySelector('[data-mech-like-but-not-owned-control]').addEventListener('click', () => {
        foreign.dataset.mechSmokeButtonEvents = String(
          Number(foreign.dataset.mechSmokeButtonEvents || '0') + 1,
        );
      });
      root.prepend(foreign);
    }
  };
  new MutationObserver(install).observe(document, { childList: true, subtree: true });
  document.addEventListener('DOMContentLoaded', install, { once: true });
  install();
})();
"""},
        session_id,
    )
    devtools.call("Page.navigate", {"url": page_url}, session_id)
    wait_for(
        "document.documentElement?.dataset.mechDocumentStatus === 'ready' && "
        "document.querySelector('.mech-root')?.dataset.mechDocumentStatus === 'ready' && "
        "document.querySelector('.mech-root')?.dataset.mechConsoleStatus === 'ready' && "
        "document.documentElement?.dataset.mechSmokeConsoleReady === 'true' && "
        "Boolean(document.querySelector('.repl-input'))",
        "the rich document controller and console",
        timeout=45,
    )
    wait_for(
        "[...document.querySelectorAll('.mech-block-output')].some((node) => node.textContent.trim())",
        "initial rendered Mech block output",
        timeout=30,
    )
    wait_for(
        "Boolean(document.querySelector('#mech-smoke-var .mech-var-placeholder')?.textContent.trim())",
        "hydration of {{VAR:answer}}",
        timeout=15,
    )
    assert_desktop_contract()
    assert_style_layer_contract()
    assert_desktop_console_controls()
    assert_output_fullscreen_control()
    assert_toc_survives_console_pressure()
    assert_toc_scrollspy_is_continuous_and_hierarchical()
    assert_empty_toc_is_removed_and_content_is_centered()
    assert_fullscreen_accessibility()
    assert_console_tab_isolation()
    assert_right_console_resize_direction()
    assert_layout_persistence()
    assert_real_pointer_capture_cleanup()
    assert_controller_cooperative_lifecycle()
    assert_console_contract()
    assert_mobile_contract()
    assert_repl_termination()
    assert_fatal_error_is_visible()
    assert_stop_invalidates_pending_ownership()
    capture_artifacts()
except Exception as error:
    capture_artifacts()
    print(f"Rich document browser case {label!r} failed: {error}", file=sys.stderr)
    raise
finally:
    stop_browser()
PY
}

run_case() {
  local label="$1"
  shift
  local case_dir="$work_dir/$label"
  mkdir -p "$case_dir"
  local server_log="$case_dir/server.log"
  local port
  port="$(port_for_test)"
  local page_url="http://127.0.0.1:${port}/"

  "$MECH_BIN" --no-config serve \
    --address 127.0.0.1 \
    --port "$port" \
    "$@" >"$server_log" 2>&1 &
  server_pid="$!"

  wait_for_server "$page_url" "$server_log"
  run_browser_case "$label" "$page_url" "$case_dir"

  for route in \
    "/" \
    "/_mech/pkg/mech_wasm.js" \
    "/_mech/pkg/mech_wasm_bg.wasm"; do
    if ! grep -F "GET $route ->" "$server_log" >/dev/null; then
      echo "Browser did not request $route for rich-document case $label" >&2
      sed -n '1,320p' "$server_log" >&2 || true
      return 1
    fi
  done
  if grep -F "GET /mech.mcfg ->" "$server_log" >/dev/null \
    || grep -F "GET /_mech/project.js ->" "$server_log" >/dev/null; then
    echo "Standalone rich document requested a project-only browser asset: $label" >&2
    sed -n '1,320p' "$server_log" >&2 || true
    return 1
  fi
  if [[ "$label" != formatted-* ]] \
    && ! grep -F "GET /code/" "$server_log" >/dev/null; then
    echo "Served rich document did not fetch its source through /code/: $label" >&2
    sed -n '1,320p' "$server_log" >&2 || true
    return 1
  fi

  stop_server
}

run_configured_case() {
  local label="configured"
  local case_dir="$work_dir/$label"
  local server_log="$case_dir/server.log"
  local port
  port="$(port_for_test)"
  local page_url="http://127.0.0.1:${port}/main.mec"

  "$MECH_BIN" serve \
    --address 127.0.0.1 \
    --port "$port" \
    "$case_dir" >"$server_log" 2>&1 &
  server_pid="$!"

  wait_for_server "$page_url" "$server_log"
  run_browser_case "$label" "$page_url" "$case_dir"

  for route in \
    "/main.mec" \
    "/code/main.mec" \
    "/source/main.mec" \
    "/source/support.mec" \
    "/source/package/index.mec" \
    "/mech.mcfg" \
    "/_mech/project-sources.json" \
    "/_mech/pkg/mech_wasm.js" \
    "/_mech/pkg/mech_wasm_bg.wasm"; do
    if ! grep -F "GET $route ->" "$server_log" >/dev/null; then
      echo "Configured rich document did not request $route" >&2
      sed -n '1,320p' "$server_log" >&2 || true
      return 1
    fi
  done

  stop_server
}

prepare_formatted_case \
  formatted-blog \
  "$repo_root/include/blog.html" \
  "$repo_root/include/blog.css"
prepare_formatted_case \
  formatted-docs \
  "$repo_root/include/docs.html" \
  "$repo_root/include/docs.css"
prepare_configured_case

run_case default "$fixture"
run_case custom "$fixture"
run_case blog \
  "$fixture" \
  --shim "$repo_root/include/blog.html" \
  --stylesheet "$repo_root/include/blog.css"
run_case docs \
  "$fixture" \
  --shim "$repo_root/include/docs.html" \
  --stylesheet "$repo_root/include/docs.css"
run_case formatted-blog \
  "$work_dir/formatted-blog/index.html" \
  "$work_dir/formatted-blog/all-slots.mec"
run_case formatted-docs \
  "$work_dir/formatted-docs/index.html" \
  "$work_dir/formatted-docs/all-slots.mec"
run_configured_case
