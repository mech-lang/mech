#!/usr/bin/env python3
"""Run the served million-particle application through a real WebGPU browser."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import queue
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time
import urllib.request
import urllib.parse


ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / "src" / "wasm" / "pkg"


def fail(message: str) -> None:
    raise SystemExit(f"GPU particle browser smoke failed: {message}")


def browser_path(explicit: str | None) -> Path:
    candidates = [explicit, os.environ.get("CHROME_BIN"), os.environ.get("EDGE_BIN")]
    if sys.platform == "darwin":
        candidates.extend([
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        ])
    elif os.name == "nt":
        roots = [os.environ.get("PROGRAMFILES"), os.environ.get("PROGRAMFILES(X86)"), os.environ.get("LOCALAPPDATA")]
        for root in filter(None, roots):
            candidates.extend([
                str(Path(root) / "Google/Chrome/Application/chrome.exe"),
                str(Path(root) / "Microsoft/Edge/Application/msedge.exe"),
            ])
    else:
        candidates.extend(shutil.which(name) for name in ("google-chrome", "chromium", "microsoft-edge"))
    for candidate in filter(None, candidates):
        path = Path(candidate)
        if path.is_file():
            return path
    fail("Chrome or Edge was not found; pass --browser or set CHROME_BIN/EDGE_BIN")


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def wait_for_server(url: str, process: subprocess.Popen[bytes], timeout: float = 30) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if process.poll() is not None:
            fail(f"Mech server exited with status {process.returncode}")
        try:
            with urllib.request.urlopen(url, timeout=1) as response:
                if response.status == 200:
                    return
        except OSError:
            time.sleep(0.1)
    fail(f"Mech server did not become ready at {url}")


def build() -> None:
    if os.name == "nt":
        subprocess.run([
            "powershell", "-ExecutionPolicy", "Bypass", "-File",
            str(ROOT / "scripts/build-mech-gpu-browser.ps1"),
        ], cwd=ROOT, check=True)
    else:
        subprocess.run([str(ROOT / "scripts/build-mech-gpu-browser.sh")], cwd=ROOT, check=True)
    subprocess.run(
        ["cargo", "build", "--release", "--features", "compute_backends_native"],
        cwd=ROOT,
        check=True,
    )


def verify_package() -> None:
    glue = PACKAGE / "mech_wasm.js"
    wasm = PACKAGE / "mech_wasm_bg.wasm"
    if not glue.is_file() or not wasm.is_file():
        fail("GPU WASM package is missing; rerun with --build")
    source = glue.read_text(encoding="utf-8")
    if "export class WasmMixedComputeProject" not in source or "static fromSource(" not in source:
        fail("selected WASM package is not the mixed CPU/GPU profile; rerun with --build")


class WebSocket:
    def __init__(self, url: str) -> None:
        parsed = urllib.parse.urlparse(url)
        self.socket = socket.create_connection((parsed.hostname, parsed.port), timeout=30)
        key = base64.b64encode(os.urandom(16)).decode("ascii")
        request = (
            f"GET {parsed.path} HTTP/1.1\r\n"
            f"Host: {parsed.hostname}:{parsed.port}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n\r\n"
        )
        self.socket.sendall(request.encode("ascii"))
        response = b""
        while b"\r\n\r\n" not in response:
            response += self.socket.recv(4096)
        if not response.startswith(b"HTTP/1.1 101"):
            raise RuntimeError(f"WebSocket handshake failed: {response[:200]!r}")
        expected = base64.b64encode(
            hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode("ascii")).digest()
        ).decode("ascii")
        if f"sec-websocket-accept: {expected}".lower() not in response.decode("ascii").lower():
            raise RuntimeError("WebSocket handshake returned the wrong accept key")

    def send_text(self, text: str) -> None:
        payload = text.encode("utf-8")
        mask = os.urandom(4)
        length = len(payload)
        if length < 126:
            header = bytes((0x81, 0x80 | length))
        elif length < 65_536:
            header = bytes((0x81, 0xFE)) + struct.pack("!H", length)
        else:
            header = bytes((0x81, 0xFF)) + struct.pack("!Q", length)
        masked = bytes(value ^ mask[index % 4] for index, value in enumerate(payload))
        self.socket.sendall(header + mask + masked)

    def _receive_exact(self, length: int) -> bytes:
        chunks = b""
        while len(chunks) < length:
            chunk = self.socket.recv(length - len(chunks))
            if not chunk:
                raise RuntimeError("WebSocket closed")
            chunks += chunk
        return chunks

    def receive_text(self) -> str:
        while True:
            first, second = self._receive_exact(2)
            opcode = first & 0x0F
            length = second & 0x7F
            if length == 126:
                length = struct.unpack("!H", self._receive_exact(2))[0]
            elif length == 127:
                length = struct.unpack("!Q", self._receive_exact(8))[0]
            mask = self._receive_exact(4) if second & 0x80 else None
            payload = self._receive_exact(length)
            if mask is not None:
                payload = bytes(value ^ mask[index % 4] for index, value in enumerate(payload))
            if opcode == 0x9:
                self.socket.sendall(bytes((0x8A, len(payload))) + payload)
                continue
            if opcode == 0x8:
                raise RuntimeError("WebSocket closed")
            if opcode == 0x1:
                return payload.decode("utf-8")


class ChromeCdp:
    def __init__(self, process: subprocess.Popen[bytes], websocket_url: str) -> None:
        self.process = process
        self.websocket = WebSocket(websocket_url)
        self.messages: queue.Queue[dict[str, object]] = queue.Queue()
        self.next_id = 1
        self.pending: dict[int, dict[str, object]] = {}
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self) -> None:
        try:
            while self.process.poll() is None:
                self.messages.put(json.loads(self.websocket.receive_text()))
        except (OSError, RuntimeError, json.JSONDecodeError):
            # The debugging socket closes as part of normal browser teardown.
            return

    def call(
        self,
        method: str,
        params: dict[str, object] | None = None,
        session_id: str | None = None,
        timeout: float = 30,
    ) -> dict[str, object]:
        message_id = self.next_id
        self.next_id += 1
        message: dict[str, object] = {
            "id": message_id,
            "method": method,
            "params": params or {},
        }
        if session_id is not None:
            message["sessionId"] = session_id
        self.websocket.send_text(json.dumps(message))
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if message_id in self.pending:
                response = self.pending.pop(message_id)
                break
            try:
                response = self.messages.get(timeout=min(0.25, deadline - time.monotonic()))
            except queue.Empty:
                if self.process.poll() is not None:
                    fail(f"browser exited with status {self.process.returncode}")
                continue
            response_id = response.get("id")
            if response_id != message_id:
                if isinstance(response_id, int):
                    self.pending[response_id] = response
                continue
            break
        else:
            fail(f"browser command {method} timed out")
        if "error" in response:
            raise RuntimeError(f"browser command {method} failed: {response['error']}")
        result = response.get("result", {})
        return result if isinstance(result, dict) else {}


def live_dom(pipe: ChromeCdp, session_id: str) -> str:
    result = pipe.call(
        "Runtime.evaluate",
        {
            "expression": "document.documentElement.outerHTML",
            "returnByValue": True,
        },
        session_id,
    )
    remote = result.get("result", {})
    if not isinstance(remote, dict):
        return ""
    value = remote.get("value", "")
    return value if isinstance(value, str) else ""


def wait_for_debugger(port: int, process: subprocess.Popen[bytes]) -> str:
    endpoint = f"http://127.0.0.1:{port}/json/version"
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if process.poll() is not None:
            fail(f"browser exited with status {process.returncode}")
        try:
            with urllib.request.urlopen(endpoint, timeout=1) as response:
                version = json.load(response)
            websocket_url = version.get("webSocketDebuggerUrl")
            if isinstance(websocket_url, str):
                return websocket_url
        except OSError:
            time.sleep(0.1)
    fail("browser debugging endpoint did not become ready")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--browser", help="path to Chrome or Edge")
    parser.add_argument("--build", action="store_true", help="rebuild WASM and the release server first")
    parser.add_argument("--backend", choices=("auto", "cpu", "gpu"), default="auto")
    parser.add_argument("--timeout", type=int, default=180)
    args = parser.parse_args()

    if args.build:
        build()
    verify_package()
    browser = browser_path(args.browser)
    mech = ROOT / "target" / "release" / ("mech.exe" if os.name == "nt" else "mech")
    if not mech.is_file():
        fail("release Mech executable is missing; rerun with --build")

    port = free_port()
    page_url = f"http://127.0.0.1:{port}/?mech-gpu-smoke=1"
    work = Path(tempfile.mkdtemp(prefix="mech-gpu-smoke-"))
    passed = False
    try:
        server_log = (work / "server.log").open("wb")
        browser_log = (work / "browser.log").open("wb")
        server = subprocess.Popen(
            [
                str(mech), "serve", "examples/gpu-particles", "--port", str(port),
                "--backend", args.backend,
            ],
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=server_log,
            stderr=subprocess.STDOUT,
        )
        browser_process: subprocess.Popen[bytes] | None = None
        try:
            wait_for_server(page_url, server)
            debugger_port = free_port()
            command = [
                str(browser),
                "--headless=new",
                f"--remote-debugging-port={debugger_port}",
                "--enable-unsafe-webgpu",
                "--ignore-gpu-blocklist",
                "--no-first-run",
                "--no-default-browser-check",
                "--run-all-compositor-stages-before-draw",
                f"--user-data-dir={work / 'profile'}",
            ]
            if sys.platform.startswith("linux"):
                command.extend(["--no-sandbox", "--enable-features=Vulkan", "--use-angle=swiftshader"])
            command.append("about:blank")
            browser_process = subprocess.Popen(
                command,
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=browser_log,
            )
            pipe = ChromeCdp(browser_process, wait_for_debugger(debugger_port, browser_process))
            target = pipe.call("Target.createTarget", {"url": page_url})
            target_id = target.get("targetId")
            if not isinstance(target_id, str):
                fail(f"browser did not create a page target; artifacts: {work}")
            attached = pipe.call(
                "Target.attachToTarget",
                {"targetId": target_id, "flatten": True},
            )
            session_id = attached.get("sessionId")
            if not isinstance(session_id, str):
                fail(f"browser did not attach to the page target; artifacts: {work}")
            deadline = time.monotonic() + args.timeout
            dom = ""
            while time.monotonic() < deadline:
                try:
                    dom = live_dom(pipe, session_id)
                except RuntimeError:
                    time.sleep(0.1)
                    continue
                if 'data-mech-gpu-smoke="passed"' in dom:
                    break
                if 'data-mech-gpu-smoke="failed"' in dom:
                    break
                time.sleep(0.1)
            (work / "page.html").write_text(dom, encoding="utf-8")
            if 'data-mech-gpu-smoke="passed"' not in dom:
                marker = 'data-mech-gpu-smoke-error="'
                detail = dom.split(marker, 1)[1].split('"', 1)[0] if marker in dom else "acceptance marker was not reached"
                fail(f"{detail}; artifacts: {work}")
            if "1,000,000" not in dom:
                fail(f"page did not report one million particles; artifacts: {work}")
            expected_backend = "wgpu" if args.backend in ("auto", "gpu") else "cpu-scalar"
            if f'data-mech-compute-backend="{expected_backend}"' not in dom:
                fail(f"page did not select {expected_backend} compute; artifacts: {work}")
            if expected_backend == "wgpu":
                marker = 'data-mech-gpu-smoke-max-completions-in-flight="1"'
                if marker not in dom:
                    fail(f"delayed GPU completion overlapped another dispatch; artifacts: {work}")
                for name in ("delayed-completions", "pending-observations"):
                    marker = f'data-mech-gpu-smoke-{name}="'
                    if marker not in dom:
                        fail(f"delayed completion evidence {name} is missing; artifacts: {work}")
                    value = int(dom.split(marker, 1)[1].split('"', 1)[0])
                    if value < 3:
                        fail(f"delayed completion evidence {name} was only {value}; artifacts: {work}")
                if 'data-mech-gpu-smoke-readback-bytes="0"' not in dom:
                    fail(f"report-only elementwise dispatch copied output data to the CPU; artifacts: {work}")
            print("Compute particle browser smoke passed")
            print(f"browser: {browser}")
            print(f"backend: {expected_backend}")
            print("particles: 1,000,000")
            print("compute frames: advanced")
            print("pointer -> Mech CPU transaction -> compute inputs: verified")
            passed = True
        finally:
            if browser_process is not None and browser_process.poll() is None:
                browser_process.terminate()
                try:
                    browser_process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    browser_process.kill()
                    browser_process.wait()
            server.terminate()
            try:
                server.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()
            server_log.close()
            browser_log.close()
    finally:
        if passed:
            shutil.rmtree(work)
        else:
            print(f"GPU particle smoke artifacts retained at {work}", file=sys.stderr)


if __name__ == "__main__":
    main()
