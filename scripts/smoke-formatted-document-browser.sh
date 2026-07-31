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
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
  rm -rf "$work_dir"
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
  "$work_dir/project/indexdep.mec" \
  "$work_dir/shared"
cat > "$work_dir/project/main.mec" <<'MEC'
+> ./café.mec
+> ./extdep.mec
+> ./indexdep.mec
+> ./vendor/support.mec
+> ./vendor/percent.mec
answer := café/value + extdep/value + indexdep/value + support/value + percent/value
answer
MEC
cat > "$work_dir/project/café.mec" <<'MEC'
value := 2
<+ value
MEC
cat > "$work_dir/project/extdep.mec.mec" <<'MEC'
value := 3
<+ value
MEC
cat > "$work_dir/project/indexdep.mec/index.mec" <<'MEC'
value := 5
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

    exact_answer = "(() => { const values = [...document.querySelectorAll('.mech-repl-result-value')]; return values.at(-1)?.textContent.trim() === '28'; })()"
    submit("answer")
    wait_for(exact_answer, "the imported source value")
    submit(":clear")
    wait_for("[...document.querySelectorAll('.mech-repl-info')].some(row => /Document reset/.test(row.textContent))", "the static document reset")
    submit("answer")
    wait_for(exact_answer, "the imported source value after reset")
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
