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
    "browser": ("browser_project", ("export class WasmDocument",)),
    "browser-compute": (
        "browser_project,browser_compute",
        (
            "export class WasmDocument",
            "export class WasmMixedComputeProject",
            "static fromSource(",
        ),
    ),
}


def run(*command: str) -> None:
    subprocess.run(command, cwd=ROOT, check=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=PROFILES, required=True)
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


if __name__ == "__main__":
    main()
