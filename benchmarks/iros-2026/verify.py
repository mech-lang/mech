#!/usr/bin/env python3
"""Verify the frozen IROS EKF evidence and representative source set."""

from __future__ import annotations

import hashlib
import json
import re
import statistics
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ARCHIVE = ROOT / "benchmarks/archive/compute/parallel-ekf"


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def chart_values(path: Path) -> dict[str, float]:
    labels: dict[str, str] = {}
    values: dict[str, float] = {}
    for node in ET.parse(path).getroot().iter():
        if not node.tag.endswith("text") or node.text is None:
            continue
        y = node.attrib.get("y")
        css_class = node.attrib.get("class")
        if y is not None and css_class == "label":
            labels[y] = node.text
        elif y is not None and css_class == "value":
            values[y] = float(node.text)
    return {label: values[y] for y, label in labels.items() if y in values}


def post_chart_medians(path: Path) -> dict[str, float]:
    values: dict[str, float] = {}
    pattern = re.compile(r"^(.*?) median: ([0-9.]+) million turns/s$")
    for node in ET.parse(path).getroot().iter():
        if not node.tag.endswith("title") or node.text is None:
            continue
        match = pattern.match(node.text)
        if match:
            values[match.group(1)] = float(match.group(2))
    return values


def close(actual: float, expected: float, tolerance: float = 0.005) -> None:
    if abs(actual - expected) > tolerance:
        raise AssertionError(f"expected {expected}, found {actual}")


def check_distribution(samples: list[float], expected: dict, prefix: str) -> None:
    close(statistics.median(samples), expected[f"{prefix}_million_turns_per_second"], 0.0005)
    observed = expected[f"{prefix}_observed_range_million_turns_per_second"]
    close(min(samples), observed[0], 0.0005)
    close(max(samples), observed[1], 0.0005)


