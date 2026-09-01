#!/usr/bin/env python3
"""Render compact per-execution-strategy EKF tables and graphs.

The source-edit mega report is intentionally retained.  This view answers a
different question: for one execution strategy, what is the one representative
source and checked/unchecked throughput for each language?
"""

from __future__ import annotations

import argparse
import datetime as dt
import html
import json
import math
import re
import statistics
from pathlib import Path

from chart_machine_specs import svg_machine_specs
from source_diff_report import COLORS, ROOT, diff_metrics, source_metrics


HERE = Path(__file__).resolve().parent
RESULTS = HERE / "results"
LANGUAGES = ("Mech", "Rust", "NumPy", "Python", "Julia", "LuaJIT", "Lua", "Taichi", "Halide", "Futhark")

STRATEGIES = {
    "interpreted-baseline": {
        "title": "Interpreted baseline",
        "description": "Interpreter and host-loop controls kept separate from native/JIT/data-parallel compilation.",
        "workload": "10,000 filters x 20 turns where available",
        "languages": ("Mech", "NumPy", "Python", "Lua"),
        "note": "NumPy is included here because this row is a Python loop invoking one-filter NumPy operations; its array kernels are native, but the outer execution remains interpreter-driven.",
    },
    "compiled-baseline": {
        "title": "Compiled baseline",
        "description": "Direct native, JIT, or ahead-of-time compiled controls, with no interpreter in the timed loop.",
        "workload": "10,000 filters x 20 turns where available",
        "languages": ("Mech", "Rust", "Julia", "LuaJIT", "Taichi", "Halide", "Futhark"),
        "note": "This view uses each language's retained native/JIT/AOT scalar control. Mech uses the paired scalar Cranelift JIT checked/unchecked measurements from the backend evidence; the single-core and multicore views remain separate SIMD/JIT controls.",
    },
    "baseline": {
        "title": "Baseline",
        "description": "The most direct scalar or fixed-shape control retained for each language.",
        "workload": "10,000 filters x 20 turns where available",
        "note": "Historical mixed view retained for compatibility. Use the interpreted and compiled baseline views for like-for-like execution-boundary comparisons.",
    },
    "single-core": {
        "title": "Single-core",
        "description": "One process and one host worker; explicit SIMD/JIT controls are used where the retained evidence provides them.",
        "workload": "10,000 filters x 20 turns where available",
        "note": "The closest Futhark comparison is its one-worker row (19.614 checked / 19.635 unchecked). The 48.391 / 47.824 Futhark result uses eight workers and belongs to the multicore view.",
        "chart_note": "Closest Futhark one-worker row: 19.614 / 19.635 M/s. The 48.391 / 47.824 row is eight-worker multicore, not a single-core comparison.",
    },
    "multicore": {
        "title": "Eight-worker multicore",
        "description": "Matched eight-worker CPU fused block; checked mode validates each candidate and publishes at the block boundary.",
        "workload": "500,000 filters x 40 turns where available",
    },
    "gpu": {
        "title": "Synchronized GPU",
        "description": "One GPU dispatch and completion wait per turn; checked rows retain the prior published state on a fault.",
        "workload": "500,000 filters x 40 turns, synchronized per turn",
    },
    "gpu-batched": {
        "title": "GPU batch ceiling",
        "description": "A device-resident multi-turn submission. This is a throughput ceiling, not a replacement for per-turn observation.",
        "workload": "500,000 filters x 40 turns, one submission where available",
    },
}

