#!/usr/bin/env python3
"""Measure the two PUC Lua EKF sources under one Lua interpreter."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import platform
import re
import shutil
import statistics
import subprocess
from pathlib import Path


HERE = Path(__file__).resolve().parent
THROUGHPUT = re.compile(r"^throughput: ([0-9.eE+-]+)$", re.MULTILINE)
CHECKSUM = re.compile(r"^checksum: ([0-9.eE+-]+)$", re.MULTILINE)
FAULTS = re.compile(r"^faults: ([0-9]+)$", re.MULTILINE)


def run(command: list[str]) -> str:
    completed = subprocess.run(
        command,
        cwd=HERE.parents[4],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=True,
    )
    return completed.stdout


def number(pattern: re.Pattern[str], output: str) -> float:
    match = pattern.search(output)
    if match is None:
        raise RuntimeError(f"missing {pattern.pattern} in output:\n{output}")
    return float(match.group(1))


def machine() -> dict[str, str]:
    result = {"model": platform.node(), "cpu": platform.processor(), "architecture": platform.machine()}
    if platform.system() == "Darwin":
        try:
            hardware = subprocess.check_output(
                ["system_profiler", "SPHardwareDataType", "SPDisplaysDataType"],
                text=True,
                stderr=subprocess.DEVNULL,
            )
        except (OSError, subprocess.CalledProcessError):
            hardware = ""
        fields = {}
        for raw in hardware.splitlines():
            if ":" in raw:
                key, value = raw.strip().split(":", 1)
                fields.setdefault(key, value.strip())
        model = fields.get("Model Identifier")
        chip = fields.get("Chip")
        cores = fields.get("Total Number of Cores")
        if model:
            result["model"] = f"{fields.get('Model Name', 'Mac')} {model}"
        if chip:
            result["cpu"] = f"{chip}, {cores}" if cores else chip
        if fields.get("Memory"):
            result["memory"] = fields["Memory"]
        if fields.get("Chipset Model"):
            result["gpu"] = f"{fields['Chipset Model']}, {fields.get('Metal Support', 'Metal support unknown')}"
    return result


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lua", default=shutil.which("lua") or "lua")
    parser.add_argument("--instances", type=int, default=10_000)
    parser.add_argument("--turns", type=int, default=20)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--output", type=Path, default=HERE.parent / "results/apple-m1-lua-2026-09-01.json")
    args = parser.parse_args()

    sources = {
        "baseline": (HERE / "luajit_fast.lua", "flat fixed-shape source using zero-based Lua tables"),
        "advanced": (HERE / "lua_advanced.lua", "flat fixed-shape source using dense one-based Lua array storage"),
    }
    variants: dict[str, dict[str, object]] = {}
    for variant, (source, description) in sources.items():
        entry: dict[str, object] = {"source": str(source.relative_to(HERE.parents[4])), "description": description}
        for mode in ("checked", "unchecked"):
            samples: list[float] = []
            checksums: list[float] = []
            faults: list[int] = []
            command = [args.lua, str(source), str(args.instances), str(args.turns), mode]
            for _ in range(args.samples):
                output = run(command)
                samples.append(number(THROUGHPUT, output) / 1_000_000.0)
                checksums.append(number(CHECKSUM, output))
                faults.append(int(number(FAULTS, output)))
            entry[mode] = {
                "throughput_millions": samples,
                "checksum": statistics.median(checksums),
                "faults": max(faults),
            }
        variants[variant] = entry

    result = {
        "schema_version": 2,
        "generated_at": dt.datetime.now().astimezone().isoformat(),
        "platform": platform.platform(),
        "machine": machine(),
        "configuration": {
            "instances": args.instances,
            "turns": args.turns,
            "samples": args.samples,
            "synchronization": "after every measured process invocation",
            "runtime": "PUC Lua; no JIT or FFI",
            "timed_region": "steady-state scalar turn loop",
            "note": "Input construction, five-turn warmup, reset, and checksum are outside the timed region.",
        },
        "variants": variants,
        "notes": [
            "Both variants execute with PUC Lua only; no JIT or FFI is used.",
            "Checked mode validates finite state/covariance values, positive covariance diagonals, and covariance symmetry before publication.",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    for variant, entry in variants.items():
        print(
            f"{variant}: checked {statistics.median(entry['checked']['throughput_millions']):.3f} M/s; "
            f"unchecked {statistics.median(entry['unchecked']['throughput_millions']):.3f} M/s"
        )


if __name__ == "__main__":
    main()
