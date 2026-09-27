#!/usr/bin/env python3
"""Build one of the supported Mech browser/WASM profiles."""

from __future__ import annotations

import argparse
import json
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
    "browser-compute-canary": (
        "browser_compute_canary",
        (
            "export class WasmDocument",
            "export class WasmMixedComputeProject",
            "static fromSource(",
        ),
    ),
    "browser-workshop": (
        "browser_workshop",
        (
            "export class WasmDocument",
            "export class WasmRepl",
            "export class WasmKernel",
            "export class WasmSceneProgram",
        ),
    ),
}


def run(*command: str) -> None:
    subprocess.run(command, cwd=ROOT, check=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=PROFILES, required=True)
    parser.add_argument(
        "--strip-debug-names",
        action="store_true",
        help="regenerate the package without the WASM debugging name section",
    )
    parser.add_argument(
        "--wasm-bindgen",
        default="wasm-bindgen",
        help="matching wasm-bindgen CLI path (used with --strip-debug-names)",
    )
    args = parser.parse_args()
    features, expected_exports = PROFILES[args.profile]
    bindgen = None
    if args.strip_debug_names:
        bindgen = shutil.which(args.wasm_bindgen)
        if bindgen is None:
            raise SystemExit(
                "--strip-debug-names requires the matching wasm-bindgen CLI; "
                "install it on PATH or pass --wasm-bindgen /path/to/wasm-bindgen"
            )

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
        "--release",
        "--no-default-features",
        "--features",
        features,
    )
    if bindgen is not None:
        metadata = json.loads(subprocess.run(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked", "--offline"],
            cwd=ROOT, check=True, capture_output=True, text=True,
        ).stdout)
        artifact = Path(metadata["target_directory"]) / "wasm32-unknown-unknown/release/mech_wasm.wasm"
        if not artifact.is_file():
            raise SystemExit(f"compiled WASM artifact is missing: {artifact}")
        # Use the same compiled module and the official binding generator;
        # only its optional debugging names are removed. Regenerate the JS
        # and WASM together so the exported binding ABI stays paired.
        run(
            bindgen, str(artifact), "--target", "web",
            "--out-dir", str(PACKAGE), "--remove-name-section",
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
