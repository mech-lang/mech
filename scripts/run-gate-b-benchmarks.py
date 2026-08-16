#!/usr/bin/env python3
"""Run and summarize the controlled Gate B benchmark lanes."""

from __future__ import annotations

import argparse
import json
import math
import os
import platform
import shutil
import statistics
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
SAMPLE_PREFIX = "GATE_B_SAMPLE "
FROZEN_BASE = "437f6c6c636d9818729597342165dfc9af5eb4a7"
FROZEN_B0_BRANCH = "test/resident-ekf-efficacy-contract"
FROZEN_B1_BRANCH = "feat/engine-resident-ekf-substrate"
FROZEN_B1_BASE = "c4f7cb1d27b9645b3f669d944c7e49bcd0829ccc"
B2_BRANCH = "perf/runtime-resident-ekf-efficacy"
FROZEN_B2_BASE = "75d0775209c8ee0eae5480facba3a9b2c9c12143"
B2_EVIDENCE_FLOOR = "d5e41f6fd43c9d21c5858d80dab50e7ce64e9a10"
EPISODE_LENGTH = 4_096
SAMPLE_PROTOCOL = {
    "criterion_sample_size": 10,
    "numpy_sample_size": 10,
    "warm_up_seconds": 1.0,
    "measurement_seconds": 3.0,
    "turns_per_sample": EPISODE_LENGTH,
    "fixture_setup_included_in_timing": False,
    "correctness_included_in_timing": False,
    "profile": "release",
}
SCALED_INSTANCES = (1, 8, 64)
THREAD_VARIABLES = (
    "OMP_NUM_THREADS",
    "OPENBLAS_NUM_THREADS",
    "BLIS_NUM_THREADS",
    "MKL_NUM_THREADS",
    "VECLIB_MAXIMUM_THREADS",
    "NUMEXPR_NUM_THREADS",
    "RAYON_NUM_THREADS",
)
ADVISORY_PERFORMANCE_GATES = {
    "b2_decision": {
        "executor_tax",
        "legacy_gap_closure",
        "raw_epoch_ratio",
        "tail_stability",
    },
    "d1_decision": {
        "complete_turn_control_ratio",
        "legacy_gap_closure",
        "raw_epoch_ratio",
        "source_bytecode_equivalence",
    },
}


def release_qualification(
    report: dict[str, Any],
) -> tuple[str, dict[str, list[str]]]:
    advisories: dict[str, list[str]] = {}
    blocking_passed = True
    for section, advisory_names in ADVISORY_PERFORMANCE_GATES.items():
        gates = report.get(section, {}).get("hard_gates", {})
        if not isinstance(gates, dict) or not gates:
            advisories[section] = []
            blocking_passed = False
            continue
        advisories[section] = sorted(
            name for name in advisory_names if gates.get(name) is False
        )
        if not all(
            passed
            for name, passed in gates.items()
            if name not in advisory_names
        ):
            blocking_passed = False
    return ("Pass" if blocking_passed else "Fail"), advisories


def command_output(command: list[str]) -> str:
    return subprocess.run(
        command,
        cwd=ROOT,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    ).stdout.strip()


