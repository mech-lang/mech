#!/usr/bin/env python3
"""Measure the resident Halide Metal EKF control at the matched GPU workload."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import shutil
import statistics
import subprocess
import tempfile
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]


def run(command: list[str], env: dict[str, str]) -> str:
    return subprocess.run(
        command,
        cwd=ROOT,
        env=env,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    ).stdout


def throughput(output: str) -> float:
    match = re.search(r"^throughput: ([0-9.eE+-]+)$", output, re.MULTILINE)
    if match is None:
        raise RuntimeError(f"missing throughput in output:\n{output}")
    return float(match.group(1))


def checksum(output: str) -> float:
    match = re.search(r"^checksum: ([0-9.eE+-]+)$", output, re.MULTILINE)
    if match is None:
        raise RuntimeError(f"missing checksum in output:\n{output}")
    return float(match.group(1))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--instances", type=int, default=500_000)
    parser.add_argument("--turns", type=int, default=40)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--halide-cxx", default=shutil.which("clang++") or "clang++")
    parser.add_argument(
        "--output",
        type=Path,
        default=HERE.parent / "results/apple-m1-halide-metal-fused-2026-08-31.json",
    )
    args = parser.parse_args()
    env = os.environ.copy()
    env["DYLD_LIBRARY_PATH"] = "/opt/homebrew/opt/halide/lib"

    with tempfile.TemporaryDirectory(prefix="mech-ekf-halide-metal-") as temp:
        binary = Path(temp) / "halide-ekf"
        run(
            [
                args.halide_cxx,
                "-O3",
                "-std=c++17",
                str(HERE / "halide_ekf.cpp"),
                "-I/opt/homebrew/opt/halide/include",
                "-L/opt/homebrew/opt/halide/lib",
                "-lHalide",
                "-o",
                str(binary),
            ],
            env,
        )
        result: dict[str, object] = {
            "schema_version": 1,
            "generated_at": dt.datetime.now().astimezone().isoformat(),
            "configuration": {
                "instances": args.instances,
                "turns": args.turns,
                "samples": args.samples,
                "backend": "native Metal",
                "synchronized_per_turn": True,
                "schedule": "single fused tuple output with shared scalar intermediates",
            },
            "rows": {},
        }
        rows: dict[str, object] = result["rows"]  # type: ignore[assignment]
        for mode in ("unchecked", "checked"):
            command = [str(binary), str(args.instances), str(args.turns), mode, "gpu"]
            run(command, env)  # warm the Metal runtime before collecting samples
            outputs = [run(command, env) for _ in range(args.samples)]
            rows[f"Halide GPU Metal {mode}"] = {
                "command": command,
                "throughput": [throughput(output) for output in outputs],
                "checksums": [checksum(output) for output in outputs],
            }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    for label, row in rows.items():
        print(f"{label}: {statistics.median(row['throughput']) / 1_000_000:.3f} M turns/s")  # type: ignore[index]


if __name__ == "__main__":
    main()
