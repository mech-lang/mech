#!/usr/bin/env python3
"""Build one of the supported Mech browser/WASM profiles."""

from __future__ import annotations

import argparse
from pathlib import Path
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / "src" / "wasm" / "pkg"
PROFILES = {
    "browser": ("browser_project", ("export class WasmDocument", "static validateStaticSources(", "static supportsServedDocumentProvenance(")),
    "browser-compute": (
        "browser_project,browser_compute",
        (
            "export class WasmDocument",
            "export class WasmMixedComputeProject",
            "static fromSource(",
            "static validateStaticSources(",
            "static supportsServedDocumentProvenance(",
        ),
    ),
    "browser-compute-canary": (
        "browser_compute_canary",
        (
            "export class WasmDocument",
            "export class WasmMixedComputeProject",
            "static fromSource(",
            "static validateStaticSources(",
            "static supportsServedDocumentProvenance(",
        ),
    ),
}


def run(*command: str) -> None:
    subprocess.run(command, cwd=ROOT, check=True)


def strip_debug_names(wasm: Path) -> None:
    """Keep executable sections intact while omitting optional debug names."""
    module = wasm.read_bytes()
    if module[:8] != b"\0asm\x01\0\0\0":
        raise ValueError(f"invalid WebAssembly header: {wasm}")

    def unsigned(offset: int, end: int) -> tuple[int, int]:
        value = 0
        for shift in range(0, 35, 7):
            if offset >= end:
                break
            byte = module[offset]
            offset += 1
            value |= (byte & 127) << shift
            if byte < 128 and value <= 0xFFFFFFFF:
                return value, offset
        raise ValueError(f"invalid WebAssembly section length: {wasm}")

    output = bytearray(module[:8])
    offset = 8
    while offset < len(module):
        start = offset
        section = module[offset]
        size, payload = unsigned(offset + 1, len(module))
        offset = payload + size
        if offset > len(module):
            raise ValueError(f"truncated WebAssembly section: {wasm}")
        if section == 0:
            size, name = unsigned(payload, offset)
            if name + size > offset:
                raise ValueError(f"truncated WebAssembly custom section name: {wasm}")
            if module[name:name + size] == b"name":
                continue
        output.extend(module[start:offset])
    wasm.write_bytes(output)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=PROFILES, required=True)
    parser.add_argument("--keep-debug-names", action="store_true", help="retain WASM function names for CPU profiling")
    args = parser.parse_args()
    features, expected_exports = PROFILES[args.profile]

    run("rustup", "target", "add", "wasm32-unknown-unknown")
    shutil.rmtree(PACKAGE, ignore_errors=True)
    run(
        "wasm-pack",
        "build",
        "src/wasm",
        "--target",
        "web",
        "--out-dir",
        "pkg",
        "--no-default-features",
        "--features",
        features,
    )

    glue = PACKAGE / "mech_wasm.js"
    wasm = PACKAGE / "mech_wasm_bg.wasm"
    if not glue.is_file() or not wasm.is_file():
        raise SystemExit(f"{args.profile} WASM build did not produce a complete package")
    source = glue.read_text(encoding="utf-8")
    missing = [export for export in expected_exports if export not in source]
    if missing:
        raise SystemExit(
            f"{args.profile} WASM package is missing expected exports: {', '.join(missing)}"
        )
    if not args.keep_debug_names:
        strip_debug_names(wasm)


if __name__ == "__main__":
    main()