BASELINES = {
    "Mech": "benchmarks/archive/compute/parallel-ekf/minimal/ekf-kernel.mec",
    "Rust": "benchmarks/archive/compute/parallel-ekf/minimal/rust_scalar.rs",
    "NumPy": "benchmarks/archive/compute/parallel-ekf/minimal/numpy_scalar.py",
    "Python": "benchmarks/archive/compute/parallel-ekf/minimal/pure_python.py",
    "Julia": "benchmarks/archive/compute/parallel-ekf/minimal/julia_scalar.jl",
    "LuaJIT": "benchmarks/archive/compute/parallel-ekf/minimal/luajit_scalar.lua",
    "Lua": "benchmarks/archive/compute/parallel-ekf/minimal/luajit_fast.lua",
    "Taichi": "benchmarks/archive/compute/parallel-ekf/minimal/taichi_comparable.py",
    "Halide": "benchmarks/archive/compute/parallel-ekf/minimal/halide_ekf.cpp",
    "Futhark": "benchmarks/archive/compute/parallel-ekf/minimal/futhark_ekf.fut",
}

SOURCES = {
    "interpreted-baseline": {
        "Mech": BASELINES["Mech"],
        "NumPy": BASELINES["NumPy"],
        "Python": BASELINES["Python"],
        "Lua": BASELINES["Lua"],
    },
    "compiled-baseline": {
        "Mech": BASELINES["Mech"],
        "Rust": BASELINES["Rust"],
        "Julia": BASELINES["Julia"],
        "LuaJIT": BASELINES["LuaJIT"],
        "Taichi": BASELINES["Taichi"],
        "Halide": BASELINES["Halide"],
        "Futhark": BASELINES["Futhark"],
    },
    "baseline": dict(BASELINES),
    "single-core": {
        "Mech": BASELINES["Mech"],
        "Rust": "benchmarks/archive/compute/parallel-ekf/minimal/rust_simd.rs",
        "NumPy": "benchmarks/archive/compute/parallel-ekf/minimal/numpy_fast.py",
        "Python": None,
        "Julia": "benchmarks/archive/compute/parallel-ekf/minimal/julia_simd.jl",
        "LuaJIT": "benchmarks/archive/compute/parallel-ekf/minimal/luajit_fast.lua",
        "Lua": BASELINES["Lua"],
        "Taichi": None,
        "Halide": BASELINES["Halide"],
        "Futhark": BASELINES["Futhark"],
    },
    "multicore": {
        "Mech": BASELINES["Mech"],
        "Rust": "benchmarks/archive/compute/parallel-ekf/minimal/rust_simd.rs",
        "NumPy": "benchmarks/archive/compute/parallel-ekf/minimal/numpy_numba.py",
        "Python": None,
        "Julia": "benchmarks/archive/compute/parallel-ekf/minimal/julia_simd_threads.jl",
        "LuaJIT": None,
        "Lua": None,
        "Taichi": "benchmarks/archive/compute/parallel-ekf/minimal/taichi_optimized.py",
        "Halide": BASELINES["Halide"],
        "Futhark": BASELINES["Futhark"],
    },
    "gpu": {
        "Mech": BASELINES["Mech"],
        "Rust": None,
        "NumPy": None,
        "Python": None,
        "Julia": "benchmarks/archive/compute/parallel-ekf/minimal/julia_metal_ekf.jl",
        "LuaJIT": None,
        "Lua": None,
        "Taichi": "benchmarks/archive/compute/parallel-ekf/minimal/taichi_optimized.py",
        "Halide": BASELINES["Halide"],
        "Futhark": None,
    },
    "gpu-batched": {
        "Mech": BASELINES["Mech"],
        "Rust": None,
        "NumPy": None,
        "Python": None,
        "Julia": None,
        "LuaJIT": None,
        "Lua": None,
        "Taichi": None,
        "Halide": None,
        "Futhark": None,
    },
}

