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


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=PROFILES, required=True)
    parser.add_argument("--locked", action="store_true", help="require an unchanged workspace lockfile")
    args = parser.parse_args()
    features, expected_exports = PROFILES[args.profile]

    cargo_options = ["--no-default-features", "--features", features]
    lockfile = ROOT / "Cargo.lock"
    original_lock = None
    if args.locked:
        original_lock = lockfile.read_bytes()
        # wasm-pack reads metadata before forwarding the Cargo build options.
        subprocess.run(
            ["cargo", "metadata", "--locked", "--format-version", "1",
             "--manifest-path", "src/wasm/Cargo.toml"],
            cwd=ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
        )
        cargo_options.append("--locked")

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
        *cargo_options,
    )

    if original_lock is not None and lockfile.read_bytes() != original_lock:
        raise SystemExit("wasm-pack changed Cargo.lock during a locked build")

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


if __name__ == "__main__":
    main()
