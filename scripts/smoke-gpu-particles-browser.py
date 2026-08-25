#!/usr/bin/env python3
"""Run the served particle application through a real WebGPU browser."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time

from browser_webgpu_flags import chrome_webgpu_test_flags


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from tests.browser.harness import (
    ChromeSession,
    NavigationContextPending,
    free_port,
    wait_for_http,
)


PACKAGE = ROOT / "src" / "wasm" / "pkg"


def fail(message: str) -> None:
    raise SystemExit(f"GPU particle browser smoke failed: {message}")


def build() -> None:
    subprocess.run(
        [sys.executable, str(ROOT / "scripts/build-wasm.py"), "--profile", "browser-compute"],
        cwd=ROOT,
        check=True,
    )
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


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--browser", help="path to Chrome or Edge")
    parser.add_argument("--build", action="store_true", help="rebuild WASM and the release server first")
    parser.add_argument("--backend", choices=("auto", "cpu", "gpu"), default="auto")
    parser.add_argument(
        "--software-adapter",
        action="store_true",
        help="force Chromium's WebGPU SwiftShader test adapter",
    )
    parser.add_argument("--timeout", type=int, default=180)
    parser.add_argument(
        "--particle-count",
        type=int,
        default=1_000_000,
        help="particle lanes compiled from the checked-in source",
    )
    args = parser.parse_args()
    if args.particle_count <= 0:
        fail("--particle-count must be positive")

    if args.build:
        build()
    verify_package()
    configured_mech = os.environ.get("MECH_BIN")
    mech = Path(configured_mech) if configured_mech else (
        ROOT / "target" / "release" / ("mech.exe" if os.name == "nt" else "mech")
    )
    if not mech.is_absolute():
        mech = ROOT / mech
    if not mech.is_file():
        fail("release Mech executable is missing; rerun with --build")

    port = free_port()
    page_url = f"http://127.0.0.1:{port}/?mech-gpu-smoke={args.particle_count}"
    work = Path(tempfile.mkdtemp(prefix="mech-gpu-smoke-"))
    project_root = ROOT / "examples/gpu-particles"
    project_copy: Path | None = None
    if args.particle_count != 1_000_000:
        target_root = ROOT / "target"
        target_root.mkdir(exist_ok=True)
        project_copy = Path(tempfile.mkdtemp(prefix="gpu-particles-ci-", dir=target_root))
        shutil.copytree(project_root, project_copy, dirs_exist_ok=True)
        particle_source = project_copy / "particles.mec"
        source = particle_source.read_text(encoding="utf-8")
        declaration = "particle-count := 1000000f32"
        if source.count(declaration) != 1:
            fail(
                "checked-in particle source no longer has one canonical particle-count "
                f"declaration; artifacts: {work}; project: {project_copy}"
            )
        particle_source.write_text(
            source.replace(declaration, f"particle-count := {args.particle_count}f32"),
            encoding="utf-8",
        )
        project_root = project_copy
    passed = False
    try:
        server_log = (work / "server.log").open("wb")
        server = subprocess.Popen(
            [
                str(mech), "serve", str(project_root), "--port", str(port),
                "--backend", args.backend,
            ],
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=server_log,
            stderr=subprocess.STDOUT,
        )
        browser_session: ChromeSession | None = None
        try:
            wait_for_http(page_url, server)
            flags = [
                "--ignore-gpu-blocklist",
                "--no-first-run",
                "--no-default-browser-check",
                "--run-all-compositor-stages-before-draw",
            ]
            software_adapter = args.software_adapter or sys.platform.startswith("linux")
            flags.extend(chrome_webgpu_test_flags(
                software_adapter=software_adapter,
                linux=sys.platform.startswith("linux"),
            ))
            browser_session = ChromeSession(
                args.browser,
                work / "profile",
                work / "browser.log",
                flags=flags,
            ).start()
            browser_session.navigate(page_url)
            deadline = time.monotonic() + args.timeout
            dom = ""
            while time.monotonic() < deadline:
                try:
                    dom = browser_session.evaluate("document.documentElement.outerHTML") or ""
                except NavigationContextPending:
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
            formatted_count = f"{args.particle_count:,}"
            if formatted_count not in dom:
                fail(f"page did not report {formatted_count} particles; artifacts: {work}")
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
                    if value < 2:
                        fail(f"delayed completion evidence {name} was only {value}; artifacts: {work}")
                if 'data-mech-gpu-smoke-readback-bytes="0"' not in dom:
                    fail(f"report-only elementwise dispatch copied output data to the CPU; artifacts: {work}")
                if 'data-mech-gpu-smoke-state-advanced="true"' not in dom:
                    fail(f"completion-backed turns did not advance rendered GPU state; artifacts: {work}")
                marker = 'data-mech-gpu-smoke-accepted-dispatches="'
                if marker not in dom:
                    fail(f"resident acknowledgement evidence is missing; artifacts: {work}")
                accepted = int(dom.split(marker, 1)[1].split('"', 1)[0])
                if accepted < 2:
                    fail(f"only {accepted} GPU completions reached the resident host; artifacts: {work}")
                if 'data-mech-gpu-smoke-last-accepted-dispatch-token="' not in dom:
                    fail(f"the accepted resident dispatch identity is missing; artifacts: {work}")
                if 'data-mech-gpu-smoke-disposed="true"' not in dom:
                    fail(f"particle compute resources were not disposed during teardown; artifacts: {work}")
                if 'data-mech-gpu-smoke-page-errors="0"' not in dom:
                    fail(f"the particle page reported a console, page, or promise error; artifacts: {work}")
            print("Compute particle browser smoke passed")
            print(f"browser: {browser_session.browser}")
            print(f"backend: {expected_backend}")
            print(f"particles: {formatted_count}")
            print("compute frames: advanced")
            print("pointer -> Mech CPU transaction -> compute inputs: verified")
            passed = True
        finally:
            if browser_session is not None:
                browser_session.close()
            server.terminate()
            try:
                server.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server.kill()
                server.wait()
            server_log.close()
    finally:
        if passed:
            shutil.rmtree(work)
            if project_copy is not None:
                shutil.rmtree(project_copy)
        else:
            print(f"GPU particle smoke artifacts retained at {work}", file=sys.stderr)
            if project_copy is not None:
                print(f"GPU particle smoke project retained at {project_copy}", file=sys.stderr)


if __name__ == "__main__":
    main()