SOURCE_LABELS = {
    "interpreted-baseline": {
        "Mech": "resident scalar interpreter",
        "NumPy": "Python loop over scalar NumPy operations",
        "Python": "standard-library lists and math",
        "Lua": "PUC Lua flat fixed-shape arrays",
    },
    "compiled-baseline": {
        "Mech": "scalar Cranelift JIT",
        "Rust": "fixed-shape scalar arrays",
        "Julia": "generic scalar JIT arrays",
        "LuaJIT": "generic FFI JIT loop",
        "Taichi": "Vector/Matrix resident fields with compiled kernel",
        "Halide": "fixed-shape compiled pipeline",
        "Futhark": "compiled data-parallel array program",
    },
    "baseline": {
        "Mech": "same high-level `.mec` recurrence",
        "Rust": "fixed-shape scalar arrays",
        "NumPy": "per-filter NumPy loop",
        "Python": "standard-library lists and math",
        "Julia": "generic scalar arrays",
        "LuaJIT": "generic FFI helper loop",
        "Lua": "flat fixed-shape Lua arrays",
        "Taichi": "Vector/Matrix resident fields",
        "Halide": "fixed-shape pipeline",
        "Futhark": "data-parallel array program",
    },
    "single-core": {
        "Mech": "same `.mec`; Cranelift SIMD/JIT backend",
        "Rust": "packed four-lane SIMD",
        "NumPy": "batched fixed-shape arrays",
        "Python": "not applicable: no optimized source",
        "Julia": "explicit SIMD.jl lanes",
        "LuaJIT": "flat scalarized FFI state",
        "Lua": "flat scalarized Lua state",
        "Taichi": "not applicable: no single-core row",
        "Halide": "fixed-shape pipeline, one host worker",
        "Futhark": "same data-parallel program, one worker",
    },
    "multicore": {
        "Mech": "same `.mec`; checkpointed fused eight-worker SIMD/JIT block",
        "Rust": "packed SIMD with eight worker-local blocks",
        "NumPy": "Numba `prange` eight-worker loop",
        "Python": "not applicable: no worker implementation",
        "Julia": "Threads.@threads static publication",
        "LuaJIT": "not applicable: no worker implementation",
        "Lua": "not applicable: no worker implementation",
        "Taichi": "scalar SoA fields with eight CPU workers",
        "Halide": "parallel/vectorized pipeline with eight workers",
        "Futhark": "ISPC fixed-mode with eight workers",
    },
    "gpu": {
        "Mech": "same `.mec`; native Metal dispatch",
        "Rust": "not applicable: no GPU row",
        "NumPy": "not applicable on Apple M1",
        "Python": "not applicable: no GPU backend",
        "Julia": "direct Metal kernel with retained state",
        "LuaJIT": "not applicable: no GPU backend",
        "Lua": "not applicable: no GPU backend",
        "Taichi": "optimized native Metal kernel",
        "Halide": "strict native Metal pipeline",
        "Futhark": "not applicable: no Metal backend",
    },
    "gpu-batched": {
        "Mech": "same `.mec`; device-resident one-submit control",
        "Rust": "not applicable: no GPU row",
        "NumPy": "not applicable on Apple M1",
        "Python": "not applicable: no GPU backend",
        "Julia": "not applicable: no batch row",
        "LuaJIT": "not applicable: no GPU backend",
        "Lua": "not applicable: no GPU backend",
        "Taichi": "not applicable: no batch row",
        "Halide": "not applicable: no batch row",
        "Futhark": "not applicable: no Metal backend",
    },
}


def load_json(path: Path) -> dict | None:
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def median_value(row: dict | None, key: str, divisor: float = 1.0) -> float | None:
    if row is None or key not in row:
        return None
    value = row[key]
    values = value if isinstance(value, list) else [value]
    return statistics.median(float(item) for item in values) / divisor


def scalar_metric(cross: dict | None, label: str, mode: str | None = None) -> float | None:
    if cross is None:
        return None
    rows = cross.get("summary", {}).get("scalar_outer_loop", {})
    key = f"{label} {mode}" if mode else label
    row = rows.get(key)
    return None if row is None else float(row["ekf_turns_per_second"]) / 1_000_000


def row_metric(data: dict | None, label: str) -> float | None:
    if data is None:
        return None
    rows = data.get("rows", {})
    if isinstance(rows, list):
        row = next((item for item in rows if item.get("label") == label), None)
    else:
        row = rows.get(label)
    if row is None:
        return None
    if "throughput_millions" in row:
        return median_value(row, "throughput_millions")
    return median_value(row, "throughput", 1_000_000)


