"""One Chrome DevTools harness shared by the real-browser scenarios."""

from __future__ import annotations

import base64
import hashlib
import json
import os
from pathlib import Path
import queue
import secrets
import shutil
import signal
import socket
import struct
import subprocess
import sys
import threading
import time
import urllib.parse
import urllib.request
from typing import Any


class BrowserFailure(AssertionError):
    """A browser scenario or its infrastructure failed."""


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


def find_browser(explicit: str | os.PathLike[str] | None = None) -> Path:
    candidates: list[str | os.PathLike[str] | None] = [
        explicit,
        os.environ.get("CHROME_BIN"),
        os.environ.get("EDGE_BIN"),
    ]
    if sys.platform == "darwin":
        candidates.extend((
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        ))
    elif os.name == "nt":
        for root in filter(None, (
            os.environ.get("PROGRAMFILES"),
            os.environ.get("PROGRAMFILES(X86)"),
            os.environ.get("LOCALAPPDATA"),
        )):
            candidates.extend((
                Path(root) / "Google/Chrome/Application/chrome.exe",
                Path(root) / "Microsoft/Edge/Application/msedge.exe",
            ))
    else:
        candidates.extend(shutil.which(name) for name in (
            "google-chrome", "chromium", "microsoft-edge", "microsoft-edge-stable",
        ))
    for candidate in filter(None, candidates):
        path = Path(candidate)
        if path.is_file():
            return path
    raise BrowserFailure(
        "Chrome or Edge was not found; pass a browser path or set CHROME_BIN/EDGE_BIN"
    )


def wait_for_http(
    url: str,
    process: subprocess.Popen[Any] | None = None,
    *,
    timeout: float = 30,
) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if process is not None and process.poll() is not None:
            raise BrowserFailure(f"server exited with status {process.returncode}")
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                if response.status == 200:
                    return
        except OSError:
            time.sleep(0.1)
    raise BrowserFailure(f"server did not become ready at {url}")


