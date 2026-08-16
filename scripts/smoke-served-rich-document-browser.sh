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

if [[ -n "${CHROME_BIN:-}" ]]; then
  chrome_bin="$CHROME_BIN"
elif command -v google-chrome >/dev/null 2>&1; then
  chrome_bin="$(command -v google-chrome)"
elif [[ -x "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" ]]; then
  chrome_bin="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
else
  echo "Google Chrome was not found" >&2
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
    "$chrome_bin" \
    "$page_url" \
    "$chrome_profile" \
    "$dom_file" \
    "$screenshot_file" \
    "$chrome_log" \
    "$label" <<'PY'
import base64
import json
import os
from pathlib import Path
import secrets
import signal
import socket
import struct
import subprocess
import sys
import time
import urllib.request


chrome, page_url, profile, dom_path, screenshot_path, chrome_log, label = sys.argv[1:]


def fail(message):
    raise AssertionError(message)


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def json_url(url):
    with urllib.request.urlopen(url, timeout=2) as response:
        return json.loads(response.read().decode("utf-8"))


class WebSocket:
    def __init__(self, url):
        if not url.startswith("ws://"):
            fail(f"unsupported DevTools websocket URL: {url}")
        authority, path = url[5:].split("/", 1)
        host, port = authority.rsplit(":", 1)
        self.socket = socket.create_connection((host, int(port)), timeout=10)
        self.socket.settimeout(20)
        nonce = base64.b64encode(secrets.token_bytes(16)).decode("ascii")
        request = (
            f"GET /{path} HTTP/1.1\r\n"
            f"Host: {authority}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {nonce}\r\n"
            "Sec-WebSocket-Version: 13\r\n\r\n"
        ).encode("ascii")
        self.socket.sendall(request)
        response = self._read_headers()
        if not response.startswith(b"HTTP/1.1 101"):
            fail(f"DevTools websocket handshake failed: {response!r}")

    def _read_headers(self):
        data = bytearray()
        while b"\r\n\r\n" not in data:
            chunk = self.socket.recv(1)
            if not chunk:
                fail("DevTools websocket closed during handshake")
            data.extend(chunk)
            if len(data) > 65536:
                fail("DevTools websocket sent oversized response headers")
        return bytes(data)

    def _read_exact(self, count):
        chunks = bytearray()
        while len(chunks) < count:
            chunk = self.socket.recv(count - len(chunks))
            if not chunk:
                fail("DevTools websocket closed unexpectedly")
            chunks.extend(chunk)
        return bytes(chunks)

    def send(self, payload, opcode=1):
        data = payload.encode("utf-8") if isinstance(payload, str) else payload
        header = bytearray([0x80 | opcode])
        length = len(data)
        if length < 126:
            header.append(0x80 | length)
        elif length <= 0xFFFF:
            header.append(0x80 | 126)
            header.extend(struct.pack("!H", length))
        else:
            header.append(0x80 | 127)
            header.extend(struct.pack("!Q", length))
        mask = os.urandom(4)
        header.extend(mask)
        masked = bytes(byte ^ mask[index % 4] for index, byte in enumerate(data))
        self.socket.sendall(bytes(header) + masked)

    def receive(self):
        first, second = self._read_exact(2)
        opcode = first & 0x0F
        masked = bool(second & 0x80)
        length = second & 0x7F
        if length == 126:
            length = struct.unpack("!H", self._read_exact(2))[0]
        elif length == 127:
            length = struct.unpack("!Q", self._read_exact(8))[0]
        mask = self._read_exact(4) if masked else None
        payload = self._read_exact(length)
        if mask:
            payload = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
        if opcode == 0x9:
            self.send(payload, opcode=0xA)
            return self.receive()
        if opcode == 0x8:
            fail("DevTools websocket closed")
        if opcode != 0x1:
            return self.receive()
        return json.loads(payload.decode("utf-8"))

    def close(self):
        try:
            self.send(b"", opcode=0x8)
        except OSError:
            pass
        self.socket.close()