def pair(checked: float | None, unchecked: float | None) -> dict[str, float | None]:
    return {"checked": checked, "unchecked": unchecked}


def build_metrics(data: dict[str, dict | None]) -> dict[str, dict[str, dict[str, float | None]]]:
    cross = data["cross"]
    runtime = data["runtime"]
    lua = data["lua"]
    minimal = data["minimal"]
    julia_threaded = data["julia_threaded"]
    simd_controls = data["simd_controls"]
    julia_gpu = data["julia_gpu"]
    pure_python = data["pure_python"]
    rust_scalar = data["rust_scalar"]
    luajit_scalar = data["luajit_scalar"]
    halide_gpu = data["halide_gpu"]
    strict_mech = data["strict_mech"]
    fused = data["fused"]
    mech_persistent = data["mech_persistent"]
    futhark_fixed = data["futhark_fixed"]

    def m(label: str, mode: str | None = None) -> float | None:
        return scalar_metric(cross, label, mode)

    def min_m(label: str) -> float | None:
        return row_metric(minimal, label)

    def run_m(label: str) -> float | None:
        return row_metric(runtime, label)

    def strict_mech_m(mode: str) -> float | None:
        row = (strict_mech or {}).get("rows", {}).get(f"Mech native Metal {mode}")
        return None if row is None else float(row["median_million_turns_per_second"])

    def mech_backend_metric(label: str) -> float | None:
        """Read a paired scalar Cranelift result from summary or raw backend output."""
        summary = (cross or {}).get("summary", {}).get("mech_backends_million_ekf_turns_per_second", {})
        if label in summary:
            return float(summary[label])
        outputs = (cross or {}).get("runs", {}).get("mech_backend_settings", {}).get("measured_stdout", [])
        pattern = re.compile(r"^" + re.escape(label) + r" throughput: ([0-9.]+) million", re.MULTILINE)
        values = []
        for output in outputs:
            match = pattern.search(output)
            if match:
                values.append(float(match.group(1)))
        return statistics.median(values) if values else None

    def mech_simd_metric(mode: str) -> float | None:
        """Use the ordinary SIMD-JIT row for the requested publication mode."""
        label = "Mech Cranelift SIMD-JIT" if mode == "checked" else "Mech Cranelift SIMD-JIT unchecked"
        return mech_backend_metric(label)

    metrics = {
        "baseline": {
            "Mech": pair(m("Mech scalar"), m("Mech scalar unchecked")),
            "Rust": pair(row_metric(rust_scalar, "checked"), row_metric(rust_scalar, "unchecked")),
            "NumPy": pair(min_m("NumPy scalar checked"), min_m("NumPy scalar unchecked")),
            "Python": pair(row_metric(pure_python, "checked"), row_metric(pure_python, "unchecked")),
            "Julia": pair(m("Julia generic", "checked"), m("Julia generic", "unchecked")),
            "LuaJIT": pair(row_metric(luajit_scalar, "checked"), row_metric(luajit_scalar, "unchecked")),
            "Lua": pair(row_metric(lua, "Lua fixed-shape flat checked"), row_metric(lua, "Lua fixed-shape flat unchecked")),
            "Taichi": pair(row_metric(data["taichi_cpu_baseline"], "checked"), row_metric(data["taichi_cpu_baseline"], "unchecked")),
            "Halide": pair(min_m("Halide checked"), min_m("Halide unchecked")),
            "Futhark": pair(min_m("Futhark multicore 1 threads checked"), min_m("Futhark multicore 1 threads unchecked")),
        },
        "single-core": {
            "Mech": pair(mech_simd_metric("checked"), mech_simd_metric("unchecked")),
            "Rust": pair(m("Rust packed SIMD", "checked"), m("Rust packed SIMD", "unchecked")),
            "NumPy": pair(min_m("NumPy advanced checked"), min_m("NumPy advanced unchecked")),
            "Python": pair(None, None),
            "Julia": pair(m("Julia SIMD.jl intrinsics", "checked"), m("Julia SIMD.jl intrinsics", "unchecked")),
            "LuaJIT": pair(m("LuaJIT fixed-shape flat", "checked"), m("LuaJIT fixed-shape flat", "unchecked")),
            "Lua": pair(row_metric(lua, "Lua fixed-shape flat checked"), row_metric(lua, "Lua fixed-shape flat unchecked")),
            "Taichi": pair(None, None),
            "Halide": pair(min_m("Halide checked"), min_m("Halide unchecked")),
            "Futhark": pair(min_m("Futhark multicore 1 threads checked"), min_m("Futhark multicore 1 threads unchecked")),
        },
        "multicore": {
            "Mech": pair(row_metric(fused, "mech_fused_checked"), row_metric(mech_persistent, "fused_unchecked_block")),
            "Rust": pair(row_metric(fused, "rust_fused_checked"), row_metric(fused, "rust_fused")),
            "NumPy": pair(row_metric(fused, "numba_fused_checked"), row_metric(fused, "numba_fused")),
            "Python": pair(None, None),
            "Julia": pair(row_metric(julia_threaded, "checked"), row_metric(julia_threaded, "unchecked")),
            "LuaJIT": pair(None, None),
            "Lua": pair(None, None),
            "Taichi": pair(run_m("Taichi LLVM CPU, checked (8 workers)"), run_m("Taichi LLVM CPU, unchecked (8 workers)")),
            "Halide": pair(row_metric(simd_controls, "Halide JIT SIMD 8 workers checked"), row_metric(simd_controls, "Halide JIT SIMD 8 workers unchecked")),
            "Futhark": pair(row_metric(futhark_fixed, "checked"), row_metric(futhark_fixed, "unchecked")),
        },
        "gpu": {
            "Mech": pair(strict_mech_m("checked") or run_m("Mech WGPU GPU, checked"), strict_mech_m("unchecked") or run_m("Mech WGPU GPU, unchecked")),
            "Rust": pair(None, None),
            "NumPy": pair(None, None),
            "Python": pair(None, None),
            "Julia": pair(row_metric(julia_gpu, "checked"), row_metric(julia_gpu, "unchecked")),
            "LuaJIT": pair(None, None),
            "Lua": pair(None, None),
            "Taichi": pair(row_metric(data["taichi_optimized"], "Taichi optimized native Metal, checked"), row_metric(data["taichi_optimized"], "Taichi optimized native Metal, unchecked")),
            "Halide": pair(row_metric(halide_gpu, "Halide GPU Metal checked"), row_metric(halide_gpu, "Halide GPU Metal unchecked")),
            "Futhark": pair(None, None),
        },
        "gpu-batched": {
            "Mech": pair(None, (cross or {}).get("summary", {}).get("mech_backends_million_ekf_turns_per_second", {}).get("Mech GPU, unchecked one submission")),
            "Rust": pair(None, None),
            "NumPy": pair(None, None),
            "Python": pair(None, None),
            "Julia": pair(None, None),
            "LuaJIT": pair(None, None),
            "Lua": pair(None, None),
            "Taichi": pair(None, None),
            "Halide": pair(None, None),
            "Futhark": pair(None, None),
        },
    }
    metrics["interpreted-baseline"] = {
        language: metrics["baseline"][language]
        for language in STRATEGIES["interpreted-baseline"]["languages"]
    }
    metrics["compiled-baseline"] = {
        language: metrics["baseline"][language]
        for language in STRATEGIES["compiled-baseline"]["languages"]
    }
    metrics["compiled-baseline"]["Mech"] = pair(
        mech_backend_metric("Mech Cranelift JIT"),
        mech_backend_metric("Mech Cranelift JIT unchecked"),
    )
    return metrics


