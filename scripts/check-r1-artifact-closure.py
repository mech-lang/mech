#!/usr/bin/env python3
"""Build representative R1 products and prove every emitted node is declared."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]

REPRESENTATIVES = {
    "standard-native": (
        "cargo",
        "test",
        "--locked",
        "-p",
        "mech",
        "--test",
        "mech_build",
        "distribution_source_bytecode_native_canary",
        "--",
        "--exact",
        "--nocapture",
    ),
    "nbody": (
        "cargo",
        "run",
        "--quiet",
        "--manifest-path",
        "tests/fixtures/d2-contract-generator/Cargo.toml",
        "--target-dir",
        "target/d2-contract-generator",
    ),
    "particles": (
        "cargo",
        "test",
        "--locked",
        "-p",
        "mech-gpu",
        "--features",
        "native",
        "--test",
        "particle_source",
        "particle_arithmetic_reaches_artifact_with_declared_contracts",
        "--",
        "--exact",
        "--nocapture",
    ),
    "ekf": (
        "cargo",
        "test",
        "--locked",
        "-p",
        "mech-wasm",
        "--features",
        "browser_project,browser_compute",
        "project::tests::ekf_scene_advances_on_every_resident_timer_packet",
        "--",
        "--nocapture",
    ),
}


def run(representative: str) -> int:
    command = REPRESENTATIVES[representative]
    print(f"R1 artifact closure: building {representative}", flush=True)
    return subprocess.run(command, cwd=ROOT, check=False).returncode


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "representative",
        choices=(*REPRESENTATIVES, "all"),
        help="representative artifact-producing product to verify",
    )
    args = parser.parse_args()
    representatives = REPRESENTATIVES if args.representative == "all" else (args.representative,)
    for representative in representatives:
        status = run(representative)
        if status != 0:
            print(
                f"R1 artifact closure failed for {representative}",
                file=sys.stderr,
            )
            return status
    print("R1 artifact closure passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