def main() -> None:
    manifest = load_json(HERE / "manifest.json")

    for source in manifest["representative_sources"]:
        path = ROOT / source["path"]
        if not path.is_file():
            raise AssertionError(f"missing representative source: {path}")
        actual = sha256(path)
        if actual != source["sha256"]:
            raise AssertionError(f"source hash changed for {source['path']}: {actual}")

    counts = load_json(ARCHIVE / "source-size-audit/counts.json")
    matched = manifest["matched_checked_comparison"]
    assert counts["ekf.mec"]["normalized_characters"] == matched["mech_normalized_source_characters"]
    assert counts["rust_simd.rs"]["normalized_characters"] == matched["rust_normalized_source_characters"]

    fused = load_json(ARCHIVE / "results/apple-m1-fused-reference-controls-2026-08-31.json")
    mech_samples = fused["rows"]["mech_fused_checked"]["throughput_millions"]
    rust_samples = fused["rows"]["rust_fused_checked"]["throughput_millions"]
    assert mech_samples == matched["mech_samples_million_turns_per_second"]
    assert rust_samples == matched["rust_samples_million_turns_per_second"]
    assert len(mech_samples) == matched["retained_process_runs_per_implementation"]
    assert len(rust_samples) == matched["retained_process_runs_per_implementation"]
    check_distribution(mech_samples, matched, "mech")
    check_distribution(rust_samples, matched, "rust")
    mech = statistics.median(mech_samples)
    rust = statistics.median(rust_samples)

    current = load_json(ROOT / manifest["current_head_verification"]["record"])
    assert current["stable_mech_benchmark"]["result"] == "passed"
    samples = current["rust_simd"]["throughput_million_turns_per_second"]
    current_rust = statistics.median(samples)
    close(
        current_rust,
        manifest["current_head_verification"]["rust_median_million_turns_per_second"],
        0.0005,
    )
    current_range = manifest["current_head_verification"][
        "rust_observed_range_million_turns_per_second"
    ]
    close(min(samples), current_range[0], 0.0005)
    close(max(samples), current_range[1], 0.0005)
    if sha256(ROOT / current["rust_simd"]["source"]) != current["rust_simd"]["source_sha256"]:
        raise AssertionError("current-head Rust verification source hash changed")

    reruns = load_json(ROOT / manifest["same_machine_reruns"]["record"])
    for lane in reruns["reruns"].values():
        if sha256(ROOT / lane["source"]) != lane["source_sha256"]:
            raise AssertionError(f"same-machine rerun source hash changed: {lane['source']}")
        for mode in ("checked", "unchecked"):
            row = lane[mode]
            lane_samples = row["samples_million_turns_per_second"]
            close(statistics.median(lane_samples), row["median_million_turns_per_second"], 0.0005)
            close(min(lane_samples), row["observed_range_million_turns_per_second"][0], 0.0005)
            close(max(lane_samples), row["observed_range_million_turns_per_second"][1], 0.0005)

    dylib = load_json(ROOT / manifest["selected_evidence"]["aot_vs_rust_dylib"])
    dylib_summary = manifest["aot_rust_dylib_comparison"]
    assert dylib["configuration"]["measured_processes_per_library"] == dylib_summary[
        "retained_process_runs_per_implementation"
    ]
    for key in ("rust_library", "common_runner", "simd_math_helper"):
        path = ROOT / dylib["sources"][key]
        if sha256(path) != dylib["sources"][f"{key}_sha256"]:
            raise AssertionError(f"dynamic-library comparison source hash changed: {path}")
    dylib_rows = (
        ("Mech Cranelift AOT", "scalar_aot"),
        ("Mech Cranelift SIMD AOT", "simd_aot"),
        ("Rust cdylib", "rust"),
    )
    for label, prefix in dylib_rows:
        row = dylib["rows"][label]
        throughput = row["throughput_million_ekf_turns_per_second"]
        throughput_samples = throughput["samples"]
        assert len(throughput_samples) == dylib_summary[
            "retained_process_runs_per_implementation"
        ]
        close(statistics.median(throughput_samples), throughput["median"], 0.0000005)
        close(min(throughput_samples), throughput["observed_range"][0], 0.0000005)
        close(max(throughput_samples), throughput["observed_range"][1], 0.0000005)
        close(
            throughput["median"],
            dylib_summary[f"{prefix}_throughput_million_turns_per_second"],
            0.0000005,
        )
        assert throughput["observed_range"] == dylib_summary[
            f"{prefix}_throughput_observed_range"
        ]
        memory = row["maximum_resident_set_bytes"]
        assert statistics.median(memory["samples"]) == memory["median"]
        assert [min(memory["samples"]), max(memory["samples"])] == memory[
            "observed_range"
        ]
        assert memory["median"] == dylib_summary[f"{prefix}_median_peak_rss_bytes"]
        assert row["faults"] == [0] * len(throughput_samples)
    assert dylib["rows"]["Mech Cranelift AOT"]["library_bytes"] == dylib_summary[
        "scalar_aot_library_bytes"
    ]
    assert dylib["rows"]["Mech Cranelift SIMD AOT"]["library_bytes"] == dylib_summary[
        "simd_aot_library_bytes"
    ]
    assert dylib["rows"]["Rust cdylib"]["library_bytes"] == dylib_summary[
        "rust_library_bytes"
    ]
    for prefix in ("scalar_aot", "simd_aot"):
        key = f"{prefix}_maximum_final_state_absolute_error_vs_rust"
        close(dylib["validation"][key], dylib_summary[key], 1.0e-12)
    assert dylib["validation"]["faults"] == 0

    direct_metal = load_json(ROOT / manifest["selected_evidence"]["mech_direct_metal"])
    direct_row = direct_metal["rows"]["Mech direct Metal generated from generic scalar IR"]
    close(
        statistics.median(direct_row["checked"]["samples_million_ekf_turns_per_second"]),
        manifest["chart_assertions"]["checked"][
            "Mech direct Metal, generic IR / per-filter GPU dispatch (current)"
        ],
    )
    close(
        statistics.median(direct_row["unchecked"]["samples_million_ekf_turns_per_second"]),
        manifest["chart_assertions"]["unchecked"][
            "Mech direct Metal, generic IR / per-filter GPU dispatch (current)"
        ],
    )

    cross_samples = {
        "Rust packed SIMD": rust_samples,
        "Mech SIMD/JIT": mech_samples,
        "Julia SIMD.jl": fused["rows"]["julia_fused_checked"]["throughput_millions"],
        "NumPy/Numba": fused["rows"]["numba_fused_checked"]["throughput_millions"],
    }

    runtime = load_json(ROOT / manifest["selected_evidence"]["mech_runtime_backends"])
    runtime_rows = {row["label"]: row for row in runtime["rows"]}
    simd = load_json(ROOT / manifest["selected_evidence"]["mech_simd_one_worker"])
    scalar = load_json(ROOT / manifest["selected_evidence"]["mech_scalar_and_jit"])
    aot = load_json(ROOT / manifest["selected_evidence"]["mech_aot"])
    mech_backend_samples = {
        "Direct Metal GPU": direct_row["checked"]["samples_million_ekf_turns_per_second"],
        "WGPU on Metal": runtime_rows["Mech WGPU GPU, checked"]["samples"],
        "SIMD/JIT CPU · 8 workers": runtime_rows[
            "Mech SIMD/JIT CPU, checked (8 workers)"
        ]["samples"],
        "SIMD/JIT CPU · 1 worker": simd["rows"]["checked"]["throughput_millions"],
        "Cranelift SIMD AOT CPU": dylib["rows"]["Mech Cranelift SIMD AOT"]
        ["throughput_million_ekf_turns_per_second"]["samples"],
        "Cranelift JIT CPU": aot["rows"]["Mech Cranelift JIT CPU, same processes"]
        ["samples_million_ekf_turns_per_second"],
        "Cranelift AOT CPU": aot["rows"]["Mech Cranelift AOT CPU"]
        ["samples_million_ekf_turns_per_second"],
        "Scalar artifact evaluator": [
            sample["scalar_checked_million_ekf_turns_per_second"]
            for sample in scalar["samples"]
        ],
    }

    for chart_name, samples_by_row in (
        ("cross_language", cross_samples),
        ("mech_backends", mech_backend_samples),
    ):
        chart = manifest["post_charts"][chart_name]
        expected = chart["rows_median_million_turns_per_second"]
        if set(samples_by_row) != set(expected):
            raise AssertionError(f"post chart row selection changed: {chart_name}")
        for label, lane_samples in samples_by_row.items():
            close(statistics.median(lane_samples), expected[label], 0.0005)

        path = ROOT / chart["file"]
        root = ET.parse(path).getroot()
        if not any(node.tag.endswith("title") for node in root.iter()):
            raise AssertionError(f"post chart is missing an accessible title: {path.name}")
        if not any(node.tag.endswith("desc") for node in root.iter()):
            raise AssertionError(f"post chart is missing an accessible description: {path.name}")
        chart_medians = post_chart_medians(path)
        if set(chart_medians) != set(expected):
            raise AssertionError(f"rendered post chart rows changed: {chart_name}")
        for label, value in chart_medians.items():
            close(value, expected[label], 0.0005)

    readme = (HERE / "README.md").read_text(encoding="utf-8")
    embedded_figures = re.findall(r"^!\[.*?\]\((.*?)\)$", readme, flags=re.MULTILINE)
    expected_figures = [
        "charts/post-cross-language-comparison.svg",
        "charts/post-mech-backend-stack.svg",
    ]
    if embedded_figures != expected_figures:
        raise AssertionError(f"README must embed exactly the two post charts: {embedded_figures}")

    charts = {
        "checked": ARCHIVE / "charts/parallel-ekf-cross-language-checked.svg",
        "unchecked": ARCHIVE / "charts/parallel-ekf-cross-language-unchecked.svg",
    }
    for mode, path in charts.items():
        rows = chart_values(path)
        for label, expected in manifest["chart_assertions"][mode].items():
            if label not in rows:
                raise AssertionError(f"missing {mode} chart row: {label}")
            close(rows[label], expected)

    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", manifest["integration_base"], "HEAD"],
        cwd=ROOT,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError("current branch is not based on the recorded v0.4 integration head")

    print("IROS EKF evidence verified")
    print(f"  matched checked throughput: Mech {mech:.3f} vs Rust {rust:.3f} M turns/s")
    print(
        "  normalized application source: "
        f"Mech {matched['mech_normalized_source_characters']:,} vs "
        f"Rust {matched['rust_normalized_source_characters']:,} characters"
    )
    print(f"  representative source hashes: {len(manifest['representative_sources'])}")
    print(
        "  matched observed ranges: "
        f"Mech {min(mech_samples):.3f}-{max(mech_samples):.3f}; "
        f"Rust {min(rust_samples):.3f}-{max(rust_samples):.3f} M turns/s"
    )
    print(
        "  current-head Rust diagnostic: "
        f"median {current_rust:.3f}, range {min(samples):.3f}-{max(samples):.3f} M turns/s"
    )
    print("  same-machine Halide/Taichi/Mojo rerun samples: verified")
    print(
        "  AOT/Rust dylib: "
        f"scalar Mech {dylib_summary['scalar_aot_throughput_million_turns_per_second']:.3f}, "
        f"Rust {dylib_summary['rust_throughput_million_turns_per_second']:.3f}, "
        f"SIMD Mech {dylib_summary['simd_aot_throughput_million_turns_per_second']:.3f} M turns/s; "
        "size and peak RSS verified"
    )
    print("  two post-facing charts: raw samples, medians, and observed ranges verified")
    print("  checked and unchecked mega-chart assertions: passed")


if __name__ == "__main__":
    main()