def status(source_path: str | None, values: dict[str, float | None]) -> str:
    if source_path is None:
        return "N/A: no implementation"
    missing = [mode for mode in ("checked", "unchecked") if values[mode] is None]
    if missing:
        return "partial: missing " + "/".join(missing)
    return "measured"


def build_report(data: dict[str, dict | None], output_directory: Path) -> dict:
    metrics = build_metrics(data)
    strategies: dict[str, list[dict[str, object]]] = {}
    omitted: dict[str, list[dict[str, str]]] = {}
    missing_cells: list[dict[str, str]] = []
    for strategy in STRATEGIES:
        rows = []
        omitted[strategy] = []
        for language in STRATEGIES[strategy].get("languages", LANGUAGES):
            baseline_path = ROOT / BASELINES[language]
            source_name = SOURCES[strategy][language]
            source_path = ROOT / source_name if source_name is not None else None
            values = metrics[strategy][language]
            if source_name is None or all(value is None for value in values.values()):
                omitted[strategy].append(
                    {
                        "language": language,
                        "reason": "backend/strategy unavailable" if source_name is None else "source exists but was not tested",
                    }
                )
                continue
            row: dict[str, object] = {
                "language": language,
                "source": SOURCE_LABELS[strategy][language],
                "source_path": source_name,
                "values": values,
                "status": status(source_name, values),
            }
            if source_path is None:
                row["code"] = None
                row["edit_vs_baseline"] = None
            else:
                source_text = source_path.read_text(encoding="utf-8")
                baseline_text = baseline_path.read_text(encoding="utf-8")
                row["code"] = source_metrics(source_text, source_path)
                row["edit_vs_baseline"] = diff_metrics(baseline_text, source_text)
            rows.append(row)
            for mode in ("checked", "unchecked"):
                if values[mode] is None and source_name is not None:
                    missing_cells.append({"strategy": strategy, "language": language, "mode": mode, "status": "not recorded"})
        strategies[strategy] = rows
    evidence = {name: (value or {}).get("generated_at") for name, value in data.items()}
    return {
        "schema_version": 1,
        "generated_at": dt.datetime.now().astimezone().isoformat(),
        "strategies": strategies,
        "omitted": omitted,
        "missing_cells": missing_cells,
        "evidence": evidence,
        "definitions": STRATEGIES,
    }