class NavigationContextPending(Exception):
    pass


def navigation_context_pending(error):
    message = str(error.get("message", "")).lower()
    return any(
        marker in message
        for marker in (
            "cannot find default execution context",
            "cannot find context with specified id",
            "execution context was destroyed",
            "inspected target navigated or closed",
        )
    )


class DevTools:
    def __init__(self, websocket):
        self.websocket = websocket
        self.next_id = 1

    def call(self, method, params=None, session_id=None):
        request_id = self.next_id
        self.next_id += 1
        request = {"id": request_id, "method": method}
        if params is not None:
            request["params"] = params
        if session_id is not None:
            request["sessionId"] = session_id
        self.websocket.send(json.dumps(request))
        while True:
            response = self.websocket.receive()
            if response.get("id") != request_id:
                continue
            if "error" in response:
                if method == "Runtime.evaluate" and navigation_context_pending(response["error"]):
                    raise NavigationContextPending(response["error"])
                fail(f"DevTools {method} failed: {response['error']!r}")
            return response.get("result", {})


def visible_expression(selector):
    return f"""
(() => {{
  const element = document.querySelector({json.dumps(selector)});
  if (!element) return false;
  const rect = element.getBoundingClientRect();
  const style = getComputedStyle(element);
  return !element.hidden && style.display !== "none" && style.visibility !== "hidden" &&
    rect.width > 0 && rect.height > 0;
}})()
"""


process = None
websocket = None
devtools = None
session_id = None


def evaluate(expression):
    result = devtools.call(
        "Runtime.evaluate",
        {
            "expression": expression,
            "returnByValue": True,
            "awaitPromise": True,
            "userGesture": True,
        },
        session_id,
    )
    if "exceptionDetails" in result:
        fail(f"browser expression failed: {result['exceptionDetails']!r}")
    remote = result.get("result", {})
    if remote.get("type") == "undefined":
        return None
    return remote.get("value")


def evaluate_json(expression):
    value = evaluate(f"JSON.stringify(({expression}))")
    if not isinstance(value, str):
        fail(f"browser expression did not produce JSON: {expression}")
    return json.loads(value)


def wait_for(expression, description, timeout=35):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        try:
            last = evaluate(expression)
        except NavigationContextPending:
            last = None
        if last:
            return last
        time.sleep(0.08)
    fail(f"timed out waiting for {description}; last value was {last!r}")


def capture_artifacts():
    if devtools is None or session_id is None:
        return
    try:
        html = evaluate("document.documentElement.outerHTML")
        if isinstance(html, str):
            Path(dom_path).write_text(html)
    except Exception as error:  # Diagnostics must not hide the original error.
        Path(dom_path).write_text(f"Could not collect DOM: {error!r}\n")
    try:
        image = devtools.call(
            "Page.captureScreenshot",
            {"format": "png", "captureBeyondViewport": False},
            session_id,
        ).get("data")
        if image:
            Path(screenshot_path).write_bytes(base64.b64decode(image))
    except Exception as error:  # Diagnostics must not hide the original error.
        Path(screenshot_path + ".error").write_text(
            f"Could not collect screenshot: {error!r}\n",
        )


