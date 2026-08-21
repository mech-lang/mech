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
    value = evaluate(f"(async () => JSON.stringify(await ({expression})))()")
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
  const header = document.querySelector('.site-header, #header');
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

  return {
    sourceMarkup,
    sourceText,
    sourceColor,
    headingFont,
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
    resident = evaluate_json("""
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
    if resident != {
        "capability": "resident",
        "label": "Mech resident REPL input",
        "placeholder": "Enter submits · Ctrl+Enter adds a line",
    }:
        fail(f"the standard document console did not advertise resident evaluation: {resident!r}")

    evaluate("document.querySelector('#output-tab')?.click()")
    wait_for(
        "document.querySelector('#output-panel')?.classList.contains('is-active') && "
        "Boolean(document.querySelector('.mech-document-output-entry'))",
        "the document output metadata panel",
    )
    metadata = evaluate_json("""
(() => {
  const panel = document.querySelector('#mech-document-output');
  const entry = panel?.querySelector('.mech-document-output-entry');
  const name = entry?.querySelector('.mech-document-output-name');
  const kind = entry?.querySelector('.mech-output-kind');
  const body = entry?.querySelector('.mech-document-output-html');
  if (!panel || !entry || !name || !kind || !body) return null;
  const probe = document.createElement('span');
  probe.style.color = 'var(--kind-annotation-color, #f09fca)';
  entry.append(probe);
  const expectedKindColor = getComputedStyle(probe).color;
  probe.remove();
  const panelRect = panel.getBoundingClientRect();
  const entryRect = entry.getBoundingClientRect();
  return {
    name: name.textContent.trim(),
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
        not metadata["name"].startswith("output ") or
        not metadata["name"].removeprefix("output ").isdigit() or
        metadata["kind"] != "f64" or
        metadata["kindColor"] != metadata["expectedKindColor"] or
        not metadata["contained"] or
        metadata["bodyOverflow"] not in {"auto", "scroll"}
    ):
        fail(f"document output metadata was not compact, typed, rose, and contained: {metadata!r}")
    evaluate("document.querySelector('#console-tab')?.click()")

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
  input.value = '';
  return result;
})()
""")
    if multiline != {
        "value": ":whos\n answer",
        "caret": 6,
        "transcriptUnchanged": True,
    }:
        fail(f"Ctrl+Enter did not insert a multiline browser REPL draft: {multiline!r}")

    submit("answer + 1")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-result')].some((row) => "
        "/42/.test(row.textContent)) && "
        "document.querySelector('.mech-repl-transcript')?.lastElementChild?.classList.contains('mech-repl-active-prompt')",
        "the document-backed resident console and descending active prompt",
    )
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
        submit(":clear")
        wait_for(
            "[...document.querySelectorAll('.mech-repl-source .repl-code')].some((row) => row.textContent.trim() === ':clear') && "
            "document.querySelector('.repl-input')?.readOnly === false",
            "the configured driver probe returning to the baseline document",
        )
    result_count = evaluate_json(
        "document.querySelectorAll('.mech-repl-result').length"
    )
    submit("answer + 2; -- suppress this browser value")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-source .repl-code')].some((row) => "
        "row.textContent.trim() === 'answer + 2; -- suppress this browser value')",
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
  const panel = document.querySelector('#mech-document-errors');
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
        "document.querySelector('#console-tab')?.getAttribute('aria-selected') === 'true'",
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
    submit(":clear errors")
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
        "[...document.querySelectorAll('.mech-repl-response')].some((row) => /capabilit/i.test(row.textContent))",
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
  const pane = document.querySelector('#mech-console, .console-pane');
  const toggle = document.querySelector('#toggle-repl, [data-mech-console-toggle]');
  const transcript = document.querySelector('.mech-repl-transcript');
  if (!root || !value || !pane || !toggle || !transcript) return null;
  toggle.click();
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
  toggle.click();
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
    evaluate("document.querySelector('#toggle-repl, [data-mech-console-toggle]')?.click()")
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
  document.querySelector('#toggle-repl, [data-mech-console-toggle]')?.click();
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
        "/:load/.test(document.querySelector('.mech-repl-help')?.textContent || '') && "
        "[...document.querySelectorAll('.mech-repl-help .mech-repl-row-muted')].some((row) => /:load/.test(row.textContent)) && "
        "document.querySelectorAll('.mech-repl-help th').length === 2",
        "the shared command registry with unavailable commands muted and no host column",
    )
    help_layout = evaluate_json("""
(() => {
  const table = document.querySelector('.mech-repl-help');
  if (!table) return null;
  return {
    borderless: [...table.querySelectorAll('th, td')].every(cell => {
      const style = getComputedStyle(cell);
      return ['Top', 'Right', 'Bottom', 'Left'].every(side =>
        parseFloat(style[`border${side}Width`]) === 0);
    }),
    unavailableReasonLeaked: /unavailable:/.test(table.textContent),
  };
})()
""")
    if help_layout is None or not help_layout["borderless"] or help_layout["unavailableReasonLeaked"]:
        fail(f"browser help retained table rules or the removed Host column text: {help_layout!r}")
    console_instance = "repl-console" if label == "configured" else "repl"
    console_context = f"console://{console_instance}/output"
    submit(":capabilities")
    wait_for(
        f"[...document.querySelectorAll('.mech-repl-response')].some((row) => row.textContent.includes({json.dumps(console_context)}))",
        "the effective generated console namespace in browser capabilities",
    )
    submit(":clear")
    wait_for(
        "[...document.querySelectorAll('.mech-repl-info')].some((row) => /Resident REPL state cleared/.test(row.textContent)) && "
        "/41/.test(document.querySelector('#mech-smoke-var')?.textContent || '')",
        "the document-backed resident REPL reset",
    )
    submit(f'@out := {console_context}{{:write(line)}}\n@out/line <- "browser-output"\n@out/line <- "continued"')
    wait_for(
        "/browser-output\\s*continued/.test(document.querySelector('[data-mech-output-region=repl]')?.textContent || '') && "
        "document.querySelectorAll('[data-mech-output-region=repl] [data-mech-display-id]').length === 1 && "
        "document.querySelector('#output-tab')?.classList.contains('active')",
        "framed program output targeted at one browser REPL stream surface",
    )
    display_id = evaluate_json(
        "document.querySelector('[data-mech-output-region=repl] [data-mech-display-id]')?.dataset.mechDisplayId || null"
    )
    if not display_id:
        fail("resident program output did not expose its stable display id")
    evaluate("""
(() => {
  const panel = document.querySelector('#mech-document-output');
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
        f"document.querySelector('#output-tab')?.classList.contains('active') && "
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
  const localOutput = root?.querySelector('#mech-document-output');
  const localErrors = root?.querySelector('#mech-document-errors');
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
  const panel = document.querySelector('#mech-document-errors');
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
    evaluate("""
(async () => {
  const { WasmDocument } = await import('/_mech/pkg/mech_wasm.js');
  const original = WasmDocument.prototype.replInvoke;
  WasmDocument.prototype.replInvoke = function(source) {
    const response = original.call(this, source);
    if (source === ':outputs') {
      response.events = response.events || [];
      response.events.push({
        event: {
          channel: 'diagnostic',
          event: {
            id: 'program-browser-smoke',
            owner: 'program',
            severity: 'error',
            phase: 'execute',
            code: 'ProgramBrowserSmoke',
            message: 'persistent program diagnostic',
            source: null,
            notes: [],
            related: [],
          },
        },
      });
      WasmDocument.prototype.replInvoke = original;
    }
    return response;
  };
})()
""")
    submit(":outputs")
    wait_for(
        "Boolean(document.querySelector('[data-mech-error-region=program-diagnostics] "
        ".mech-program-diagnostic[data-mech-diagnostic-id=program-browser-smoke]')) && "
        "document.querySelector('#errors-tab')?.getAttribute('aria-selected') === 'true'",
        "a program-owned diagnostic routing into the Errors pane",
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
    submit(":clear output")
    wait_for(
        "!document.querySelector('[data-mech-output-region=repl] [data-mech-display-id]') && "
        "(document.querySelector('[data-mech-error-region=program]')?.children.length || 0) === 0 && "
        "Boolean(document.querySelector('[data-mech-error-region=program-diagnostics] "
        ".mech-program-diagnostic[data-mech-diagnostic-id=program-browser-smoke]'))",
        "program stream clearing without deleting program diagnostics",
    )
    submit(":clear errors")
    wait_for(
        "!document.querySelector('[data-mech-error-region=program-diagnostics]') && "
        "Boolean(document.querySelector('[data-mech-document-error-smoke=true]'))",
        "diagnostic clearing without deleting document-owned errors",
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
  input.value = '`';
  input.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  return {
    consoleOpen: root.dataset.mechConsoleOpen,
    unchanged: root.dataset.mechConsoleOpen === before,
    value: input.value,
  };
})()
""")
    if typed_backtick is None or not typed_backtick["unchanged"] or typed_backtick["value"] != "`":
        fail(f"typing a backtick in the REPL toggled the document console: {typed_backtick!r}")
    toggled = evaluate_json("""
(() => {
  const root = document.querySelector('.mech-root');
  if (!root) return null;
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  const closed = root.dataset.mechConsoleOpen;
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  return {
    closed,
    reopened: root.dataset.mechConsoleOpen,
    consoleActive: document.querySelector('#console-tab')?.classList.contains('active'),
  };
})()
""")
    if toggled != {"closed": "false", "reopened": "true", "consoleActive": True}:
        fail(f"backtick did not close and reopen the browser REPL: {toggled!r}")


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
    outputActive: document.querySelector('#output-panel')?.classList.contains('is-active'),
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
  const pane = document.querySelector('#mech-console, .console-pane');
  const handle = document.querySelector('#resizer, [data-mech-console-resizer], #edgeHandle');
  const toggle = document.querySelector('#toggle-repl, [data-mech-console-toggle]');
  if (!root || !pane || !handle || !toggle) return null;
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
        fullscreen: pane.classList.contains('is-fullscreen'),
        fallback: pane.dataset.mechFullscreenFallback,
      });
    }
    const finalDelta = deltaXs.at(-1) || 0;
    window.dispatchEvent(new PointerEvent('pointerup', {
      bubbles: true, pointerId, clientX: startX + finalDelta, clientY: startY,
    }));
    return states;
  };
  const before = pane.getBoundingClientRect().width;
  drag(-48);
  const widened = pane.getBoundingClientRect().width;

  drag(widened);
  const collapsed =
    root.dataset.mechConsoleOpen === 'false' &&
    (pane.hidden || pane.classList.contains('is-collapsed'));
  toggle.click();
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
    entered?.fallback === 'true';
  const returned =
    exited?.fullscreen === false &&
    exited?.fallback !== 'true' &&
    exited?.open === 'true';
  return { before, widened, collapsed, reopened, fullscreen, returned };
})()
""")
    if (
        state is None or
        state["widened"] <= state["before"] or
        not state["collapsed"] or
        not state["reopened"] or
        not state["fullscreen"] or
        not state["returned"]
    ):
        fail(f"right-console drag thresholds did not widen, collapse, fullscreen, and return: {state!r}")


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
    pane_geometry = evaluate_json("""
(() => {
  const pane = document.querySelector('[data-mech-console-pane]');
  if (!pane) return null;
  const rect = pane.getBoundingClientRect();
  return {
    width: rect.width,
    right: rect.right,
    viewportWidth: innerWidth,
    expectedMaximum: Math.min(innerWidth * 0.94, 520),
  };
})()
""")
    if (
        pane_geometry is None or
        pane_geometry["width"] > pane_geometry["expectedMaximum"] + 1 or
        pane_geometry["right"] > pane_geometry["viewportWidth"] + 1
    ):
        fail(f"mobile console retained an overflowing desktop width: {pane_geometry!r}")


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
    document.querySelector('#toggle-repl, [data-mech-console-toggle]')?.click();
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
    stopped = evaluate_json("""
(() => {
  const renders = Number(window.__MECH_DOCUMENT_RENDERS__ || 0);
  window.dispatchEvent(new Event('beforeunload'));
  document.documentElement.dataset.mechDocumentStatus = 'error';
  document.querySelector('.mech-root').dataset.mechDocumentStatus = 'error';
  return { renders };
})()
""")
    evaluate("""
(() => {
  window.__MECH_DOCUMENTATION_RELEASES__.get('latency')?.();
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
  appended: Boolean(document.querySelector(
    '[data-mech-documentation-topic="browser-smoke/latency"]'
  )),
  inspectorPresent: Boolean(document.querySelector(
    '.mech-inline-popup[data-mech-repl-popup]'
  )),
  anchorFocused:
    document.activeElement === window.__MECH_SHUTDOWN_INSPECTOR_ANCHOR__,
}))()
""")
    if (
        stopped_after["rootStatus"] != "error" or
        stopped_after["documentStatus"] != "error" or
        stopped_after["consoleStatus"] != "terminated" or
        stopped_after["hostRequestId"] is not None or
        stopped_after["renders"] != stopped["renders"] or
        stopped_after["appended"] or
        stopped_after["inspectorPresent"] or
        stopped_after["anchorFocused"]
    ):
        fail(f"stale async ownership changed a stopped/fatal document: {stopped_after!r}")


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
    assert_fullscreen_accessibility()
    assert_console_tab_isolation()
    assert_right_console_resize_direction()
    assert_console_contract()
    assert_mobile_contract()
    assert_repl_termination()
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