def fmt(value: float | None) -> str:
    return "N/A" if value is None else f"{value:.3f}"


def markdown(report: dict, strategy: str) -> str:
    spec = STRATEGIES[strategy]
    lines = [
        f"# Parallel EKF: {spec['title']}",
        "",
        f"{spec['description']} Workload: **{spec['workload']}**. Checked and unchecked are separate columns; source edits are measured against each language's baseline source.",
        "",
    ]
    if spec.get("note"):
        lines += [f"**Scope note:** {spec['note']}", ""]
    lines += [
        "| Language | Representative source | Code L/C | Edit vs baseline L/C | Checked M/s | Unchecked M/s | Result |",
        "| --- | --- | ---: | ---: | ---: | ---: | --- |",
    ]
    for row in report["strategies"][strategy]:
        code = row["code"]
        edit = row["edit_vs_baseline"]
        code_cell = "N/A" if code is None else f"{code['lines']} / {code['chars']:,}"
        edit_cell = "N/A" if edit is None else f"{edit['changed_line_slots']} / {edit['changed_chars']:,}"
        values = row["values"]
        lines.append(f"| {row['language']} | {row['source']} | {code_cell} | {edit_cell} | {fmt(values['checked'])} | {fmt(values['unchecked'])} | {row['status']} |")
    lines += [
        "",
        "`partial` means the source exists but one checked/unchecked measurement is not recorded yet; it is not treated as zero.",
        "",
    ]
    if report["omitted"][strategy]:
        lines += ["## Missing backends and untested controls", ""]
        for item in report["omitted"][strategy]:
            lines.append(f"- **{item['language']}**: {item['reason']}.")
        lines.append("")
    return "\n".join(lines)