def stop_browser():
    global process
    if websocket is not None:
        websocket.close()
    if process is None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=10)
    except (OSError, subprocess.TimeoutExpired):
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except OSError:
            pass
        process.wait()
    process = None


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
  const header = first(["#header", ".site-header"]);
  const navigation = first(["#nav", ".top-nav"]);
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
    header: rectangle(header),
    navigationVisible: Boolean(navigation && navigation.querySelectorAll("a").length && visible("#nav, .top-nav")),
    titleVisible: Boolean(title && visible("#document-title, .mech-document-content h1, .main-content h1")),
    contentVisible: Boolean(content && visible("#left-pane, .content-shell, .main-content")),
    console: rectangle(console),
    consoleVisible: visible(".console-pane"),
    tabs: console?.querySelectorAll(".console-tab").length || 0,
    consoleTabActive: Boolean(document.querySelector("#console-tab.console-tab.active[aria-selected='true']")),
    consoleToggleVisible: visible("#toggle-repl, [data-mech-console-toggle]"),
    promptVisible: visible(".repl-prompt"),
    inputVisible: visible(".repl-input"),
    outputIsPlaceholder: /under construction/i.test(document.querySelector("#mech-document-output")?.textContent || ""),
    errorsIsPlaceholder: /under construction/i.test(document.querySelector("#mech-document-errors")?.textContent || ""),
    resizerVisible: visible("#resizer"),
    fullscreenVisible: visible("#consoleFullscreenToggle"),
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
    if not desktop["header"] or desktop["header"]["height"] < 40:
        fail(f"document header is missing or too short: {desktop!r}")
    for name in (
        "navigationVisible", "titleVisible", "contentVisible", "consoleVisible",
        "consoleTabActive", "promptVisible", "inputVisible", "resizerVisible",
        "consoleToggleVisible", "fullscreenVisible", "citationsVisible", "footnotesVisible", "blockOutput",
        "inlineOutput", "variableHydrated",
    ):
        if not desktop[name]:
            fail(f"desktop rich-document contract failed for {name}: {desktop!r}")
    if desktop["tabs"] != 3:
        fail(f"expected exactly three console tabs: {desktop!r}")
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
    if label in ("blog", "docs", "formatted-blog", "formatted-docs"):
        if not evaluate(visible_expression(".footer")):
            fail("rich blog/docs shell did not render a visible footer")
        pagination = evaluate_json("""
(() => ({
  previous: Boolean(document.querySelector(".post-pagination-prev")),
  next: Boolean(document.querySelector(".post-pagination-next")),
}))()
""")
        if not pagination["previous"] or not pagination["next"]:
            fail(f"rich shell did not render previous/next controls: {pagination!r}")


def assert_desktop_console_controls():
    state = evaluate_json("""
(() => {
  const root = document.querySelector(".mech-root");
  const pane = document.querySelector(".console-pane");
  const toggle = document.querySelector("#toggle-repl, [data-mech-console-toggle]");
  return {
    rootOpen: root?.dataset.mechConsoleOpen,
    paneHidden: pane?.hidden,
    expanded: toggle?.getAttribute("aria-expanded"),
  };
})()
""")
    if state["rootOpen"] != "true" or state["paneHidden"] or state["expanded"] != "true":
        fail(f"desktop console did not begin in an accessible open state: {state!r}")

    evaluate("document.querySelector('#toggle-repl, [data-mech-console-toggle]')?.click()")
    wait_for(
        "document.querySelector('.mech-root')?.dataset.mechConsoleOpen === 'false' && "
        "document.querySelector('.console-pane')?.hidden === true && "
        "document.querySelector('#toggle-repl, [data-mech-console-toggle]')?.getAttribute('aria-expanded') === 'false'",
        "the desktop console closing through its accessible toggle",
    )

    evaluate("document.querySelector('#toggle-repl, [data-mech-console-toggle]')?.click()")
    wait_for(
        "document.querySelector('.mech-root')?.dataset.mechConsoleOpen === 'true' && "
        "document.querySelector('.console-pane')?.hidden === false && "
        "document.querySelector('#toggle-repl, [data-mech-console-toggle]')?.getAttribute('aria-expanded') === 'true'",
        "the desktop console reopening through its accessible toggle",
    )