class WebSocket:
    """Small RFC 6455 client sufficient for the Chrome DevTools protocol."""

    def __init__(self, url: str, timeout: float = 30) -> None:
        parsed = urllib.parse.urlparse(url)
        if parsed.scheme != "ws" or parsed.hostname is None or parsed.port is None:
            raise BrowserFailure(f"unsupported DevTools websocket URL: {url}")
        self.socket = socket.create_connection((parsed.hostname, parsed.port), timeout=timeout)
        self.socket.settimeout(timeout)
        key = base64.b64encode(secrets.token_bytes(16)).decode("ascii")
        authority = parsed.netloc
        path = parsed.path or "/"
        if parsed.query:
            path += f"?{parsed.query}"
        request = (
            f"GET {path} HTTP/1.1\r\n"
            f"Host: {authority}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n\r\n"
        )
        self.socket.sendall(request.encode("ascii"))
        response = self._read_headers()
        if not response.startswith(b"HTTP/1.1 101"):
            raise BrowserFailure(f"DevTools websocket handshake failed: {response[:200]!r}")
        expected = base64.b64encode(hashlib.sha1(
            (key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode("ascii")
        ).digest()).decode("ascii")
        if f"sec-websocket-accept: {expected}".lower() not in response.decode("ascii").lower():
            raise BrowserFailure("DevTools websocket returned the wrong accept key")

    def _read_headers(self) -> bytes:
        response = bytearray()
        while b"\r\n\r\n" not in response:
            chunk = self.socket.recv(1)
            if not chunk:
                raise BrowserFailure("DevTools websocket closed during handshake")
            response.extend(chunk)
            if len(response) > 65_536:
                raise BrowserFailure("DevTools websocket sent oversized response headers")
        return bytes(response)

    def _read_exact(self, count: int) -> bytes:
        response = bytearray()
        while len(response) < count:
            chunk = self.socket.recv(count - len(response))
            if not chunk:
                raise BrowserFailure("DevTools websocket closed unexpectedly")
            response.extend(chunk)
        return bytes(response)

    def send(self, value: str | bytes, opcode: int = 1) -> None:
        payload = value.encode("utf-8") if isinstance(value, str) else value
        header = bytearray([0x80 | opcode])
        if len(payload) < 126:
            header.append(0x80 | len(payload))
        elif len(payload) <= 0xFFFF:
            header.extend((0x80 | 126, *struct.pack("!H", len(payload))))
        else:
            header.extend((0x80 | 127, *struct.pack("!Q", len(payload))))
        mask = os.urandom(4)
        header.extend(mask)
        masked = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
        self.socket.sendall(header + masked)

    def receive(self) -> dict[str, Any]:
        while True:
            first, second = self._read_exact(2)
            opcode = first & 0x0F
            length = second & 0x7F
            if length == 126:
                length = struct.unpack("!H", self._read_exact(2))[0]
            elif length == 127:
                length = struct.unpack("!Q", self._read_exact(8))[0]
            mask = self._read_exact(4) if second & 0x80 else None
            payload = self._read_exact(length)
            if mask is not None:
                payload = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
            if opcode == 0x9:
                self.send(payload, opcode=0xA)
                continue
            if opcode == 0x8:
                raise BrowserFailure("DevTools websocket closed")
            if opcode == 0x1:
                return json.loads(payload.decode("utf-8"))

    def close(self) -> None:
        try:
            self.send(b"", opcode=0x8)
        except OSError:
            pass
        self.socket.close()


class NavigationContextPending(Exception):
    """Evaluation raced a navigation and may be retried."""


def _navigation_context_pending(error: object) -> bool:
    message = str(error).lower()
    return any(marker in message for marker in (
        "cannot find default execution context",
        "cannot find context with specified id",
        "execution context was destroyed",
        "inspected target navigated or closed",
    ))


class DevTools:
    """Concurrent-safe request/response transport for flattened CDP sessions."""

    def __init__(self, process: subprocess.Popen[bytes], websocket_url: str) -> None:
        self.process = process
        self.websocket = WebSocket(websocket_url)
        self.messages: queue.Queue[dict[str, Any]] = queue.Queue()
        self.pending: dict[int, dict[str, Any]] = {}
        self.next_id = 1
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self) -> None:
        try:
            while self.process.poll() is None:
                self.messages.put(self.websocket.receive())
        except (OSError, BrowserFailure, json.JSONDecodeError):
            return

    def call(
        self,
        method: str,
        params: dict[str, Any] | None = None,
        session_id: str | None = None,
        timeout: float = 30,
    ) -> dict[str, Any]:
        request_id = self.next_id
        self.next_id += 1
        request: dict[str, Any] = {"id": request_id, "method": method}
        if params is not None:
            request["params"] = params
        if session_id is not None:
            request["sessionId"] = session_id
        self.websocket.send(json.dumps(request))
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if request_id in self.pending:
                response = self.pending.pop(request_id)
                break
            remaining = max(0.01, min(0.25, deadline - time.monotonic()))
            try:
                response = self.messages.get(timeout=remaining)
            except queue.Empty:
                if self.process.poll() is not None:
                    raise BrowserFailure(f"browser exited with status {self.process.returncode}")
                continue
            response_id = response.get("id")
            if response_id != request_id:
                if isinstance(response_id, int):
                    self.pending[response_id] = response
                continue
            break
        else:
            raise BrowserFailure(f"DevTools command {method} timed out")
        if "error" in response:
            if method == "Runtime.evaluate" and _navigation_context_pending(response["error"]):
                raise NavigationContextPending(response["error"])
            raise BrowserFailure(f"DevTools {method} failed: {response['error']!r}")
        result = response.get("result", {})
        return result if isinstance(result, dict) else {}

    def close(self) -> None:
        self.websocket.close()