def svg(report: dict, strategy: str) -> str:
    rows = report["strategies"][strategy]
    omitted = report["omitted"][strategy]
    positive = [value for row in rows for value in row["values"].values() if value is not None and value > 0]
    maximum_value = max(positive) if positive else 1.0
    raw_step = maximum_value / 6.0
    magnitude = 10 ** math.floor(math.log10(raw_step)) if raw_step > 0 else 1.0
    fraction = raw_step / magnitude
    if fraction <= 1.0:
        tick_step = magnitude
    elif fraction <= 2.0:
        tick_step = 2.0 * magnitude
    elif fraction <= 5.0:
        tick_step = 5.0 * magnitude
    else:
        tick_step = 10.0 * magnitude
    maximum = tick_step * math.ceil(maximum_value / tick_step)
    width, left, right, row_height = 1500, 300, 100, 55
    top = 145 if STRATEGIES[strategy].get("chart_note") else 125
    bottom = 90 + 18 * (1 + len(omitted))
    height = top + row_height * len(rows) + bottom

    def esc(value: object) -> str:
        return html.escape(str(value), quote=True)

    def x(value: float) -> float:
        return left + value / maximum * (width - left - right)

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#91a0b5}.grid{stroke:#263246;stroke-width:1}.label{font-size:15px}.value{font-size:13px;font-variant-numeric:tabular-nums}</style>',
        f'<text x="38" y="42" font-size="27" font-weight="700">Parallel EKF: {esc(STRATEGIES[strategy]["title"])} throughput</text>',
        f'<text x="38" y="69" class="muted" font-size="15">{esc(STRATEGIES[strategy]["description"])} Checked is solid; unchecked is lighter. Linear M/s axis.</text>',
    ]
    if STRATEGIES[strategy].get("chart_note"):
        lines.append(f'<text x="38" y="91" class="muted" font-size="13">{esc(STRATEGIES[strategy]["chart_note"])}</text>')
        legend_y = 108
    else:
        legend_y = 87
    lines.extend([
        f'<rect x="38" y="{legend_y}" width="18" height="12" fill="#dce5f2"/><text x="64" y="{legend_y + 11}" class="muted" font-size="13">checked</text>',
        f'<rect x="145" y="{legend_y}" width="18" height="12" fill="#dce5f2" opacity="0.42"/><text x="171" y="{legend_y + 11}" class="muted" font-size="13">unchecked</text>',
    ])
    tick = 0.0
    while tick <= maximum * 1.0001:
        tick_x = x(tick)
        lines.append(f'<line x1="{tick_x:.1f}" y1="{top - 15}" x2="{tick_x:.1f}" y2="{height - bottom}" class="grid"/>')
        lines.append(f'<text x="{tick_x:.1f}" y="{height - bottom + 24}" text-anchor="middle" class="muted" font-size="12">{tick:g}</text>')
        tick += tick_step
    for index, row in enumerate(rows):
        y = top + index * row_height
        color = COLORS[row["language"]]
        lines.append(f'<text x="{left - 14}" y="{y + 24}" text-anchor="end" class="label">{esc(row["language"])}</text>')
        for offset, mode in ((4, "checked"), (29, "unchecked")):
            value = row["values"][mode]
            if value is None:
                lines.append(f'<text x="{left + 8}" y="{y + offset + 14}" class="muted" font-size="12">N/A</text>')
                continue
            end = x(value)
            lines.append(f'<rect x="{left}" y="{y + offset}" width="{max(2, end - left):.1f}" height="16" rx="2" fill="{color}" opacity="{0.92 if mode == "checked" else 0.42}"/>')
            lines.append(f'<text x="{min(end + 7, width - right - 35):.1f}" y="{y + offset + 13}" class="value">{value:.3f}</text>')
    footer_y = height - bottom + 45
    lines.append(f'<text x="38" y="{footer_y}" class="muted" font-size="12">Bars show only languages with at least one measured result; checked is solid and unchecked is lighter.</text>')
    for index, item in enumerate(omitted, start=1):
        lines.append(f'<text x="38" y="{footer_y + 18 * index}" class="muted" font-size="12">{esc(item["language"])}: {esc(item["reason"])}.</text>')
    lines.append(svg_machine_specs(width, height, right=right, bottom=18))
    lines.append("</svg>")
    return "\n".join(lines) + "\n"