def assert_fullscreen_accessibility():
    initial = evaluate_json("""
(() => {
  const toggle = document.querySelector("#consoleFullscreenToggle, [data-mech-console-fullscreen]");
  return {
    pressed: toggle?.getAttribute("aria-pressed"),
    label: toggle?.getAttribute("aria-label"),
  };
})()
""")
    if initial["pressed"] != "false" or initial["label"] != "Enter fullscreen":
        fail(f"fullscreen control did not begin with a collapsed accessible state: {initial!r}")

    evaluate("document.querySelector('#consoleFullscreenToggle, [data-mech-console-fullscreen]')?.click()")
    wait_for(
        "document.querySelector('#consoleFullscreenToggle, [data-mech-console-fullscreen]')?.getAttribute('aria-pressed') === 'true' && "
        "document.querySelector('#consoleFullscreenToggle, [data-mech-console-fullscreen]')?.getAttribute('aria-label') === 'Exit fullscreen'",
        "the fullscreen control entering an accessible active state",
    )

    evaluate("document.querySelector('#consoleFullscreenToggle, [data-mech-console-fullscreen]')?.click()")
    wait_for(
        "document.querySelector('#consoleFullscreenToggle, [data-mech-console-fullscreen]')?.getAttribute('aria-pressed') === 'false' && "
        "document.querySelector('#consoleFullscreenToggle, [data-mech-console-fullscreen]')?.getAttribute('aria-label') === 'Enter fullscreen'",
        "the fullscreen control restoring its accessible inactive state",
    )


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


def assert_console_contract():
    command_only = evaluate_json("""
(() => {
  const input = document.querySelector('.repl-input');
  if (!input) return null;
  return {
    capability: input.dataset.mechInteractiveEvaluation,
    label: input.getAttribute('aria-label'),
    placeholder: input.getAttribute('placeholder'),
  };
})()
""")
    if command_only != {
        "capability": "unavailable",
        "label": "Mech document command input",
        "placeholder": "Document commands only (:help)",
    }:
        fail(f"the standard document console advertised developer evaluation: {command_only!r}")

    if label == "configured":
        # This value is deliberately distinct from the mutable `answer`
        # fixture. It proves the configured page resolved its sibling module
        # while the document bootstrap independently proves that its granted
        # live clock host can be validated and installed.
        submit(":whos configured-answer")
        wait_for(
            "[...document.querySelectorAll('.mech-repl-symbols')].some((table) => "
            "/configured-answer/.test(table.textContent) && /41/.test(table.textContent))",
            "the configured document's imported value",
        )
    submit("answer + 1")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-error')].some((row) => "
        "/interactive source evaluation is unavailable/.test(row.textContent)) && "
        "document.querySelectorAll('.mech-repl-result').length === 0",
        "the standard console rejecting developer source evaluation",
    )
    submit(":whos answer")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-symbols')].some((table) => /answer/.test(table.textContent))",
        "the answer row from :whos",
    )
    submit(":help")
    wait_for("Boolean(document.querySelector('.mech-repl-help'))", "the browser console help table")
    submit(":clear")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-info')].some((row) => /Document reset/.test(row.textContent)) && "
        "/41/.test(document.querySelector('#mech-smoke-var')?.textContent || '')",
        "the reset browser document state",
    )
    evaluate("document.querySelector('#output-tab')?.click()")
    wait_for(
        "document.querySelector('#output-tab')?.classList.contains('active') && "
        "document.querySelector('#output-panel')?.classList.contains('is-active') && "
        "document.querySelector('#mech-document-output')?.textContent.trim().length > 0",
        "the rendered Output console tab",
    )
    submit(":clc")
    wait_for(
        "document.querySelector('.mech-repl-transcript')?.children.length === 0",
        "the cleared browser console transcript",
    )