class ChromeSession:
    """Own a browser process, a page target, evaluation, polling, and artifacts."""

    def __init__(
        self,
        browser: str | os.PathLike[str] | None,
        profile: str | os.PathLike[str],
        log: str | os.PathLike[str],
        *,
        flags: list[str] | tuple[str, ...] = (),
        startup_timeout: float = 30,
        window_size: tuple[int, int] | None = None,
    ) -> None:
        self.browser = find_browser(browser)
        self.profile = Path(profile)
        self.log = Path(log)
        self.flags = list(flags)
        self.startup_timeout = startup_timeout
        self.window_size = window_size
        self.process: subprocess.Popen[bytes] | None = None
        self.devtools: DevTools | None = None
        self.session_id: str | None = None
        self._log_handle: Any = None

    def start(self) -> "ChromeSession":
        try:
            return self._start()
        except BaseException:
            # Callers cannot own a session until start returns, so partial
            # startup remains this object's responsibility.
            self.close()
            raise

    def _start(self) -> "ChromeSession":
        self.profile.mkdir(parents=True, exist_ok=True)
        debug_port = free_port()
        args = [
            str(self.browser),
            "--headless=new",
            "--no-sandbox",
            "--disable-dev-shm-usage",
            "--remote-allow-origins=*",
            f"--remote-debugging-port={debug_port}",
            f"--user-data-dir={self.profile}",
        ]
        if self.window_size is not None:
            args.append(f"--window-size={self.window_size[0]},{self.window_size[1]}")
        args.extend(self.flags)
        args.append("about:blank")
        self._log_handle = self.log.open("wb")
        self.process = subprocess.Popen(
            args,
            stdout=subprocess.DEVNULL,
            stderr=self._log_handle,
            start_new_session=True,
        )
        endpoint = f"http://127.0.0.1:{debug_port}/json/version"
        deadline = time.monotonic() + self.startup_timeout
        websocket_url = None
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise BrowserFailure(f"browser exited with status {self.process.returncode}")
            try:
                with urllib.request.urlopen(endpoint, timeout=2) as response:
                    version = json.load(response)
                websocket_url = version.get("webSocketDebuggerUrl")
                if isinstance(websocket_url, str):
                    break
            except OSError:
                time.sleep(0.1)
        if not isinstance(websocket_url, str):
            raise BrowserFailure("browser debugging endpoint did not become ready")
        self.devtools = DevTools(self.process, websocket_url)
        target = self.devtools.call("Target.createTarget", {"url": "about:blank"})["targetId"]
        self.session_id = self.devtools.call(
            "Target.attachToTarget", {"targetId": target, "flatten": True}
        )["sessionId"]
        self.call("Page.enable")
        self.call("Runtime.enable")
        return self

    def call(self, method: str, params: dict[str, Any] | None = None) -> dict[str, Any]:
        if self.devtools is None or self.session_id is None:
            raise BrowserFailure("browser session is not running")
        return self.devtools.call(method, params, self.session_id)

    def navigate(self, url: str) -> None:
        self.call("Page.navigate", {"url": url})

    def add_script(self, source: str) -> None:
        self.call("Page.addScriptToEvaluateOnNewDocument", {"source": source})

    def evaluate(self, expression: str, *, user_gesture: bool = True) -> Any:
        result = self.call("Runtime.evaluate", {
            "expression": expression,
            "returnByValue": True,
            "awaitPromise": True,
            "userGesture": user_gesture,
        })
        if "exceptionDetails" in result:
            raise BrowserFailure(f"browser expression failed: {result['exceptionDetails']!r}")
        remote = result.get("result", {})
        if remote.get("type") == "undefined":
            return None
        return remote.get("value")

    def evaluate_json(self, expression: str) -> Any:
        value = self.evaluate(f"(async () => JSON.stringify(await ({expression})))()")
        if not isinstance(value, str):
            raise BrowserFailure(f"browser expression did not produce JSON: {expression}")
        return json.loads(value)

    def wait_for(
        self,
        expression: str,
        description: str,
        *,
        timeout: float = 35,
        interval: float = 0.08,
    ) -> Any:
        deadline = time.monotonic() + timeout
        last = None
        while time.monotonic() < deadline:
            try:
                last = self.evaluate(expression)
            except NavigationContextPending:
                last = None
            if last:
                return last
            time.sleep(interval)
        raise BrowserFailure(f"timed out waiting for {description}; last value was {last!r}")

    def write_dom(self, path: str | os.PathLike[str]) -> None:
        html = self.evaluate("document.documentElement.outerHTML") or ""
        Path(path).write_text(str(html))

    def capture_screenshot(self, path: str | os.PathLike[str]) -> None:
        image = self.call("Page.captureScreenshot", {
            "format": "png", "captureBeyondViewport": False,
        }).get("data")
        if image:
            Path(path).write_bytes(base64.b64decode(image))

    def close(self) -> None:
        if self.devtools is not None:
            try:
                self.devtools.close()
            except (OSError, BrowserFailure):
                pass
            self.devtools = None
        if self.process is not None:
            try:
                if os.name == "nt":
                    self.process.terminate()
                else:
                    os.killpg(self.process.pid, signal.SIGTERM)
                self.process.wait(timeout=10)
            except (OSError, subprocess.TimeoutExpired):
                try:
                    if os.name == "nt":
                        self.process.kill()
                    else:
                        os.killpg(self.process.pid, signal.SIGKILL)
                except OSError:
                    pass
                self.process.wait()
            self.process = None
        if self._log_handle is not None:
            self._log_handle.close()
            self._log_handle = None

    def __enter__(self) -> "ChromeSession":
        return self.start()

    def __exit__(self, _kind: object, _value: object, _traceback: object) -> None:
        self.close()


def visible_expression(selector: str) -> str:
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


def _snapshot_main() -> int:
    import argparse

    parser = argparse.ArgumentParser(description="Capture one checked browser scenario DOM")
    parser.add_argument("--url", required=True)
    parser.add_argument("--profile", required=True)
    parser.add_argument("--log", required=True)
    parser.add_argument("--dom", required=True)
    parser.add_argument("--wait", required=True)
    parser.add_argument("--description", required=True)
    parser.add_argument("--timeout", type=float, default=45)
    parser.add_argument("--browser")
    parser.add_argument("--flag", action="append", default=[])
    args = parser.parse_args()
    session = ChromeSession(
        args.browser, args.profile, args.log, flags=args.flag,
    ).start()
    try:
        session.navigate(args.url)
        session.wait_for(args.wait, args.description, timeout=args.timeout)
        session.write_dom(args.dom)
    except BaseException:
        try:
            session.write_dom(args.dom)
        except BaseException:
            pass
        raise
    finally:
        session.close()
    return 0
