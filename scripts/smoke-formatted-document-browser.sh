#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  target_dir="$CARGO_TARGET_DIR"
  [[ "$target_dir" = /* ]] || target_dir="$repo_root/$target_dir"
else
  target_dir="$repo_root/target"
fi

MECH_BIN="${MECH_BIN:-$target_dir/debug/mech}"
[[ -x "$MECH_BIN" ]] || { echo "Mech binary is not executable: $MECH_BIN" >&2; exit 1; }

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

work_dir="$(mktemp -d "$target_dir/formatted-document.XXXXXX")"
server_pid=""

cleanup() {
  local status="$?"
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  if [[ "$status" -ne 0 ]]; then
    echo "Formatted document browser artifacts retained at: $work_dir" >&2
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

mkdir -p \
  "$work_dir/project/vendor" \
  "$work_dir/project/package" \
  "$work_dir/shared"
cat > "$work_dir/project/main.mec" <<'MEC'
+> ./café.mec
+> ./extdep
+> ./package
+> ./vendor/support.mec
+> ./vendor/percent.mec
+> ./rate%.mec
{included.mec}
~answer := 0
answer += café/value + extdep/value + package/value + support/value + percent/value + included-value + nested-included-value
MEC
cat > "$work_dir/project/café.mec" <<'MEC'
value := 2
<+ value
MEC
cat > "$work_dir/project/extdep.mec" <<'MEC'
value := 3
<+ value
MEC
cat > "$work_dir/project/package/index.mec" <<'MEC'
value := 5
<+ value
MEC
cat > "$work_dir/project/included.mec" <<'MEC'
{nested-included.mec}
included-value := 13
MEC
cat > "$work_dir/project/nested-included.mec" <<'MEC'
nested-included-value := 17
MEC
cat > "$work_dir/project/rate%.mec" <<'MEC'
value := 29
literal-percent-pass! := value == 29
<+ value
MEC
cat > "$work_dir/shared/support.mec" <<'MEC'
+> ./nested.mec
value := nested/value
<+ value
MEC
cat > "$work_dir/shared/nested.mec" <<'MEC'
value := 7
<+ value
MEC
cat > "$work_dir/shared/rate%.mec" <<'MEC'
value := 11
<+ value
MEC
ln -s ../../shared/support.mec "$work_dir/project/vendor/support.mec"
ln -s '../../shared/rate%.mec' "$work_dir/project/vendor/percent.mec"
output_dir="$work_dir/static"
format_log="$work_dir/format.log"
if ! "$MECH_BIN" --no-config format "$work_dir/project/main.mec" --html --out "$output_dir" >"$format_log" 2>&1; then
  sed -n '1,240p' "$format_log" >&2 || true
  exit 1
fi

page_file="$(find "$output_dir" -name main.html -type f -print -quit)"
[[ -n "$page_file" ]] || { echo "formatter did not emit main.html" >&2; exit 1; }
[[ -s "$output_dir/_mech/pkg/mech_wasm.js" ]] || { echo "formatter did not emit mech_wasm.js" >&2; exit 1; }
[[ -s "$output_dir/_mech/pkg/mech_wasm_bg.wasm" ]] || {
  echo "formatter did not emit mech_wasm_bg.wasm" >&2
  exit 1
}
canonical_support="$(cd "$work_dir/shared" && pwd -P)/support.mec"
canonical_percent="$(cd "$work_dir/shared" && pwd -P)/rate%.mec"
if grep -F "$work_dir" "$page_file" >/dev/null \
  || grep -F 'file://' "$page_file" >/dev/null \
  || grep -F "$canonical_support" "$page_file" >/dev/null \
  || grep -F "$canonical_percent" "$page_file" >/dev/null; then
  echo "standalone source bundle leaked a filesystem location" >&2
  exit 1
fi

port="$(port_for_test)"
server_log="$work_dir/static-server.log"
PYTHONUNBUFFERED=1 python3 -m http.server "$port" --bind 127.0.0.1 --directory "$output_dir" >"$server_log" 2>&1 &
server_pid="$!"
page_relative="${page_file#"$output_dir"/}"
page_url="http://127.0.0.1:${port}/${page_relative}"
for _ in $(seq 1 150); do
  if curl --fail --silent --output /dev/null "$page_url" 2>/dev/null; then
    break
  fi
  sleep 0.1
done
curl --fail --silent --output /dev/null "$page_url" || {
  sed -n '1,240p' "$server_log" >&2 || true
  exit 1
}

python3 - "$chrome_bin" "$page_url" "$work_dir/chrome-profile" "$work_dir/chrome.log" <<'PY'
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

chrome, page_url, profile, chrome_log = sys.argv[1:]

def fail(message):
    raise AssertionError(message)

def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]

class WebSocket:
    def __init__(self, url):
        authority, path = url.removeprefix("ws://").split("/", 1)
        host, port = authority.rsplit(":", 1)
        self.socket = socket.create_connection((host, int(port)), timeout=10)
        self.socket.settimeout(20)
        nonce = base64.b64encode(secrets.token_bytes(16)).decode("ascii")
        request = (
            f"GET /{path} HTTP/1.1\r\nHost: {authority}\r\nUpgrade: websocket\r\n"
            f"Connection: Upgrade\r\nSec-WebSocket-Key: {nonce}\r\n"
            "Sec-WebSocket-Version: 13\r\n\r\n"
        ).encode("ascii")
        self.socket.sendall(request)
        response = bytearray()
        while b"\r\n\r\n" not in response:
            response.extend(self.socket.recv(1))
        if not response.startswith(b"HTTP/1.1 101"):
            fail(f"DevTools websocket handshake failed: {bytes(response)!r}")

    def read_exact(self, count):
        out = bytearray()
        while len(out) < count:
            part = self.socket.recv(count - len(out))
            if not part:
                fail("DevTools websocket closed unexpectedly")
            out.extend(part)
        return bytes(out)

    def send(self, value, opcode=1):
        payload = value.encode("utf-8") if isinstance(value, str) else value
        header = bytearray([0x80 | opcode])
        if len(payload) < 126:
            header.append(0x80 | len(payload))
        elif len(payload) <= 0xffff:
            header.extend([0x80 | 126, *struct.pack("!H", len(payload))])
        else:
            header.extend([0x80 | 127, *struct.pack("!Q", len(payload))])
        mask = os.urandom(4)
        header.extend(mask)
        header.extend(byte ^ mask[index % 4] for index, byte in enumerate(payload))
        self.socket.sendall(header)

    def receive(self):
        first, second = self.read_exact(2)
        opcode, length = first & 0x0f, second & 0x7f
        if length == 126:
            length = struct.unpack("!H", self.read_exact(2))[0]
        elif length == 127:
            length = struct.unpack("!Q", self.read_exact(8))[0]
        mask = self.read_exact(4) if second & 0x80 else None
        payload = self.read_exact(length)
        if mask:
            payload = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
        if opcode == 9:
            self.send(payload, opcode=10)
            return self.receive()
        if opcode != 1:
            return self.receive()
        return json.loads(payload.decode("utf-8"))

    def close(self):
        self.socket.close()

class NavigationContextPending(Exception):
    pass

class DevTools:
    def __init__(self, websocket):
        self.websocket, self.next_id = websocket, 1

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
                message = str(response["error"].get("message", "")).lower()
                if method == "Runtime.evaluate" and any(marker in message for marker in (
                    "cannot find default execution context",
                    "cannot find context with specified id",
                    "execution context was destroyed",
                    "inspected target navigated or closed",
                )):
                    raise NavigationContextPending(response["error"])
                fail(f"DevTools {method} failed: {response['error']!r}")
            return response.get("result", {})

process = None
websocket = None
try:
    debug_port = free_port()
    Path(profile).mkdir(parents=True, exist_ok=True)
    with Path(chrome_log).open("wb") as stderr:
        process = subprocess.Popen(
            [chrome, "--headless=new", "--no-sandbox", "--disable-gpu",
             "--remote-allow-origins=*", f"--remote-debugging-port={debug_port}",
             f"--user-data-dir={profile}", "about:blank"],
            stdout=subprocess.DEVNULL, stderr=stderr, start_new_session=True,
        )
    deadline = time.monotonic() + 25
    version = None
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{debug_port}/json/version", timeout=2) as response:
                version = json.loads(response.read())
                break
        except OSError:
            time.sleep(0.1)
    if version is None:
        fail("headless Chrome did not expose its DevTools endpoint")

    websocket = WebSocket(version["webSocketDebuggerUrl"])
    devtools = DevTools(websocket)
    target = devtools.call("Target.createTarget", {"url": "about:blank"})["targetId"]
    session = devtools.call("Target.attachToTarget", {"targetId": target, "flatten": True})["sessionId"]
    devtools.call("Page.enable", session_id=session)
    devtools.call("Runtime.enable", session_id=session)
    devtools.call("Page.navigate", {"url": page_url}, session)

    def evaluate(expression):
        result = devtools.call("Runtime.evaluate", {
            "expression": expression, "returnByValue": True,
            "awaitPromise": True, "userGesture": True,
        }, session)
        if "exceptionDetails" in result:
            fail(f"browser expression failed: {result['exceptionDetails']!r}")
        return result.get("result", {}).get("value")

    def wait_for(expression, description):
        deadline = time.monotonic() + 40
        while time.monotonic() < deadline:
            try:
                if evaluate(expression):
                    return
            except NavigationContextPending:
                pass
            time.sleep(0.08)
        fail(f"timed out waiting for {description}")

    wait_for(
        "(() => { const html = document.documentElement; const root = document.querySelector('.mech-root'); "
        "const input = document.querySelector('.repl-input'); return Boolean(html && root && input && "
        "html.dataset.mechDocumentStatus === 'ready' && root.dataset.mechDocumentStatus === 'ready' && "
        "root.dataset.mechConsoleStatus === 'ready'); })()",
        "the standalone document controller",
    )

    def submit(command):
        encoded = json.dumps(command)
        if not evaluate(
            "(() => { const input = document.querySelector('.repl-input'); if (!input) return false; "
            f"input.focus(); input.value = {encoded}; input.dispatchEvent(new KeyboardEvent('keydown', {{key: 'Enter', bubbles: true, cancelable: true}})); return true; }})()"
        ):
            fail(f"could not submit browser REPL command: {command}")

    def wait_for_new_symbol_value(previous_count, name, value, description):
        wait_for(
            "(() => { const tables = [...document.querySelectorAll('.mech-repl-symbols')]; "
            f"const table = tables.at(-1); return tables.length > {previous_count} && "
            f"[...(table?.tBodies[0]?.rows || [])].some(row => "
            f"row.cells[0]?.textContent.trim() === {json.dumps(name)} && "
            f"row.textContent.includes({json.dumps(value)})); }})()",
            description,
        )

    submit("answer = 58")
    submit("answer")
    wait_for("[...document.querySelectorAll('.mech-repl-result')].some(row => /58/.test(row.textContent))", "the resident source value")
    previous_symbol_tables = evaluate("document.querySelectorAll('.mech-repl-symbols').length")
    submit(":whos answer")
    wait_for_new_symbol_value(previous_symbol_tables, "answer", "58", "the resident symbol value")
    submit(":clear")
    wait_for("[...document.querySelectorAll('.mech-repl-info')].some(row => /Resident workspace cleared/.test(row.textContent))", "the resident workspace clear")
    submit("answer := 59")
    previous_symbol_tables = evaluate("document.querySelectorAll('.mech-repl-symbols').length")
    submit(":whos answer")
    wait_for_new_symbol_value(previous_symbol_tables, "answer", "59", "the resident symbol value after reset")
    popup_state = evaluate("""
(() => {
  const root = document.querySelector('.mech-root');
  const pane = document.querySelector('#mech-console, .console-pane');
  const transcript = document.querySelector('.mech-repl-transcript');
  const value = [...document.querySelectorAll('.mech-var-name')].find(element =>
    !element.closest('#mech-console, .console-pane') &&
    (element.dataset.mechVarName || element.textContent.trim()) === 'answer');
  if (!root || !pane || !transcript || !value) return null;
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  const transcriptEntries = transcript.children.length;
  value.click();
  const popup = document.querySelector('.mech-inline-popup[data-mech-repl-popup]');
  if (!popup) return null;
  const style = getComputedStyle(popup);
  const rect = popup.getBoundingClientRect();
  const valueRect = value.getBoundingClientRect();
  const result = {
    consoleClosed: root.dataset.mechConsoleOpen === 'false' && pane.hidden,
    rendered: /59/.test(popup.textContent || ''),
    role: popup.getAttribute('role'),
    styled:
      style.position === 'fixed' && style.backgroundColor !== 'rgba(0, 0, 0, 0)' &&
      rect.width >= 200 && rect.height > 40,
    contained:
      rect.left >= 0 && rect.top >= 0 && rect.right <= innerWidth && rect.bottom <= innerHeight,
    anchored: Math.abs(rect.top - valueRect.top) < 80,
    transcriptClean: transcript.children.length === transcriptEntries,
  };
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: 'Escape', bubbles: true, cancelable: true,
  }));
  result.dismissed = !document.querySelector('.mech-inline-popup[data-mech-repl-popup]');
  document.dispatchEvent(new KeyboardEvent('keydown', {
    key: '`', bubbles: true, cancelable: true,
  }));
  result.reopened = root.dataset.mechConsoleOpen === 'true' && !pane.hidden;
  return result;
})()
""")
    if (
        popup_state is None or
        not popup_state["consoleClosed"] or
        not popup_state["rendered"] or
        popup_state["role"] != "dialog" or
        not popup_state["styled"] or
        not popup_state["contained"] or
        not popup_state["anchored"] or
        not popup_state["transcriptClean"] or
        not popup_state["dismissed"] or
        not popup_state["reopened"]
    ):
        fail(f"closed standalone console did not show a styled value popup: {popup_state!r}")
    previous_symbol_tables = evaluate("document.querySelectorAll('.mech-repl-symbols').length")
    submit(":whos ans")
    wait_for_new_symbol_value(
        previous_symbol_tables,
        "ans",
        "59",
        "the popup selection becoming ans",
    )
finally:
    if websocket is not None:
        websocket.close()
    if process is not None:
        try:
            os.killpg(process.pid, signal.SIGTERM)
            process.wait(timeout=10)
        except (OSError, subprocess.TimeoutExpired):
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
PY

for request in \
  "GET /${page_relative} " \
  'GET /_mech/pkg/mech_wasm.js ' \
  'GET /_mech/pkg/mech_wasm_bg.wasm '; do
  grep -F "$request" "$server_log" >/dev/null || {
    echo "standalone browser did not request $request" >&2
    sed -n '1,240p' "$server_log" >&2 || true
    exit 1
  }
done

if grep -E 'GET /(code|source)/|GET /_mech/project-sources\.json|GET /mech\.mcfg|GET /_mech/project\.js' "$server_log" >/dev/null; then
  echo "standalone browser requested a server-only Mech route" >&2
  sed -n '1,240p' "$server_log" >&2 || true
  exit 1
fi

echo "formatted static document browser smoke passed"