def assert_console_tab_isolation():
    state = evaluate_json("""
(() => {
  const root = document.querySelector('.mech-root');
  const outputTab = document.querySelector('#output-tab');
  if (!root || !outputTab) return null;

  const foreign = document.querySelector('#mech-smoke-unrelated-controls');
  if (!foreign) return null;
  const foreignTab = foreign.querySelector('[data-tab="output"]');
  const foreignPanel = foreign.querySelector('[data-panel="output"]');
  const foreignResize = foreign.querySelector('.resize-handle');
  const foreignButton = foreign.querySelector('[data-mech-like-but-not-owned-control]');
  const pane = document.querySelector('#mech-console, .console-pane');
  if (!foreignTab || !foreignPanel || !foreignResize || !foreignButton || !pane) return null;

  outputTab.click();
  const before = {
    consoleSize: root.style.getPropertyValue('--mech-console-size'),
    consoleOpen: root.dataset.mechConsoleOpen,
    outputAria: outputTab.getAttribute('aria-selected'),
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
    outputActive: document.querySelector('#output-panel')?.classList.contains('is-active'),
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
        not state["consoleSizeUnchanged"] or
        not state["consoleOpenUnchanged"] or
        not state["consoleTabAriaUnchanged"]
    ):
        fail(f"document console captured an unrelated custom-shim control: {state!r}")


def assert_right_console_resize_direction():
    state = evaluate_json("""
(() => {
  const pane = document.querySelector('#mech-console, .console-pane');
  const handle = document.querySelector('#resizer, [data-mech-console-resizer], #edgeHandle');
  if (!pane || !handle) return null;
  const rect = handle.getBoundingClientRect();
  const startX = rect.left + Math.max(1, rect.width / 2);
  const startY = rect.top + Math.max(1, rect.height / 2);
  const before = pane.getBoundingClientRect().width;
  handle.dispatchEvent(new PointerEvent('pointerdown', {
    bubbles: true, cancelable: true, pointerId: 73, clientX: startX, clientY: startY,
  }));
  window.dispatchEvent(new PointerEvent('pointermove', {
    bubbles: true, pointerId: 73, clientX: startX - 48, clientY: startY,
  }));
  window.dispatchEvent(new PointerEvent('pointerup', {
    bubbles: true, pointerId: 73, clientX: startX - 48, clientY: startY,
  }));
  return { before, after: pane.getBoundingClientRect().width };
})()
""")
    if state is None or state["after"] <= state["before"]:
        fail(f"dragging the right console's left handle left did not widen it: {state!r}")


def assert_mobile_contract():
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
  const content = document.querySelector("#left-pane, .content-shell, .main-content");
  const toggle = document.querySelector("#toggle-repl, #edgeHandle, [data-mech-console-toggle]");
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
  const toggle = document.querySelector("#toggle-repl, [data-mech-console-toggle]");
  if (toggle) {
    toggle.click();
    return root?.dataset.mechConsoleOpen || "";
  }
  const edge = document.querySelector("#edgeHandle");
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


try:
    debugger_port = free_port()
    Path(profile).mkdir(parents=True, exist_ok=True)
    with Path(chrome_log).open("wb") as stderr:
        process = subprocess.Popen(
            [
                chrome,
                "--headless=new",
                "--no-sandbox",
                "--disable-gpu",
                "--disable-dev-shm-usage",
                "--run-all-compositor-stages-before-draw",
                "--hide-scrollbars",
                "--remote-allow-origins=*",
                "--window-size=1680,900",
                f"--remote-debugging-port={debugger_port}",
                f"--user-data-dir={profile}",
                "about:blank",
            ],
            stdout=subprocess.DEVNULL,
            stderr=stderr,
            start_new_session=True,
        )
    deadline = time.monotonic() + 25
    version = None
    while time.monotonic() < deadline:
        try:
            version = json_url(f"http://127.0.0.1:{debugger_port}/json/version")
            break
        except OSError:
            time.sleep(0.1)
    if not version:
        fail("headless Chrome did not expose its DevTools endpoint")

    websocket = WebSocket(version["webSocketDebuggerUrl"])
    devtools = DevTools(websocket)
    target = devtools.call("Target.createTarget", {"url": "about:blank"})["targetId"]
    session_id = devtools.call(
        "Target.attachToTarget", {"targetId": target, "flatten": True},
    )["sessionId"]
    devtools.call("Page.enable", session_id=session_id)
    devtools.call("Runtime.enable", session_id=session_id)
    devtools.call(
        "Emulation.setDeviceMetricsOverride",
        {"width": 1680, "height": 900, "deviceScaleFactor": 1, "mobile": False},
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
    assert_desktop_console_controls()
    assert_fullscreen_accessibility()
    assert_console_tab_isolation()
    assert_right_console_resize_direction()
    assert_console_contract()
    assert_mobile_contract()
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