def load_inputs(results: Path) -> dict[str, dict | None]:
    names = {
        "cross": "apple-m1-checked-cross-language-2026-08-31.json",
        "runtime": "apple-m1-mech-taichi-runtime-2026-08-31.json",
        "native": "apple-m1-mech-taichi-native-metal-2026-08-31.json",
        "taichi": "apple-m1-taichi-optimized-native-metal-2026-08-31.json",
        "taichi_optimized": "apple-m1-taichi-optimized-native-metal-2026-08-31.json",
        "taichi_cpu_baseline": "apple-m1-taichi-cpu-baseline-2026-09-01.json",
        "lua": "apple-m1-lua-2026-08-31.json",
        "minimal": "apple-m1-minimal-source-2026-08-31.json",
        "julia_threaded": "apple-m1-julia-threaded-2026-08-31.json",
        "numpy_numba": "apple-m1-numpy-numba-2026-08-31.json",
        "simd_controls": "apple-m1-futhark-halide-simd-2026-08-31.json",
        "futhark_fixed": "apple-m1-futhark-ispc-fixed-2026-08-31.json",
        "julia_gpu": "apple-m1-julia-metal-2026-08-31.json",
        "pure_python": "apple-m1-pure-python-2026-09-01.json",
        "rust_scalar": "apple-m1-rust-scalar-2026-09-01.json",
        "luajit_scalar": "apple-m1-luajit-scalar-2026-09-01.json",
        "halide_gpu": "apple-m1-halide-metal-strict-2026-08-31.json",
        "strict_mech": "apple-m1-mech-halide-strict-2026-08-31.json",
        "fused": "apple-m1-fused-reference-controls-2026-08-31.json",
        "mech_persistent": "apple-m1-mech-persistent-simd-2026-08-31.json",
    }
    return {key: load_json(results / filename) for key, filename in names.items()}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--results", type=Path, default=RESULTS)
    args = parser.parse_args()
    report = build_report(load_inputs(args.results), args.results)
    args.results.mkdir(parents=True, exist_ok=True)
    (args.results / "parallel-ekf-execution-strategy-report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    index = ["# Parallel EKF execution-strategy reports", "", "The existing source-edit mega report remains the complete cross-variant view. These compact reports select one representative source and one checked/unchecked result per language for each execution strategy. The baseline is split into interpreted and compiled controls; the historical mixed baseline remains for compatibility.", "", "| Strategy | Diff table | Graph |", "| --- | --- | --- |"]
    for strategy in STRATEGIES:
        stem = f"parallel-ekf-strategy-{strategy}"
        (args.results / f"{stem}.md").write_text(markdown(report, strategy), encoding="utf-8")
        (args.results / f"{stem}.svg").write_text(svg(report, strategy), encoding="utf-8")
        index.append(f"| {STRATEGIES[strategy]['title']} | [`{stem}.md`]({stem}.md) | [`{stem}.svg`]({stem}.svg) |")
    index += ["", "## Omitted controls", "", "Languages with no measured result are omitted from the corresponding tables and graphs. The reason is retained here:", "", "| Strategy | Language | Reason |", "| --- | --- | --- |"]
    for strategy in STRATEGIES:
        for item in report["omitted"][strategy]:
            index.append(f"| {strategy} | {item['language']} | {item['reason']} |")
    index += ["", "## Evidence gaps", "", "The generator does not turn an absent measurement into zero. Applicable cells still awaiting a run are listed here and in the JSON `missing_cells` array:", "", "| Strategy | Language | Mode |", "| --- | --- | --- |"]
    for cell in report["missing_cells"]:
        index.append(f"| {cell['strategy']} | {cell['language']} | {cell['mode']} |")
    index += ["", "`partial` means a source exists but a checked/unchecked measurement is not retained; those cells are listed above rather than being fabricated.", ""]
    (args.results / "parallel-ekf-execution-strategy-reports.md").write_text("\n".join(index), encoding="utf-8")


if __name__ == "__main__":
    main()
