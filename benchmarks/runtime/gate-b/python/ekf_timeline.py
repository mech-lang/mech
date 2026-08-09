#!/usr/bin/env python3
"""Ordered pure-Python timing samples for the frozen Gate B EKF."""

from __future__ import annotations

import argparse
import gc
import importlib.util
import json
import struct
import time
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[4]
TRACE_PATH = ROOT / "benchmarks/runtime/gate-b/ekf-input-v1.bin"
ORACLE_PATH = ROOT / "scripts/generate-gate-b-ekf-trace.py"
EPISODE_LENGTH = 4_096


def load_oracle() -> Any:
    spec = importlib.util.spec_from_file_location("gate_b_oracle", ORACLE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load Gate B scalar oracle")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_trace() -> tuple[tuple[float, float, float, float], ...]:
    values = struct.iter_unpack("<4d", TRACE_PATH.read_bytes())
    trace = tuple(values)
    if len(trace) != EPISODE_LENGTH:
        raise RuntimeError("Gate B trace has the wrong number of rows")
    return trace


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--samples", type=int, default=60)
    args = parser.parse_args()
    if args.samples < 1:
        parser.error("--samples must be positive")

    oracle = load_oracle()
    trace = load_trace()
    gc_started_ns = 0
    gc_total_ns = 0

    def gc_event(phase: str, _info: dict[str, int]) -> None:
        nonlocal gc_started_ns, gc_total_ns
        if phase == "start":
            gc_started_ns = time.perf_counter_ns()
        elif gc_started_ns:
            gc_total_ns += time.perf_counter_ns() - gc_started_ns
            gc_started_ns = 0

    gc.callbacks.append(gc_event)
    try:
        for sample in range(args.samples + 1):
            state = oracle.INITIAL_STATE
            covariance = oracle.INITIAL_COVARIANCE
            gc_before = gc_total_ns
            started = time.perf_counter_ns()
            for inputs in trace:
                state, covariance, _, _, _ = oracle.ekf_step(
                    state, covariance, inputs
                )
            elapsed_ns = time.perf_counter_ns() - started
            if sample:
                print(
                    json.dumps(
                        {
                            "lane": "python-scalar",
                            "sample": sample - 1,
                            "turns": EPISODE_LENGTH,
                            "elapsed_ns": elapsed_ns,
                            "gc_ns": gc_total_ns - gc_before,
                        },
                        separators=(",", ":"),
                    )
                )
    finally:
        gc.callbacks.remove(gc_event)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