def percentile(values: list[float], fraction: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    rank = fraction * (len(ordered) - 1)
    lower = math.floor(rank)
    upper = math.ceil(rank)
    if lower == upper:
        return ordered[lower]
    return ordered[lower] + (ordered[upper] - ordered[lower]) * (rank - lower)


def cargo_target_directory(environment: dict[str, str]) -> Path:
    configured = Path(environment.get("CARGO_TARGET_DIR", "target"))
    if not configured.is_absolute():
        configured = ROOT / configured
    return configured.resolve()


def clear_gate_b_criterion_results(target_dir: Path) -> None:
    criterion_root = target_dir / "criterion"
    if not criterion_root.exists():
        return
    for child in criterion_root.iterdir():
        if child.name != "gate_b" and not child.name.startswith("gate_b_"):
            continue
        if child.is_symlink():
            child.unlink()
        elif child.is_dir():
            shutil.rmtree(child)


def criterion_samples_from_root(criterion_root: Path) -> dict[str, dict[str, Any]]:
    summaries: dict[str, dict[str, Any]] = {}
    if not criterion_root.exists():
        return summaries
    sample_paths = list(criterion_root.glob("gate_b/**/new/sample.json"))
    sample_paths.extend(criterion_root.glob("gate_b_*/**/new/sample.json"))
    for sample_path in sorted(sample_paths):
        payload = json.loads(sample_path.read_text(encoding="utf-8"))
        iterations = payload.get("iters", [])
        times = payload.get("times", [])
        if len(iterations) != len(times):
            raise ValueError(f"mismatched Criterion sample arrays in {sample_path}")
        per_episode = [
            float(elapsed) / float(iteration)
            for iteration, elapsed in zip(iterations, times)
            if float(iteration) > 0.0
        ]
        relative = sample_path.parent.parent.relative_to(criterion_root)
        parts = list(relative.parts)
        if parts[0].startswith("gate_b_"):
            parts = ["gate_b", parts[0][len("gate_b_") :], *parts[1:]]
        benchmark = "/".join(parts)
        summaries[benchmark] = {
            "benchmark": benchmark,
            "sample_count": len(per_episode),
            "median_episode_ns": statistics.median(per_episode)
            if per_episode
            else 0.0,
            "p95_episode_ns": percentile(per_episode, 0.95),
        }
    return summaries


def criterion_samples(target_dir: Path) -> dict[str, dict[str, Any]]:
    return criterion_samples_from_root(target_dir / "criterion")


ProbeKey = tuple[str, int, int, int]


def probe_key(sample: dict[str, Any]) -> ProbeKey:
    lane = sample.get("lane")
    if not isinstance(lane, str) or not lane:
        raise ValueError("Gate B probe lane must be a non-empty string")
    values = {
        "instances": sample.get("instances"),
        "retained_history": sample.get("retained_history"),
        "next_epoch": sample.get("next_epoch"),
        "turns": sample.get("turns"),
    }
    for field, value in values.items():
        if not isinstance(value, int) or isinstance(value, bool):
            raise ValueError(f"Gate B probe {field} must be an exact JSON integer")
    if values["turns"] != EPISODE_LENGTH:
        raise ValueError(f"Gate B probe turns must equal {EPISODE_LENGTH}")
    return (
        lane,
        values["instances"],
        values["retained_history"],
        values["next_epoch"],
    )


def expected_b2_probe_keys() -> tuple[set[ProbeKey], set[ProbeKey]]:
    legacy = {
        *(("mech-legacy-atomic", instances, 0, 1) for instances in SCALED_INSTANCES),
        ("mech-legacy-atomic-full-write", 1, 0, 1),
    }
    resident = {
        *(("mech-resident-kernel", instances, 0, 1) for instances in SCALED_INSTANCES),
        ("mech-resident-kernel-full-write", 1, 0, 1),
        ("mech-resident-scheduled", 1, 0, 1),
        ("mech-resident-turn", 1, 0, 1),
        ("mech-resident-turn", 1, 1_000, 1),
        ("mech-resident-turn", 1, 100_000, 1),
        ("mech-resident-turn", 1, 0, 1_000_000_001),
        ("mech-resident-turn-full-write", 1, 0, 1),
        ("mech-resident-artifact-source", 1, 0, 1),
        ("mech-resident-artifact-source", 1, 1_000, 1),
        ("mech-resident-artifact-source", 1, 100_000, 1),
        ("mech-resident-artifact-source", 1, 0, 1_000_000_001),
        ("mech-resident-artifact-bytecode", 1, 0, 1),
        ("mech-resident-artifact-kernel-source", 1, 0, 1),
        ("mech-resident-artifact-kernel-bytecode", 1, 0, 1),
    }
    raw = {
        *(("rust-kernel", instances, 0, 1) for instances in SCALED_INSTANCES),
        *(("rust-epoch", instances, 0, 1) for instances in SCALED_INSTANCES),
        ("rust-epoch-full-write", 1, 0, 1),
    }
    return raw | legacy | resident, legacy | resident


def parse_probe_samples(output: str) -> dict[ProbeKey, dict[str, Any]]:
    samples: dict[ProbeKey, dict[str, Any]] = {}
    for line in output.splitlines():
        marker = line.find(SAMPLE_PREFIX)
        if marker < 0:
            continue
        sample = json.loads(line[marker + len(SAMPLE_PREFIX) :])
        key = probe_key(sample)
        if key in samples:
            # Criterion may emit the same deterministic probe more than once as
            # it calibrates iteration counts. Never let a later record replace
            # evidence from an already-seen identity.
            if samples[key] != sample:
                raise ValueError(f"conflicting duplicate Gate B probe {key}")
            continue
        samples[key] = sample
    return samples


def merge_structural_probes(
    timed: dict[ProbeKey, dict[str, Any]],
    structural: dict[ProbeKey, dict[str, Any]],
) -> dict[ProbeKey, dict[str, Any]]:
    merged = {key: value.copy() for key, value in timed.items()}
    legacy = {
        *(("mech-legacy-atomic", instances, 0, 1) for instances in SCALED_INSTANCES),
        ("mech-legacy-atomic-full-write", 1, 0, 1),
    }
    resident = {
        *(("mech-resident-kernel", instances, 0, 1) for instances in SCALED_INSTANCES),
        ("mech-resident-kernel-full-write", 1, 0, 1),
    }
    if any(key[0] == "mech-resident-turn" for key in timed):
        resident.update(
            {
                ("mech-resident-scheduled", 1, 0, 1),
                ("mech-resident-turn", 1, 0, 1),
                ("mech-resident-turn", 1, 1_000, 1),
                ("mech-resident-turn", 1, 100_000, 1),
                ("mech-resident-turn", 1, 0, 1_000_000_001),
                ("mech-resident-turn-full-write", 1, 0, 1),
                ("mech-resident-artifact-source", 1, 0, 1),
                ("mech-resident-artifact-source", 1, 1_000, 1),
                ("mech-resident-artifact-source", 1, 100_000, 1),
                ("mech-resident-artifact-source", 1, 0, 1_000_000_001),
                ("mech-resident-artifact-bytecode", 1, 0, 1),
                ("mech-resident-artifact-kernel-source", 1, 0, 1),
                ("mech-resident-artifact-kernel-bytecode", 1, 0, 1),
            }
        )
    for key in legacy | resident:
        if key not in merged:
            raise ValueError(f"missing timed structural probe {key}")
        if key not in structural:
            raise ValueError(f"missing untimed structural probe {key}")
        fields = (
            ("commit_runtime_call_count", "legacy_journal_capture_count")
            if key in legacy
            else STRUCTURAL_FIELDS
        )
        for field in fields:
            merged[key][field] = structural[key].get(field)
    return merged


def controlled_environment(source: dict[str, str]) -> dict[str, str]:
    environment = source.copy()
    for variable in THREAD_VARIABLES:
        environment[variable] = "1"
    return environment


def hardware_description(machine_label: str | None = None) -> str:
    if machine_label and machine_label.strip():
        return machine_label.strip()
    if sys.platform == "darwin":
        try:
            overview = command_output(
                ["system_profiler", "SPHardwareDataType", "-detailLevel", "mini"]
            )
            fields = {}
            for line in overview.splitlines():
                key, separator, value = line.strip().partition(":")
                if separator and key in {"Model Name", "Model Identifier", "Chip"}:
                    fields[key] = value.strip()
            parts = [
                fields.get("Model Name"),
                fields.get("Model Identifier"),
                fields.get("Chip"),
            ]
            specific = [part for part in parts if part]
            if specific:
                return ", ".join(specific)
        except (OSError, subprocess.CalledProcessError):
            pass
    processor = platform.processor().strip()
    generic = {
        "",
        "arm",
        "arm64",
        "aarch64",
        "x86_64",
        "amd64",
        "i386",
        "i686",
        platform.machine().strip().lower(),
    }
    if processor.lower() not in generic:
        return processor
    raise ValueError(
        "a specific CPU/model identity is unavailable; pass --machine-label "
        "with the controlled machine's stable model label"
    )


def _worker_error(process: subprocess.Popen[str], reason: str) -> RuntimeError:
    stderr = ""
    if process.stderr is not None:
        stderr = process.stderr.read().strip()
    detail = f": {stderr}" if stderr else ""
    return RuntimeError(f"persistent NumPy worker {reason}{detail}")


def numpy_worker_command(python: str) -> tuple[list[str], Path]:
    python_path = Path(python)
    if not python_path.is_absolute():
        raise ValueError("the Gate B NumPy interpreter path must be absolute")
    module = command_output(
        [
            str(python_path),
            "-I",
            "-c",
            (
                "import importlib.util; "
                "spec = importlib.util.find_spec('numpy'); "
                "assert spec is not None and spec.origin is not None; "
                "print(spec.origin)"
            ),
        ]
    )
    expected_module = Path(module).resolve()
    return (
        [
            str(python_path),
            "-I",
            str(ROOT / "benchmarks/runtime/gate-b/numpy/ekf_v1.py"),
            "--expected-numpy-module",
            str(expected_module),
        ],
        expected_module,
    )


def run_numpy_controls(
    python: str, samples: int, environment: dict[str, str]
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    command, expected_module = numpy_worker_command(python)
    process = subprocess.Popen(
        command,
        cwd=ROOT,
        env=environment,
        text=True,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        if process.stdout is None or process.stdin is None:
            raise _worker_error(process, "did not expose its protocol pipes")
        ready_line = process.stdout.readline()
        if not ready_line:
            raise _worker_error(process, "exited before its ready message")
        ready = json.loads(ready_line)
        if ready.get("type") != "ready" or ready.get("protocol") != "gate-b-numpy-v1":
            raise RuntimeError(f"unexpected persistent NumPy ready message: {ready}")
        if ready.get("python_isolated") is not True:
            raise RuntimeError("persistent NumPy worker is not running in isolated mode")
        if Path(str(ready.get("numpy_module_path", ""))).resolve() != expected_module:
            raise RuntimeError(
                "persistent NumPy worker imported outside the authenticated environment"
            )
        results = []
        for instances in SCALED_INSTANCES:
            request = {
                "command": "benchmark",
                "instances": instances,
                "samples": samples,
            }
            process.stdin.write(json.dumps(request) + "\n")
            process.stdin.flush()
            response_line = process.stdout.readline()
            if not response_line:
                raise _worker_error(process, f"exited during the {instances}-instance lane")
            response = json.loads(response_line)
            if response.get("type") == "error":
                raise RuntimeError(
                    "persistent NumPy worker rejected the "
                    f"{instances}-instance lane: {response.get('message')}"
                )
            if response.get("type") != "benchmark-result":
                raise RuntimeError(f"unexpected persistent NumPy result: {response}")
            results.append(response)
        process.stdin.write(json.dumps({"command": "quit"}) + "\n")
        process.stdin.flush()
        bye = json.loads(process.stdout.readline())
        if bye.get("type") != "bye":
            raise RuntimeError(f"unexpected persistent NumPy shutdown: {bye}")
        if process.wait(timeout=10) != 0:
            raise _worker_error(process, "exited unsuccessfully")
        return ready, results
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


STRUCTURAL_FIELDS = (
    "candidate_seed_bytes",
    "candidate_written_bytes",
    "published_buffer_copy_bytes",
    "publication_store_count",
    "receipt_bytes",
    "commit_runtime_call_count",
    "legacy_journal_capture_count",
    "abort_output_hash",
    "dirty_node_count",
    "record_preparation_count",
    "record_append_count",
    "records_retained_before_timing",
    "records_appended",
    "ledger_records_inspected",
    "post_publication_append_infallible",
)


def lane_record(
    lane: str,
    instances: int,
    sample_count: int,
    median_episode_ns: float,
    p95_episode_ns: float,
    probe: dict[str, Any],
    reference_hash: str,
) -> dict[str, Any]:
    turns = int(probe["turns"])
    allocations = probe.get("allocation_count")
    allocated_bytes = probe.get("allocated_bytes")
    return {
        "lane": lane,
        "instances": instances,
        "retained_history": int(probe.get("retained_history", 0)),
        "next_epoch": int(probe.get("next_epoch", 1)),
        "sample_count": sample_count,
        "turns_per_sample": turns,
        "timing": {
            "median_ns_per_turn": median_episode_ns / turns,
            "p95_ns_per_turn": p95_episode_ns / turns,
        },
        "allocation": {
            "allocations_per_turn": (
                float(allocations) / turns if allocations is not None else None
            ),
            "allocated_bytes_per_turn": (
                float(allocated_bytes) / turns
                if allocated_bytes is not None
                else None
            ),
            "episode_allocation_count": allocations,
            "episode_allocated_bytes": allocated_bytes,
        },
        "correctness": bool(probe["correctness"]),
        "quantized_state_hash": probe["quantized_state_hash"],
        "reference_quantized_state_hash": reference_hash,
        "structural": {field: probe.get(field) for field in STRUCTURAL_FIELDS},
    }


def assemble_lanes(
    criterion: dict[str, dict[str, Any]],
    probes: dict[ProbeKey, dict[str, Any]],
    numpy_results: list[dict[str, Any]],
    reference_hash: str,
) -> list[dict[str, Any]]:
    lanes = []
    rust_lanes = (
        ("rust-kernel", "gate_b/rust-kernel/{instances}"),
        ("rust-epoch", "gate_b/rust-epoch/{instances}"),
        ("mech-resident-kernel", "gate_b/mech-resident-kernel/{instances}"),
        ("mech-legacy-atomic", "gate_b/mech-legacy-atomic/{instances}"),
    )
    for lane, benchmark_template in rust_lanes:
        for instances in SCALED_INSTANCES:
            benchmark = benchmark_template.format(instances=instances)
            if benchmark not in criterion:
                raise ValueError(f"missing Criterion result {benchmark}")
            key = (lane, instances, 0, 1)
            if key not in probes:
                raise ValueError(f"missing structural probe {lane}/{instances}")
            timing = criterion[benchmark]
            lanes.append(
                lane_record(
                    lane,
                    instances,
                    int(timing["sample_count"]),
                    float(timing["median_episode_ns"]),
                    float(timing["p95_episode_ns"]),
                    probes[key],
                    reference_hash,
                )
            )
    for lane, benchmark in (
        ("rust-epoch-full-write", "gate_b/full-write/rust-epoch"),
        ("mech-legacy-atomic-full-write", "gate_b/full-write/mech-legacy-atomic"),
        ("mech-resident-kernel-full-write", "gate_b/full-write/mech-resident-kernel"),
    ):
        if benchmark not in criterion:
            raise ValueError(f"missing Criterion result {benchmark}")
        key = (lane, 1, 0, 1)
        if key not in probes:
            raise ValueError(f"missing structural probe {lane}/1")
        timing = criterion[benchmark]
        lanes.append(
            lane_record(
                lane,
                1,
                int(timing["sample_count"]),
                float(timing["median_episode_ns"]),
                float(timing["p95_episode_ns"]),
                probes[key],
                probes[key]["quantized_state_hash"],
            )
        )
    for lane, benchmark, history, next_epoch in (
        (
            "mech-resident-artifact-source",
            "gate_b/mech-resident-artifact/source-history-0-low-epoch",
            0,
            1,
        ),
        (
            "mech-resident-artifact-source",
            "gate_b/mech-resident-artifact/source-history-1000-low-epoch",
            1_000,
            1,
        ),
        (
            "mech-resident-artifact-source",
            "gate_b/mech-resident-artifact/source-history-100000-low-epoch",
            100_000,
            1,
        ),
        (
            "mech-resident-artifact-source",
            "gate_b/mech-resident-artifact/source-history-0-high-epoch",
            0,
            1_000_000_001,
        ),
        (
            "mech-resident-artifact-bytecode",
            "gate_b/mech-resident-artifact/bytecode-history-0-low-epoch",
            0,
            1,
        ),
        (
            "mech-resident-artifact-kernel-source",
            "gate_b/mech-resident-artifact-kernel/source",
            0,
            1,
        ),
        (
            "mech-resident-artifact-kernel-bytecode",
            "gate_b/mech-resident-artifact-kernel/bytecode",
            0,
            1,
        ),
    ):
        if benchmark not in criterion:
            raise ValueError(f"missing Criterion result {benchmark}")
        key = (lane, 1, history, next_epoch)
        if key not in probes:
            raise ValueError(f"missing structural probe {lane}/1")
        timing = criterion[benchmark]
        lanes.append(
            lane_record(
                lane,
                1,
                int(timing["sample_count"]),
                float(timing["median_episode_ns"]),
                float(timing["p95_episode_ns"]),
                probes[key],
                reference_hash,
            )
        )
        lanes[-1]["retained_history"] = history
        lanes[-1]["next_epoch"] = next_epoch
    if ("mech-resident-turn", 1, 0, 1) in probes:
        for lane, benchmark, history, next_epoch in (
            (
                "mech-resident-scheduled",
                "gate_b/mech-resident-scheduled/1",
                0,
                1,
            ),
            (
                "mech-resident-turn",
                "gate_b/mech-resident-turn/history-0-low-epoch",
                0,
                1,
            ),
            (
                "mech-resident-turn",
                "gate_b/mech-resident-turn/history-1000-low-epoch",
                1_000,
                1,
            ),
            (
                "mech-resident-turn",
                "gate_b/mech-resident-turn/history-100000-low-epoch",
                100_000,
                1,
            ),
            (
                "mech-resident-turn",
                "gate_b/mech-resident-turn/history-0-high-epoch",
                0,
                1_000_000_001,
            ),
            (
                "mech-resident-turn-full-write",
                "gate_b/full-write/mech-resident-turn",
                0,
                1,
            ),
        ):
            if benchmark not in criterion:
                raise ValueError(f"missing Criterion result {benchmark}")
            key = (lane, 1, history, next_epoch)
            if key not in probes:
                raise ValueError(f"missing structural probe {key}")
            timing = criterion[benchmark]
            lanes.append(
                lane_record(
                    lane,
                    1,
                    int(timing["sample_count"]),
                    float(timing["median_episode_ns"]),
                    float(timing["p95_episode_ns"]),
                    probes[key],
                    (
                        probes[key]["quantized_state_hash"]
                        if lane == "mech-resident-turn-full-write"
                        else reference_hash
                    ),
                )
            )
    for result in numpy_results:
        durations = [float(value) for value in result["samples_ns"]]
        probe = {
            **result,
            "allocation_count": None,
            "allocated_bytes": None,
        }
        lanes.append(
            lane_record(
                "numpy-persistent",
                int(result["instances"]),
                len(durations),
                statistics.median(durations),
                percentile(durations, 0.95),
                probe,
                result["reference_quantized_state_hash"],
            )
        )
    return sorted(
        lanes,
        key=lambda lane: (
            lane["lane"],
            lane["instances"],
            lane["retained_history"],
            lane["next_epoch"],
        ),
    )


def primary_median(lanes: list[dict[str, Any]], name: str) -> float:
    matches = [
        lane
        for lane in lanes
        if lane["lane"] == name
        and lane["instances"] == 1
        and lane.get("retained_history", 0) == 0
        and lane.get("next_epoch", 1) == 1
    ]
    if len(matches) != 1:
        raise ValueError(f"expected exactly one primary {name} lane")
    return float(matches[0]["timing"]["median_ns_per_turn"])


def legacy_denominator(lanes: list[dict[str, Any]]) -> dict[str, Any]:
    legacy = primary_median(lanes, "mech-legacy-atomic")
    raw_epoch = primary_median(lanes, "rust-epoch")
    denominator = legacy - raw_epoch
    return {
        "mech_legacy_atomic_ns_per_turn": legacy,
        "rust_epoch_ns_per_turn": raw_epoch,
        "legacy_denominator_ns_per_turn": denominator,
        "positive": denominator > 0.0,
    }


def b1_progression(lanes: list[dict[str, Any]]) -> dict[str, Any]:
    resident = primary_median(lanes, "mech-resident-kernel")
    rust_kernel = primary_median(lanes, "rust-kernel")
    rust_epoch = primary_median(lanes, "rust-epoch")
    multiplier = 1.05
    limit = multiplier * rust_epoch
    return {
        "resident_kernel_ns_per_turn": resident,
        "rust_kernel_ns_per_turn": rust_kernel,
        "rust_epoch_ns_per_turn": rust_epoch,
        "resident_kernel_ratio": resident / rust_kernel,
        "resident_kernel_vs_raw_epoch": resident / rust_epoch,
        "limit_multiplier": multiplier,
        "limit_ns_per_turn": limit,
        "passed": resident <= limit,
    }


def selected_lane(
    lanes: list[dict[str, Any]],
    name: str,
    *,
    history: int = 0,
    next_epoch: int = 1,
) -> dict[str, Any]:
    matches = [
        lane
        for lane in lanes
        if lane["lane"] == name
        and lane["instances"] == 1
        and lane.get("retained_history", 0) == history
        and lane.get("next_epoch", 1) == next_epoch
    ]
    if len(matches) != 1:
        raise ValueError(
            f"expected exactly one {name} lane for history={history}, "
            f"next_epoch={next_epoch}"
        )
    return matches[0]


def b2_decision(lanes: list[dict[str, Any]]) -> dict[str, Any]:
    legacy = primary_median(lanes, "mech-legacy-atomic")
    raw_epoch = primary_median(lanes, "rust-epoch")
    numpy = primary_median(lanes, "numpy-persistent")
    kernel = primary_median(lanes, "mech-resident-kernel")
    scheduled = primary_median(lanes, "mech-resident-scheduled")
    turn = selected_lane(lanes, "mech-resident-turn")
    turn_median = float(turn["timing"]["median_ns_per_turn"])
    turn_p95 = float(turn["timing"]["p95_ns_per_turn"])
    history_100k = selected_lane(
        lanes, "mech-resident-turn", history=100_000
    )
    history_1k = selected_lane(lanes, "mech-resident-turn", history=1_000)
    high_epoch = selected_lane(
        lanes, "mech-resident-turn", next_epoch=1_000_000_001
    )
    full_write = selected_lane(lanes, "mech-resident-turn-full-write")
    resident_lanes = [
        lane
        for lane in lanes
        if lane["lane"]
        in {
            "mech-resident-scheduled",
            "mech-resident-turn",
            "mech-resident-turn-full-write",
        }
    ]
    legacy_gap_closure = (legacy - turn_median) / (legacy - raw_epoch)
    raw_epoch_ratio = turn_median / raw_epoch
    executor_tax = turn_median - kernel
    scheduler_tax = scheduled - kernel
    recording_tax = turn_median - scheduled
    numpy_ratio = turn_median / numpy
    tail_ratio = turn_p95 / turn_median
    history_ratio = (
        float(history_100k["timing"]["median_ns_per_turn"]) / turn_median
    )
    history_1k_ratio = (
        float(history_1k["timing"]["median_ns_per_turn"]) / turn_median
    )
    high_epoch_ratio = (
        float(high_epoch["timing"]["median_ns_per_turn"]) / turn_median
    )

    structural = turn["structural"]
    full_structural = full_write["structural"]
    hard_gates = {
        "correctness": all(
            lane["correctness"]
            and lane["quantized_state_hash"]
            == lane["reference_quantized_state_hash"]
            for lane in resident_lanes
        ),
        "zero_allocation": all(
            lane["allocation"]["episode_allocation_count"] == 0
            for lane in resident_lanes
        ),
        "constant_publication": (
            structural["publication_store_count"] == 1
            and full_structural["publication_store_count"] == 1
        ),
        "no_full_clone": (
            structural["candidate_seed_bytes"] == 0
            and structural["published_buffer_copy_bytes"] == 0
            and full_structural["candidate_seed_bytes"] == 0
            and full_structural["candidate_written_bytes"] == 32_768
            and full_structural["published_buffer_copy_bytes"] == 0
        ),
        "history_independent": (
            history_1k_ratio <= 1.05
            and history_ratio <= 1.05
            and high_epoch_ratio <= 1.05
            and all(
                lane["structural"]["ledger_records_inspected"] == 0
                for lane in resident_lanes
                if lane["lane"] == "mech-resident-turn"
            )
        ),
        "legacy_gap_closure": legacy_gap_closure >= 0.80,
        "raw_epoch_ratio": raw_epoch_ratio <= 1.25,
        "executor_tax": executor_tax <= (1.25 * raw_epoch - kernel),
        "tail_stability": tail_ratio <= 1.50,
        "post_publication_append_infallible": (
            structural["post_publication_append_infallible"] is True
            and full_structural["post_publication_append_infallible"] is True
        ),
    }
    numpy_target = turn_median <= 1.10 * numpy
    hard_pass = all(hard_gates.values())
    if hard_pass and numpy_target:
        decision = "Pass"
        attribution = None
    elif hard_pass:
        decision = "ConditionalPass"
        attribution = "kernel selection, numerical backend, or data layout"
    else:
        decision = "Fail"
        attribution = None
    return {
        "legacy_gap_closure": legacy_gap_closure,
        "raw_epoch_ratio": raw_epoch_ratio,
        "executor_tax_ns": executor_tax,
        "scheduler_tax_ns": scheduler_tax,
        "recording_tax_ns": recording_tax,
        "numpy_ratio": numpy_ratio,
        "tail_ratio": tail_ratio,
        "history_1k_over_history_0_median_ratio": history_1k_ratio,
        "history_100k_over_history_0_median_ratio": history_ratio,
        "high_epoch_over_low_epoch_median_ratio": high_epoch_ratio,
        "hard_gates": hard_gates,
        "numpy_target": numpy_target,
        "decision": decision,
        "conditional_attribution": attribution,
    }


def d1_decision(lanes: list[dict[str, Any]]) -> dict[str, Any]:
    legacy = primary_median(lanes, "mech-legacy-atomic")
    raw_epoch = primary_median(lanes, "rust-epoch")
    gate_b_turn = selected_lane(lanes, "mech-resident-turn")
    source = selected_lane(lanes, "mech-resident-artifact-source")
    bytecode = selected_lane(lanes, "mech-resident-artifact-bytecode")
    history_1k = selected_lane(lanes, "mech-resident-artifact-source", history=1_000)
    history_100k = selected_lane(lanes, "mech-resident-artifact-source", history=100_000)
    high_epoch = selected_lane(
        lanes, "mech-resident-artifact-source", next_epoch=1_000_000_001
    )
    kernel_source = selected_lane(lanes, "mech-resident-artifact-kernel-source")
    kernel_bytecode = selected_lane(lanes, "mech-resident-artifact-kernel-bytecode")
    source_median = float(source["timing"]["median_ns_per_turn"])
    bytecode_median = float(bytecode["timing"]["median_ns_per_turn"])
    gate_b_median = float(gate_b_turn["timing"]["median_ns_per_turn"])
    source_bytecode_ratio = max(source_median, bytecode_median) / min(
        source_median, bytecode_median
    )
    complete_control_ratio = source_median / gate_b_median
    raw_epoch_ratio = source_median / raw_epoch
    legacy_gap_closure = (legacy - source_median) / (legacy - raw_epoch)
    history_1k_ratio = (
        float(history_1k["timing"]["median_ns_per_turn"]) / source_median
    )
    history_100k_ratio = (
        float(history_100k["timing"]["median_ns_per_turn"]) / source_median
    )
    high_epoch_ratio = (
        float(high_epoch["timing"]["median_ns_per_turn"]) / source_median
    )
    complete_lanes = [source, bytecode, history_1k, history_100k, high_epoch]
    structural = source["structural"]
    structural_equivalent = all(
        lane["structural"][field] == structural[field]
        for lane in complete_lanes
        for field in (
            "candidate_seed_bytes",
            "candidate_written_bytes",
            "published_buffer_copy_bytes",
            "publication_store_count",
            "record_preparation_count",
            "record_append_count",
            "records_appended",
            "ledger_records_inspected",
            "post_publication_append_infallible",
            "commit_runtime_call_count",
            "legacy_journal_capture_count",
        )
    )
    hard_gates = {
        "correctness": all(
            lane["correctness"]
            and lane["quantized_state_hash"] == lane["reference_quantized_state_hash"]
            for lane in complete_lanes
        ),
        "source_bytecode_equivalence": source_bytecode_ratio <= 1.03,
        "complete_turn_control_ratio": complete_control_ratio <= 1.20,
        "raw_epoch_ratio": raw_epoch_ratio <= 1.50,
        "legacy_gap_closure": legacy_gap_closure >= 0.75,
        "history_independent": history_1k_ratio <= 1.05 and history_100k_ratio <= 1.05,
        "epoch_magnitude_independent": high_epoch_ratio <= 1.05,
        "zero_allocation": all(
            lane["allocation"]["episode_allocation_count"] == 0
            for lane in complete_lanes
        ),
        "candidate_contract": (
            structural["candidate_seed_bytes"] == 0
            and structural["candidate_written_bytes"] == 96
            and structural["published_buffer_copy_bytes"] == 0
            and structural["publication_store_count"] == 1
        ),
        "recording_contract": (
            structural_equivalent
            and structural["record_preparation_count"] == 1
            and structural["record_append_count"] == 1
            and structural["records_appended"] == EPISODE_LENGTH
            and structural["ledger_records_inspected"] == 0
            and structural["post_publication_append_infallible"] is True
        ),
        "legacy_boundaries_unused": (
            structural["commit_runtime_call_count"] == 0
            and structural["legacy_journal_capture_count"] == 0
        ),
    }
    return {
        "legacy_gap_closure": legacy_gap_closure,
        "raw_epoch_ratio": raw_epoch_ratio,
        "source_bytecode_ratio": source_bytecode_ratio,
        "artifact_complete_turn_ratio": complete_control_ratio,
        "executor_tax_ns": source_median - gate_b_median,
        "history_1k_over_history_0_median_ratio": history_1k_ratio,
        "history_100k_over_history_0_median_ratio": history_100k_ratio,
        "high_epoch_over_low_epoch_median_ratio": high_epoch_ratio,
        "kernel_source_ns_per_turn": float(
            kernel_source["timing"]["median_ns_per_turn"]
        ),
        "kernel_bytecode_ns_per_turn": float(
            kernel_bytecode["timing"]["median_ns_per_turn"]
        ),
        "hard_gates": hard_gates,
        "decision": "Pass" if all(hard_gates.values()) else "Fail",
    }


def worktree_changes() -> str:
    return command_output(["git", "status", "--porcelain=v1", "--untracked-files=all"])


def frozen_base_error(
    commit: str, branch: str, phase: str | None = None
) -> str | None:
    if phase == "B2-resident-turn":
        expected_base = B2_EVIDENCE_FLOOR
    else:
        expected_base = {
            FROZEN_B0_BRANCH: FROZEN_BASE,
            FROZEN_B1_BRANCH: FROZEN_B1_BASE,
            B2_BRANCH: FROZEN_B2_BASE,
        }.get(branch)
    if expected_base is None:
        return f"Gate B controls cannot run on unapproved branch {branch}"
    merge_base = command_output(["git", "merge-base", commit, expected_base])
    if merge_base != expected_base:
        return (
            f"Gate B commit {commit} is not based on frozen base "
            f"{expected_base}; merge-base is {merge_base}"
        )
    return None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        help="write the report here; otherwise print it to stdout",
    )
    parser.add_argument(
        "--raw-output",
        type=Path,
        help="write combined Cargo/Criterion output here (defaults under target)",
    )
    parser.add_argument(
        "--raw-structural-output",
        type=Path,
        help="write the untimed probe-run output here (defaults under target)",
    )
    parser.add_argument(
        "--raw-numpy-output",
        type=Path,
        help="write the exact persistent NumPy samples here",
    )
    parser.add_argument(
        "--python",
        default=sys.executable,
        help="Python executable containing NumPy",
    )
    parser.add_argument(
        "--sample-size", type=int, default=SAMPLE_PROTOCOL["criterion_sample_size"]
    )
    parser.add_argument(
        "--warm-up-time", type=float, default=SAMPLE_PROTOCOL["warm_up_seconds"]
    )
    parser.add_argument(
        "--measurement-time",
        type=float,
        default=SAMPLE_PROTOCOL["measurement_seconds"],
    )
    parser.add_argument(
        "--phase",
        choices=("B2-resident-turn",),
        help="refresh the complete B2 evidence lane set on a descendant branch",
    )
    parser.add_argument(
        "--machine-label",
        help="stable controlled-machine model label when automatic detection is unavailable",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    requested_protocol = {
        "criterion_sample_size": args.sample_size,
        "numpy_sample_size": args.sample_size,
        "warm_up_seconds": args.warm_up_time,
        "measurement_seconds": args.measurement_time,
        "turns_per_sample": EPISODE_LENGTH,
        "fixture_setup_included_in_timing": False,
        "correctness_included_in_timing": False,
        "profile": "release",
    }
    if requested_protocol != SAMPLE_PROTOCOL:
        print(
            "Gate B sample protocol is frozen at 10 Criterion samples, 10 NumPy "
            "samples, 1 second warm-up, and 3 seconds measurement",
            file=sys.stderr,
        )
        return 2
    try:
        hardware = hardware_description(args.machine_label)
    except ValueError as error:
        print(str(error), file=sys.stderr)
        return 2
    dirty = worktree_changes()
    if dirty:
        print(
            "refusing to attribute Gate B benchmark results to a dirty worktree:\n"
            f"{dirty}",
            file=sys.stderr,
        )
        return 2

    commit = command_output(["git", "rev-parse", "HEAD"])
    branch = command_output(["git", "symbolic-ref", "--short", "HEAD"])
    base_error = frozen_base_error(commit, branch, args.phase)
    if base_error:
        print(base_error, file=sys.stderr)
        return 2
    environment = controlled_environment(os.environ)
    target_dir = cargo_target_directory(environment)
    raw_output = args.raw_output or (
        target_dir / "gate-b-benchmark-runs" / commit / "b0-controls.log"
    )
    if not raw_output.is_absolute():
        raw_output = (ROOT / raw_output).resolve()
    raw_output.parent.mkdir(parents=True, exist_ok=True)
    raw_structural_output = args.raw_structural_output or (
        target_dir / "gate-b-benchmark-runs" / commit / "b0-structural.log"
    )
    if not raw_structural_output.is_absolute():
        raw_structural_output = (ROOT / raw_structural_output).resolve()
    raw_structural_output.parent.mkdir(parents=True, exist_ok=True)
    raw_numpy_output = args.raw_numpy_output
    if raw_numpy_output is not None:
        if not raw_numpy_output.is_absolute():
            raw_numpy_output = (ROOT / raw_numpy_output).resolve()
        raw_numpy_output.parent.mkdir(parents=True, exist_ok=True)

    command = [
        "cargo",
        "bench",
        "-p",
        "mech-runtime",
        "--bench",
        "resident_ekf",
        "--features",
        "source_default,runtime_bench_gate_b",
        "--",
        "--noplot",
        "--sample-size",
        str(args.sample_size),
        "--warm-up-time",
        str(args.warm_up_time),
        "--measurement-time",
        str(args.measurement_time),
        "--nresamples",
        "10000",
    ]
    clear_gate_b_criterion_results(target_dir)
    process = subprocess.run(
        command,
        cwd=ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    raw_output.write_text(process.stdout, encoding="utf-8")
    if process.returncode != 0:
        sys.stderr.write(process.stdout)
        return process.returncode

    structural_command = [
        "cargo",
        "bench",
        "-p",
        "mech-runtime",
        "--bench",
        "resident_ekf",
        "--features",
        "source_default,runtime_bench_gate_b,runtime_bench_probes",
        "--",
        "--noplot",
    ]
    structural_environment = environment.copy()
    structural_environment["MECH_GATE_B_STRUCTURAL_ONLY"] = "1"
    structural_process = subprocess.run(
        structural_command,
        cwd=ROOT,
        env=structural_environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    raw_structural_output.write_text(
        structural_process.stdout, encoding="utf-8"
    )
    if structural_process.returncode != 0:
        sys.stderr.write(structural_process.stdout)
        return structural_process.returncode

    try:
        ready, numpy_results = run_numpy_controls(
            args.python, args.sample_size, environment
        )
        if raw_numpy_output is not None:
            raw_numpy_output.write_text(
                json.dumps(
                    {"ready": ready, "results": numpy_results},
                    indent=2,
                    sort_keys=True,
                )
                + "\n",
                encoding="utf-8",
            )
        manifest = json.loads(
            (ROOT / "benchmarks/runtime/gate-b/ekf-v1.json").read_text(
                encoding="utf-8"
            )
        )
        probes = merge_structural_probes(
            parse_probe_samples(process.stdout),
            parse_probe_samples(structural_process.stdout),
        )
        lanes = assemble_lanes(
            criterion_samples(target_dir),
            probes,
            numpy_results,
            manifest["reference"]["quantized_trajectory_sha256"],
        )
        derived = legacy_denominator(lanes)
    except (OSError, ValueError, RuntimeError, KeyError, json.JSONDecodeError) as error:
        print(f"Gate B result assembly failed: {error}", file=sys.stderr)
        return 2

    qualification_decision = "Pass"
    advisory_performance_failures: dict[str, list[str]] = {}
    summary = {
        "schema_version": 1,
        "gate": "B",
        "phase": args.phase
        or (
            "B0-controls"
            if branch == FROZEN_B0_BRANCH
            else "B1-resident-kernel"
            if branch == FROZEN_B1_BRANCH
            else "B2-resident-turn"
        ),
        "git_commit": commit,
        "git_branch": branch,
        "machine": {
            "identity": hardware,
            "os": platform.platform(),
            "architecture": platform.machine(),
        },
        "toolchain": {
            "rustc": command_output(["rustc", "-Vv"]),
            "RUSTFLAGS": os.environ.get("RUSTFLAGS", ""),
            "CARGO_ENCODED_RUSTFLAGS": os.environ.get(
                "CARGO_ENCODED_RUSTFLAGS", ""
            ),
            "python": ready["python"],
            "python_executable": args.python,
            "numpy": ready["numpy"],
            "numpy_config": ready["numpy_config"],
            "blas_lapack_provider": ready["blas_lapack_provider"],
        },
        "thread_environment": {
            variable: environment[variable] for variable in THREAD_VARIABLES
        },
        "trace": {
            "sha256": manifest["trace"]["sha256"],
            "file": manifest["trace"]["file"],
        },
        "workload": {
            "version": manifest["workload"],
            "episode_length": EPISODE_LENGTH,
            "scaled_instances": list(SCALED_INSTANCES),
            "scalar": "f64",
            "matrix_storage": "column-major",
        },
        "sample_protocol": SAMPLE_PROTOCOL,
        "benchmark_arguments": command,
        "structural_probe_arguments": structural_command,
        "raw_criterion_directory": str(target_dir / "criterion"),
        "raw_output": str(raw_output),
        "raw_structural_probe_output": str(raw_structural_output),
        "lanes": lanes,
        "derived": derived,
        "stop_condition": {
            "name": "positive-legacy-denominator",
            "passed": bool(derived["positive"]),
        },
    }
    if branch == FROZEN_B1_BRANCH:
        summary["b1_progression"] = b1_progression(lanes)
    if branch == B2_BRANCH or args.phase == "B2-resident-turn":
        summary["b1_progression"] = b1_progression(lanes)
        summary["b2_decision"] = b2_decision(lanes)
        if any(lane["lane"] == "mech-resident-artifact-source" for lane in lanes):
            summary["d1_decision"] = d1_decision(lanes)
        qualification_decision, advisory_performance_failures = (
            release_qualification(summary)
        )
    rendered = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if args.output:
        output = args.output if args.output.is_absolute() else ROOT / args.output
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)
    if not derived["positive"]:
        print(
            "Gate B B0 stop: T_mech-legacy-atomic - T_rust-epoch is non-positive",
            file=sys.stderr,
        )
        return 3
    if branch == FROZEN_B1_BRANCH and not summary["b1_progression"]["passed"]:
        print(
            "Gate B B1 stop: mech-resident-kernel exceeds 1.05 x rust-epoch",
            file=sys.stderr,
        )
        return 4
    for section, findings in advisory_performance_failures.items():
        if findings:
            print(
                f"Gate B advisory performance findings ({section}): "
                + ", ".join(findings),
                file=sys.stderr,
            )
    if (
        branch == B2_BRANCH or args.phase == "B2-resident-turn"
    ) and qualification_decision == "Fail":
        print(
            "Gate B B2 stop: complete resident turn failed one or more "
            "release-blocking gates",
            file=sys.stderr,
        )
        return 5
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
