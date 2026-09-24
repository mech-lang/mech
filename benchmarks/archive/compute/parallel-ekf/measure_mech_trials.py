#!/usr/bin/env python3
"""Measure every non-fast Mech execution trial used by the progression chart."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import statistics
import subprocess
from pathlib import Path


THROUGHPUT_PATTERNS = {
    "Mech scalar": r"^Mech scalar throughput: ([0-9.eE+-]+) million",
    "Mech scalar unchecked": r"^Mech scalar unchecked throughput: ([0-9.eE+-]+) million",
    "Mech SIMD": r"^Mech SIMD throughput: ([0-9.eE+-]+) million",
    "Mech SIMD unchecked": r"^Mech SIMD unchecked throughput: ([0-9.eE+-]+) million",
    "Mech Cranelift JIT": r"^Mech Cranelift JIT throughput: ([0-9.eE+-]+) million",
    "Mech Cranelift JIT unchecked": r"^Mech Cranelift JIT unchecked throughput: ([0-9.eE+-]+) million",
    "Mech Cranelift SIMD-JIT": r"^Mech Cranelift SIMD-JIT throughput: ([0-9.eE+-]+) million",
    "Mech Cranelift SIMD-JIT unchecked": r"^Mech Cranelift SIMD-JIT unchecked throughput: ([0-9.eE+-]+) million",
    "Mech Cranelift SIMD-JIT parallel": r"^Mech Cranelift SIMD-JIT parallel throughput: ([0-9.eE+-]+) million",
    "Mech Cranelift SIMD-JIT parallel unchecked": r"^Mech Cranelift SIMD-JIT parallel unchecked throughput: ([0-9.eE+-]+) million",
    "Mech Cranelift SIMD-JIT parallel checked fused block": r"^Mech Cranelift SIMD-JIT parallel checked fused block throughput: ([0-9.eE+-]+) million",
    "Mech Cranelift SIMD-JIT parallel unchecked fused block": r"^Mech Cranelift SIMD-JIT parallel unchecked fused block throughput: ([0-9.eE+-]+) million",
    "GPU checked one-turn": r"^GPU checked one-turn throughput: ([0-9.eE+-]+) million",
    "GPU checked repeated": r"^GPU checked repeated throughput \(per-turn validation\): ([0-9.eE+-]+) million",
    "GPU unchecked one-turn": r"^GPU unchecked one-turn throughput: ([0-9.eE+-]+) million",
    "GPU unchecked repeated": r"^GPU unchecked repeated throughput: ([0-9.eE+-]+) million",
    "GPU unchecked in-place one-turn": r"^GPU unchecked in-place one-turn throughput: ([0-9.eE+-]+) million",
    "GPU unchecked in-place repeated": r"^GPU unchecked in-place repeated throughput: ([0-9.eE+-]+) million",
    "GPU unchecked one-submit": r"^GPU unchecked one-submit throughput: ([0-9.eE+-]+) million",
}


def parse_output(text: str) -> dict[str, float]:
    values: dict[str, float] = {}
    for label, pattern in THROUGHPUT_PATTERNS.items():
        match = re.search(pattern, text, flags=re.MULTILINE)
        if match is None:
            raise RuntimeError(f"missing {label!r} in benchmark output")
        values[label] = float(match.group(1))
    return values


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--instances", type=int, default=100_000)
    parser.add_argument("--cpu-turns", type=int, default=5)
    parser.add_argument("--gpu-turns", type=int, default=120)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--workers", type=int, default=8)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.samples < 1:
        raise SystemExit("--samples must be positive")
    command = [
        str(args.binary),
        str(args.instances),
        str(args.cpu_turns),
        "40",
        str(args.gpu_turns),
    ]
    environment = os.environ.copy()
    environment["MECH_PARALLEL_WORKERS"] = str(args.workers)
    warmup = subprocess.run(
        command,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        env=environment,
    ).stdout
    outputs = [
        subprocess.run(
            command,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            env=environment,
        ).stdout
        for _ in range(args.samples)
    ]
    parsed = [parse_output(text) for text in outputs]
    rows = {}
    for label in THROUGHPUT_PATTERNS:
        mode = "unchecked" if "unchecked" in label else "checked"
        rows[label] = {
            "label": label,
            "mode": mode,
            "throughput_millions": [sample[label] for sample in parsed],
            "median_throughput_millions": statistics.median(sample[label] for sample in parsed),
        }
    evidence = {
        "schema_version": 1,
        "generated_at": dt.datetime.now().astimezone().isoformat(),
        "platform": {"system": os.uname().sysname, "machine": os.uname().machine},
        "configuration": {
            "binary": str(args.binary),
            "instances": args.instances,
            "cpu_turns": args.cpu_turns,
            "gpu_turns": args.gpu_turns,
            "workers": args.workers,
            "samples": args.samples,
        },
        "command": command,
        "environment": {"MECH_PARALLEL_WORKERS": str(args.workers)},
        "discarded_warmup_stdout": warmup,
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    for row in rows.values():
        print(f"{row['label']}: {row['median_throughput_millions']:.3f} M/s")


if __name__ == "__main__":
    main()
