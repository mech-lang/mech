#!/usr/bin/env python3
"""Run the served million-particle application through a real WebGPU browser."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request


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
    subprocess.run(["cargo", "build", "--release"], cwd=ROOT, check=True)


def verify_package() -> None:
    glue = PACKAGE / "mech_wasm.js"
    wasm = PACKAGE / "mech_wasm_bg.wasm"
    if not glue.is_file() or not wasm.is_file():
        fail("GPU WASM package is missing; rerun with --build")
    source = glue.read_text(encoding="utf-8")
    if "export class WasmMixedGpuProject" not in source or "static fromSource(" not in source:
        fail("selected WASM package is not the mixed CPU/GPU profile; rerun with --build")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--browser", help="path to Chrome or Edge")
    parser.add_argument("--build", action="store_true", help="rebuild WASM and the release server first")
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
            [str(mech), "serve", "examples/gpu-particles", "--port", str(port)],
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=server_log,
            stderr=subprocess.STDOUT,
        )
        try:
            wait_for_server(page_url, server)
            command = [
                str(browser),
                "--headless=new",
                "--enable-unsafe-webgpu",
                "--ignore-gpu-blocklist",
                "--no-first-run",
                "--no-default-browser-check",
                "--run-all-compositor-stages-before-draw",
                f"--virtual-time-budget={args.timeout * 1000}",
                f"--user-data-dir={work / 'profile'}",
                "--dump-dom",
            ]
            if sys.platform.startswith("linux"):
                command.extend(["--no-sandbox", "--enable-features=Vulkan", "--use-angle=swiftshader"])
            command.append(page_url)
            result = subprocess.run(
                command,
                cwd=ROOT,
                stdout=subprocess.PIPE,
                stderr=browser_log,
                timeout=args.timeout + 60,
            )
            dom = result.stdout.decode("utf-8", errors="replace")
            (work / "page.html").write_text(dom, encoding="utf-8")
            if result.returncode != 0:
                fail(f"browser exited with status {result.returncode}; artifacts: {work}")
            if 'data-mech-gpu-smoke="passed"' not in dom:
                marker = 'data-mech-gpu-smoke-error="'
                detail = dom.split(marker, 1)[1].split('"', 1)[0] if marker in dom else "acceptance marker was not reached"
                fail(f"{detail}; artifacts: {work}")
            if "1,000,000" not in dom:
                fail(f"page did not report one million particles; artifacts: {work}")
            print("GPU particle browser smoke passed")
            print(f"browser: {browser}")
            print("particles: 1,000,000")
            print("GPU frames: advanced")
            print("pointer -> Mech CPU transaction -> GPU inputs: verified")
            passed = True
        finally:
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
