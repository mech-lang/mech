#!/usr/bin/env python3
"""Measure source edits behind the parallel-EKF benchmark variants."""

from __future__ import annotations

import argparse
import difflib
import html
import json
import statistics
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
BASE_MECH = ROOT / "benchmarks/archive/compute/parallel-ekf/minimal/ekf-kernel.mec"
REFERENCE_MECH = ROOT / "hosts/gpu/fixtures/ekf-kernel.mec"
MECH_SUPPORT_BASE = "1ca47cdeef1cef071a891babf5423af03f51466f"
MECH_SUPPORT_FILES = (
    "Cargo.lock",
    "hosts/gpu/Cargo.toml",
    "hosts/gpu/examples/parallel_ekf_benchmark.rs",
    "hosts/gpu/src/batched/mod.rs",
    "hosts/gpu/src/metal.rs",
)

COLORS = {
    "Mech": "#f4c430",
    "Rust": "#dea584",
    "Python": "#4d77cf",
    "NumPy": "#4d77cf",
    "Julia": "#9558b2",
    "Lua": "#000080",
    "LuaJIT": "#000080",
    "Taichi": "#e36b6b",
    "Halide": "#ff8f00",
    "Futhark": "#e94f37",
}


VARIANTS = [
    {
        "language": "Mech",
        "baseline": "benchmarks/archive/compute/parallel-ekf/minimal/ekf-kernel.mec",
        "advanced": "benchmarks/archive/compute/parallel-ekf/minimal/ekf-kernel.mec",
        "baseline_label": "compact high-level `.mec` program",
        "advanced_label": "same compact `.mec`; native backend selected at build",
        "note": "The compact source recurrence does not change. Native Metal specialization is backend support, not a second Mech program.",
    },
    {
        "language": "Rust",
        "baseline": "benchmarks/archive/compute/parallel-ekf/minimal/rust_scalar.rs",
        "advanced": "benchmarks/archive/compute/parallel-ekf/minimal/rust_simd.rs",
        "baseline_label": "compact fixed-shape scalar control",
        "advanced_label": "compact packed four-lane SIMD control",
        "note": "The compact controls preserve the checked-in Rust algorithms while removing narrative scaffolding; the advanced control still changes the value representation and execution loop.",
    },
    {
        "language": "NumPy",
        "baseline": "benchmarks/archive/compute/parallel-ekf/minimal/numpy_scalar.py",
        "advanced": "benchmarks/archive/compute/parallel-ekf/minimal/numpy_fast.py",
        "baseline_label": "compact per-filter scalar loop",
        "advanced_label": "compact batched fixed-shape vectorized operations",
        "note": "The baseline is a per-filter NumPy call from a Python loop; the advanced control uses fixed-shape batched arrays. The row is labeled NumPy because both variants use NumPy for the numeric work.",
    },
    {
        "language": "Python",
        "baseline": "benchmarks/archive/compute/parallel-ekf/minimal/pure_python.py",
        "advanced": "benchmarks/archive/compute/parallel-ekf/minimal/pure_python.py",
        "baseline_label": "standard-library scalar control",
        "advanced_label": "same scalar control; no optimized Python variant",
        "note": "This is a pure-Python lower-bound control using only math and ordinary lists. Baseline and advanced intentionally reference the same source because no optimized Python source variant is retained.",
    },
    {
        "language": "Julia",
        "baseline": "benchmarks/archive/compute/parallel-ekf/minimal/julia_scalar.jl",
        "advanced": "benchmarks/archive/compute/parallel-ekf/minimal/julia_simd.jl",
        "baseline_label": "compact generic scalar Julia",
        "advanced_label": "compact explicit four-lane SIMD.jl intrinsics",
        "note": "The compact controls preserve the Julia algorithms; the advanced source introduces an explicit packed value type and lane loop.",
    },
    {
        "language": "LuaJIT",
        "baseline": "benchmarks/archive/compute/parallel-ekf/minimal/luajit_scalar.lua",
        "advanced": "benchmarks/archive/compute/parallel-ekf/minimal/luajit_fast.lua",
        "baseline_label": "compact generic matrix helper loop",
        "advanced_label": "compact flat fixed-shape scalarized state",
        "note": "The compact controls preserve the Lua algorithms; the advanced source removes helper-level matrix temporaries and writes each component directly.",
    },
    {
        "language": "Lua",
        "baseline": "benchmarks/archive/compute/parallel-ekf/minimal/luajit_fast.lua",
        "advanced": "benchmarks/archive/compute/parallel-ekf/minimal/luajit_fast.lua",
        "baseline_label": "same compact flat source under PUC Lua",
        "advanced_label": "same compact flat source under PUC Lua",
        "note": "The Lua comparison isolates the runtime: the source is identical to the LuaJIT flat control.",
    },
    {
        "language": "Taichi",
        "baseline": "benchmarks/archive/compute/parallel-ekf/minimal/taichi_comparable.py",
        "advanced": "benchmarks/archive/compute/parallel-ekf/minimal/taichi_optimized.py",
        "baseline_label": "compact Vector/Matrix resident fields",
        "advanced_label": "compact scalar SoA fields and unrolled 3x3 arithmetic",
        "note": "This is the compact source-specialized Taichi control; it still uses stock Taichi 1.7.4 and per-turn sync.",
    },
    {
        "language": "Halide",
        "baseline": "benchmarks/archive/compute/parallel-ekf/minimal/halide_ekf.cpp",
        "advanced": "benchmarks/archive/compute/parallel-ekf/minimal/halide_ekf.cpp",
        "baseline_label": "same fixed-shape JIT pipeline",
        "advanced_label": "same pipeline; strict checked publication and fault output",
        "note": "Halide is a fixed-shape C++ pipeline JIT. Checked mode selects the previous lane state when the candidate fails the finite/diagonal/symmetry checks and emits a per-lane fault code for host observation.",
    },
    {
        "language": "Futhark",
        "baseline": "benchmarks/archive/compute/parallel-ekf/minimal/futhark_ekf.fut",
        "advanced": "benchmarks/archive/compute/parallel-ekf/minimal/futhark_ekf.fut",
        "baseline_label": "same data-parallel program",
        "advanced_label": "same program; multicore worker count",
        "note": "Futhark expresses the lane map in the source. The reported advanced control uses the same source with eight multicore workers and keeps the turns loop inside one compiled invocation; OpenCL is recorded separately when the local driver can execute it.",
    },
]


FACTORS = {
    "Mech": {
        "layout": "column-major resident graph values",
        "boundary": "resident host turn; backend selected at build",
        "contract": "checked rejects candidate and keeps prior; unchecked omits checks",
    },
    "Rust": {
        "layout": "fixed scalar arrays -> four-lane packed values",
        "boundary": "synchronous host loop, one update per turn",
        "contract": "checked and unchecked controls; no rollback in unchecked",
    },
    "NumPy": {
        "layout": "per-lane arrays -> batched SoA arrays",
        "boundary": "Python host loop (scalar) or one vectorized call per turn",
        "contract": "checked masked copyback keeps prior lane; unchecked overwrites",
    },
    "Python": {
        "layout": "ordinary Python lists of scalar state/covariance values",
        "boundary": "synchronous Python loop, one update per turn",
        "contract": "checked candidate publication; unchecked overwrites",
    },
    "Julia": {
        "layout": "generic arrays -> explicit four-lane SIMD values",
        "boundary": "synchronous host loop, one update per turn",
        "contract": "checked candidate publication; unchecked omits checks",
    },
    "LuaJIT": {
        "layout": "matrix helpers -> flat fixed-shape scalar state",
        "boundary": "synchronous host loop, one update per turn",
        "contract": "checked candidate publication; unchecked omits checks",
    },
    "Lua": {
        "layout": "flat fixed-shape Lua tables/FFI-compatible arrays",
        "boundary": "synchronous host loop, one update per turn",
        "contract": "checked candidate publication; unchecked omits checks",
    },
    "Taichi": {
        "layout": "Vector/Matrix fields -> scalar SoA fields",
        "boundary": "resident kernel with per-turn device synchronization",
        "contract": "checked alternate fields keep prior; unchecked writes in place",
    },
    "Halide": {
        "layout": "fixed-shape lane buffers, vectorized by eight",
        "boundary": "one JIT pipeline call per host turn",
        "contract": "checked validates finite/positive/symmetric candidates, reports per-lane faults, and keeps prior; unchecked omits checks",
    },
    "Futhark": {
        "layout": "fixed-size array of 12-value lane states",
        "boundary": "turn loop inside one compiled invocation; multicore map",
        "contract": "checked select keeps prior lane; unchecked selects candidate",
    },
}


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def source_metrics(text: str, path: Path) -> dict[str, int]:
    """Count non-empty, non-comment source so comments do not skew the table."""
    code: list[str] = []
    in_docstring = False
    for raw in text.splitlines():
        line = raw.strip()
        if path.suffix == ".py":
            if line.startswith(('"""', "'''")):
                delimiter = line[:3]
                if line.count(delimiter) % 2:
                    in_docstring = not in_docstring
                continue
            if in_docstring or line.startswith("#"):
                continue
        elif path.suffix in {".mec", ".lua"}:
            if line.startswith("--") or (line and set(line) == {"-"}):
                continue
            if path.suffix == ".mec" and "--" in line:
                line = line.split("--", 1)[0].rstrip()
        elif path.suffix in {".cpp", ".rs", ".jl"}:
            if line.startswith("//") or line.startswith("#"):
                continue
        if line:
            code.append(line)
    return {"lines": len(code), "chars": sum(len(line) for line in code)}


def mech_support_delta() -> dict[str, object]:
    """Count the backend implementation delta separately from `.mec` edits."""
    command = ["git", "diff", "--numstat", MECH_SUPPORT_BASE, "HEAD", "--", *MECH_SUPPORT_FILES]
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
    additions = deletions = 0
    files = []
    if result.returncode == 0:
        for line in result.stdout.splitlines():
            fields = line.split("\t", 2)
            if len(fields) != 3:
                continue
            added, deleted, path = fields
            if added.isdigit() and deleted.isdigit():
                additions += int(added)
                deletions += int(deleted)
                files.append(path)
    return {
        "base_commit": MECH_SUPPORT_BASE,
        "head_commit": "HEAD",
        "files": files,
        "added_lines": additions,
        "deleted_lines": deletions,
        "changed_line_slots": additions + deletions,
        "note": "This is backend/compiler support for native Metal and the direct benchmark path; generated WGSL/MSL is an artifact, not a second user program.",
    }


def diff_metrics(old: str, new: str) -> dict[str, int]:
    old_lines = old.splitlines(keepends=True)
    new_lines = new.splitlines(keepends=True)
    line_slots = added_lines = deleted_lines = 0
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(
        None, old_lines, new_lines, autojunk=False
    ).get_opcodes():
        if tag != "equal":
            line_slots += max(i2 - i1, j2 - j1)
            deleted_lines += i2 - i1
            added_lines += j2 - j1
    # Character-level SequenceMatcher becomes quadratic for the larger Rust
    # and Taichi controls. Count character slots in the changed line blocks
    # instead; this is deterministic and measures the edit surface directly.
    changed_chars = 0
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(
        None, old_lines, new_lines, autojunk=False
    ).get_opcodes():
        if tag != "equal":
            changed_chars += max(
                sum(len(line) for line in old_lines[i1:i2]),
                sum(len(line) for line in new_lines[j1:j2]),
            )
    return {
        "changed_line_slots": line_slots,
        "added_lines": added_lines,
        "deleted_lines": deleted_lines,
        "changed_chars": changed_chars,
    }


def performance_maxima(table: Path) -> dict[str, dict[str, dict[str, dict[str, object]]]]:
    """Read the canonical ranked table and keep the best lane per execution class."""
    maxima: dict[str, dict[str, dict[str, dict[str, object]]]] = {}
    mode: str | None = None
    for raw in table.read_text(encoding="utf-8").splitlines():
        heading = raw.strip().lower()
        if heading.startswith("## checked"):
            mode = "checked"
            continue
        if heading.startswith("## unchecked"):
            mode = "unchecked"
            continue
        if mode is None or not raw.startswith("|"):
            continue
        fields = [field.strip() for field in raw.strip().strip("|").split("|")]
        if len(fields) != 4 or not fields[0].isdigit():
            continue
        family = fields[2]
        try:
            throughput = float(fields[3])
        except ValueError:
            continue
        label = fields[1]
        lowered = label.lower()
        if any(marker in lowered for marker in ("gpu", "metal", "wgpu")):
            if any(
                marker in lowered
                for marker in (
                    "one-submit",
                    "one submission",
                    "repeated",
                    "batched",
                    "turns/submission",
                )
            ):
                category = "gpu_batched"
            else:
                category = "gpu"
        elif any(
            marker in lowered
            for marker in ("worker", "workers", "multicore", "parallel", "pool", "thread")
        ):
            category = "simd_multicore"
        else:
            category = "single_core"
        current = maxima.setdefault(family, {}).setdefault(category, {}).get(mode)
        if current is None or throughput > float(current["throughput"]):
            maxima[family][category][mode] = {
                "throughput": throughput,
                "label": label,
            }
    return maxima


def scalar_throughput(cross: dict, label: str) -> dict[str, float | None]:
    row = cross.get("summary", {}).get("scalar_outer_loop", {}).get(label)
    if row is None:
        return {"checked": None, "unchecked": None}
    # The scalar summary stores one mode per label. The caller maps explicit
    # checked/unchecked labels where both modes exist.
    mode = "checked" if label.endswith(" checked") else "unchecked"
    return {"checked": row["ekf_turns_per_second"] / 1e6 if mode == "checked" else None,
            "unchecked": row["ekf_turns_per_second"] / 1e6 if mode == "unchecked" else None}


def throughput_rows(cross: dict, native: dict, taichi: dict, lua: dict, minimal: dict | None) -> dict[str, dict[str, float | None]]:
    scalar = cross["summary"]["scalar_outer_loop"]
    native_rows = {row["label"]: row for row in native["rows"]}
    result: dict[str, dict[str, float | None]] = {
        "Mech": {"checked": native_rows["Mech native Metal, checked"]["throughput_millions"], "unchecked": native_rows["Mech native Metal, unchecked"]["throughput_millions"]},
        "Rust": {"checked": scalar["Rust packed SIMD checked"]["ekf_turns_per_second"] / 1e6, "unchecked": scalar["Rust packed SIMD unchecked"]["ekf_turns_per_second"] / 1e6},
        "NumPy": {"checked": scalar["NumPy vectorized fixed-shape checked"]["ekf_turns_per_second"] / 1e6, "unchecked": scalar["NumPy vectorized fixed-shape unchecked"]["ekf_turns_per_second"] / 1e6},
        "Julia": {"checked": scalar["Julia SIMD.jl intrinsics checked"]["ekf_turns_per_second"] / 1e6, "unchecked": scalar["Julia SIMD.jl intrinsics unchecked"]["ekf_turns_per_second"] / 1e6},
        "LuaJIT": {"checked": scalar["LuaJIT fixed-shape flat checked"]["ekf_turns_per_second"] / 1e6, "unchecked": scalar["LuaJIT fixed-shape flat unchecked"]["ekf_turns_per_second"] / 1e6},
        "Lua": {"checked": lua["rows"][0]["throughput_millions"], "unchecked": lua["rows"][1]["throughput_millions"]},
        "Taichi": {"checked": taichi["rows"][0]["throughput_millions"], "unchecked": taichi["rows"][1]["throughput_millions"]},
    }
    if minimal is not None:
        rows = minimal.get("rows", {})

        def median(label: str) -> float | None:
            row = rows.get(label)
            if row is None or "throughput" not in row:
                return None
            return statistics.median(row["throughput"]) / 1e6

        result["Halide"] = {"checked": median("Halide checked"), "unchecked": median("Halide unchecked")}
        result["Futhark"] = {
            "checked": median("Futhark multicore 8 threads checked"),
            "unchecked": median("Futhark multicore 8 threads unchecked"),
        }
    return result


def throughput_variants(
    cross: dict,
    native: dict,
    taichi: dict,
    lua: dict,
    minimal: dict | None,
    strict_mech: dict | None = None,
    strict_halide: dict | None = None,
    pure_python: dict | None = None,
) -> dict[str, dict[str, dict[str, float | None]]]:
    """Return benchmark values for both sides of every source pair."""
    scalar = cross["summary"]["scalar_outer_loop"]
    native_rows = {row["label"]: row for row in native["rows"]}
    strict_mech_rows = (strict_mech or {}).get("rows", {})

    def mech_metric(mode: str) -> float:
        strict_row = strict_mech_rows.get(f"Mech native Metal {mode}")
        if strict_row is not None:
            return strict_row["median_million_turns_per_second"]
        return native_rows[f"Mech native Metal, {mode}"]["throughput_millions"]

    def m(label: str, mode: str) -> float | None:
        row = scalar.get(label if not mode else f"{label} {mode}")
        return None if row is None else row["ekf_turns_per_second"] / 1e6

    def min_m(label: str) -> float | None:
        if minimal is None:
            return None
        row = minimal.get("rows", {}).get(label)
        if row is None or "throughput" not in row:
            return None
        return statistics.median(row["throughput"]) / 1e6

    def strict_halide_metric(mode: str) -> float | None:
        if strict_halide is None:
            return None
        row = strict_halide.get("rows", {}).get(f"Halide GPU Metal {mode}")
        if row is None or "throughput" not in row:
            return None
        return statistics.median(row["throughput"]) / 1e6

    def pure_python_metric(mode: str) -> float | None:
        if pure_python is None:
            return None
        row = pure_python.get("rows", {}).get(mode)
        if row is None or "throughput_millions" not in row:
            return None
        return statistics.median(row["throughput_millions"])

    def pair(checked: float | None, unchecked: float | None) -> dict[str, float | None]:
        return {"checked": checked, "unchecked": unchecked}

    result: dict[str, dict[str, dict[str, float | None]]] = {
        "Mech": {
            "baseline": pair(m("Mech scalar", ""), m("Mech scalar", "unchecked")),
            "advanced": pair(mech_metric("checked"), mech_metric("unchecked")),
        },
        "Rust": {
            "baseline": pair(None, m("Rust optimized fixed-shape", "")),
            "advanced": pair(m("Rust packed SIMD", "checked"), m("Rust packed SIMD", "unchecked")),
        },
        "NumPy": {
            "baseline": pair(min_m("NumPy scalar checked"), min_m("NumPy scalar unchecked")),
            "advanced": pair(min_m("NumPy advanced checked"), min_m("NumPy advanced unchecked")),
        },
        "Python": {
            "baseline": pair(pure_python_metric("checked"), pure_python_metric("unchecked")),
            "advanced": pair(pure_python_metric("checked"), pure_python_metric("unchecked")),
        },
        "Julia": {
            "baseline": pair(m("Julia generic", "checked"), m("Julia generic", "unchecked")),
            "advanced": pair(m("Julia SIMD.jl intrinsics", "checked"), m("Julia SIMD.jl intrinsics", "unchecked")),
        },
        "LuaJIT": {
            "baseline": pair(None, m("LuaJIT scalar outer loop", "")),
            "advanced": pair(m("LuaJIT fixed-shape flat", "checked"), m("LuaJIT fixed-shape flat", "unchecked")),
        },
        "Lua": {
            "baseline": pair(lua["rows"][0]["throughput_millions"], lua["rows"][1]["throughput_millions"]),
            "advanced": pair(lua["rows"][0]["throughput_millions"], lua["rows"][1]["throughput_millions"]),
        },
        "Taichi": {
            "baseline": pair(native_rows["Taichi native Metal, checked"]["throughput_millions"], native_rows["Taichi native Metal, unchecked"]["throughput_millions"]),
            "advanced": pair(taichi["rows"][0]["throughput_millions"], taichi["rows"][1]["throughput_millions"]),
        },
        "Halide": {
            "baseline": pair(
                strict_halide_metric("checked") or min_m("Halide checked"),
                strict_halide_metric("unchecked") or min_m("Halide unchecked"),
            ),
            "advanced": pair(
                strict_halide_metric("checked") or min_m("Halide checked"),
                strict_halide_metric("unchecked") or min_m("Halide unchecked"),
            ),
        },
        "Futhark": {
            "baseline": pair(min_m("Futhark multicore 1 threads checked"), min_m("Futhark multicore 1 threads unchecked")),
            "advanced": pair(min_m("Futhark multicore 8 threads checked"), min_m("Futhark multicore 8 threads unchecked")),
        },
    }
    # The legacy cross-language file predates the compact NumPy controls.
    if result["NumPy"]["baseline"]["unchecked"] is None:
        result["NumPy"]["baseline"]["unchecked"] = m("NumPy scalar outer loop", "")
    if result["NumPy"]["advanced"]["unchecked"] is None:
        result["NumPy"]["advanced"]["unchecked"] = m("NumPy vectorized fixed-shape", "unchecked")
    if result["NumPy"]["advanced"]["checked"] is None:
        result["NumPy"]["advanced"]["checked"] = m("NumPy vectorized fixed-shape", "checked")
    if result["NumPy"]["baseline"]["checked"] is None:
        result["NumPy"]["baseline"]["checked"] = None
    return result


def build_report(
    cross: dict,
    native: dict,
    taichi: dict,
    lua: dict,
    minimal: dict | None = None,
    strict_mech: dict | None = None,
    strict_halide: dict | None = None,
    strict_julia: dict | None = None,
    pure_python: dict | None = None,
    maxima: dict[str, dict[str, dict[str, dict[str, object]]]] | None = None,
) -> dict:
    base = read(BASE_MECH)
    variants_throughput = throughput_variants(
        cross,
        native,
        taichi,
        lua,
        minimal,
        strict_mech,
        strict_halide,
        pure_python,
    )
    cross_config = cross.get("configuration", {})
    native_config = (strict_mech or native).get("configuration", {})
    minimal_config = (minimal or {}).get("configuration", {})
    halide_config = (strict_halide or minimal or {}).get("configuration", {})

    def extent(config: dict, key: str) -> str:
        value = config.get(key, "?")
        return f"{value:,}" if isinstance(value, int) else str(value)

    scalar_workload = f"{extent(cross_config, 'scalar_instances')} x {extent(cross_config, 'scalar_turns')}"
    native_workload = f"{extent(native_config, 'instances')} x {extent(native_config, 'turns')}"
    minimal_workload = f"{extent(minimal_config, 'instances')} x {extent(minimal_config, 'turns')}"
    halide_workload = f"{extent(halide_config, 'instances')} x {extent(halide_config, 'turns')}"
    workload = {
        "Mech": f"{scalar_workload} -> {native_workload}",
        "Taichi": f"{native_workload} -> {native_workload}",
        "Halide": f"{halide_workload} -> {halide_workload}",
        "Futhark": f"{minimal_workload} -> {minimal_workload}",
    }
    rows = []
    for variant in VARIANTS:
        baseline_path = ROOT / variant["baseline"]
        advanced_path = ROOT / variant["advanced"]
        baseline = read(baseline_path)
        advanced = read(advanced_path)
        rows.append(
            {
                **variant,
                "baseline_lines": len(baseline.splitlines()),
                "baseline_chars": len(baseline),
                "advanced_lines": len(advanced.splitlines()),
                "advanced_chars": len(advanced),
                "baseline_code": source_metrics(baseline, baseline_path),
                "advanced_code": source_metrics(advanced, advanced_path),
                "factors": FACTORS[variant["language"]],
                "throughput_variants": variants_throughput[variant["language"]],
                "workload": workload.get(variant["language"], f"{scalar_workload} -> {scalar_workload}"),
                "baseline_to_advanced": diff_metrics(baseline, advanced),
                "baseline_to_base_mech": diff_metrics(base, baseline),
                "advanced_to_base_mech": diff_metrics(base, advanced),
                "throughput_millions": variants_throughput[variant["language"]]["advanced"],
                "performance_maxima": (maxima or {}).get(variant["language"], {}),
            }
        )
    return {
        "schema_version": 1,
        "base_mech": str(BASE_MECH.relative_to(ROOT)),
        "reference_mech": str(REFERENCE_MECH.relative_to(ROOT)),
        "benchmark_evidence": {
            "cross_language": cross.get("generated_at"),
            "native": native.get("generated_at"),
            "strict_mech": (strict_mech or {}).get("generated_at"),
            "strict_halide": (strict_halide or {}).get("generated_at"),
            "strict_julia": (strict_julia or {}).get("generated_at"),
            "pure_python": (pure_python or {}).get("generated_at"),
            "taichi": taichi.get("generated_at"),
            "lua": lua.get("generated_at"),
            "minimal": (minimal or {}).get("generated_at"),
        },
        "definition": "Code lines/chars exclude blank lines and full-line comments (and Mech section separators); changed line slots count the larger side of each non-equal diff block; changed characters count the larger character span within those changed line blocks. The vs Mech columns compare against the compact Mech source; the full reference path is retained separately. The max single-core, SIMD/multicore, and synchronized GPU columns are maxima by family and contract from the canonical ranked throughput table. Single-thread SIMD/JIT rows remain in single-core; the SIMD/multicore class requires an explicit worker, thread, pool, or parallel marker; multi-turn/fused GPU rows are retained separately as gpu_batched maxima. This is an edit-size measure, not a claim about semantic difficulty.",
        "mech_backend_support_delta": mech_support_delta(),
        "rows": rows,
    }


def markdown(report: dict) -> str:
    lines = [
        "# Parallel EKF source-edit cost",
        "",
        "This report measures source edits and runtime factors behind the parallel EKF variants. Source sizes count non-empty, non-comment code only, so comments and formatting do not make a control look larger. `Edit L/C` is the line/character span changed from baseline to advanced; the two `vs Mech` columns use the same metric against the compact checked-in Mech EKF source. The full teaching listing and each row's exact baseline-to-advanced workload are retained in the JSON. Throughput is reported for both baseline and advanced controls, with checked and unchecked kept separate; their headers identify the row workload. The three max columns are the best retained result in that execution class for each family, shown as checked / unchecked M/s; GPU maxima use synchronized per-turn rows. Throughput provenance, including strict Mech and Halide evidence when present, is recorded in the JSON `benchmark_evidence` field.",
        "",
        "## Variant matrix",
        "",
        "| Language | Baseline model | Advanced model | Baseline L/C | Advanced L/C | Edit L/C | Baseline vs Mech L/C | Advanced vs Mech L/C | Baseline checked M/s (baseline row workload) | Baseline unchecked M/s (baseline row workload) | Advanced checked M/s (advanced row workload) | Advanced unchecked M/s (advanced row workload) | Max single-core M/s (10,000 x 20; checked / unchecked) | Max SIMD/multicore M/s (500,000 x 40 where available; checked / unchecked) | Max GPU M/s (500,000 x 40 synchronized per-turn; checked / unchecked) |",
        "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    def maximum_cell(row: dict, category: str) -> str:
        values = row.get("performance_maxima", {}).get(category, {})
        checked = values.get("checked")
        unchecked = values.get("unchecked")

        def value(entry: dict[str, object] | None) -> str:
            return "--" if entry is None else f"{float(entry['throughput']):.3f}"

        return f"{value(checked)} / {value(unchecked)}"

    for row in report["rows"]:
        baseline_throughput = row["throughput_variants"]["baseline"]
        advanced_throughput = row["throughput_variants"]["advanced"]
        baseline_checked = "--" if baseline_throughput["checked"] is None else f"{baseline_throughput['checked']:.3f}"
        baseline_unchecked = "--" if baseline_throughput["unchecked"] is None else f"{baseline_throughput['unchecked']:.3f}"
        advanced_checked = "--" if advanced_throughput["checked"] is None else f"{advanced_throughput['checked']:.3f}"
        advanced_unchecked = "--" if advanced_throughput["unchecked"] is None else f"{advanced_throughput['unchecked']:.3f}"
        lines.append(
            f"| {row['language']} | {row['baseline_label']} | {row['advanced_label']} | {row['baseline_code']['lines']} / {row['baseline_code']['chars']:,} | {row['advanced_code']['lines']} / {row['advanced_code']['chars']:,} | {row['baseline_to_advanced']['changed_line_slots']} / {row['baseline_to_advanced']['changed_chars']:,} | {row['baseline_to_base_mech']['changed_line_slots']} / {row['baseline_to_base_mech']['changed_chars']:,} | {row['advanced_to_base_mech']['changed_line_slots']} / {row['advanced_to_base_mech']['changed_chars']:,} | {baseline_checked} | {baseline_unchecked} | {advanced_checked} | {advanced_unchecked} | {maximum_cell(row, 'single_core')} | {maximum_cell(row, 'simd_multicore')} | {maximum_cell(row, 'gpu')} |"
        )
    lines += [
        "",
        "## Runtime factors",
        "",
        "| Language | Data layout | Turn/dispatch boundary | Validation and publication |",
        "| --- | --- | --- | --- |",
    ]
    for row in report["rows"]:
        factor = FACTORS[row["language"]]
        lines.append(f"| {row['language']} | {factor['layout']} | {factor['boundary']} | {factor['contract']} |")
    lines += [
        "",
        "## Interpretation",
        "",
        "`--` means that exact checked/unchecked baseline was not part of the retained evidence; it is not a zero-throughput result. Futhark baseline/advanced values differ only by worker count, while Halide, Mech, and the pure-Python control keep the same source across both sides. The source pair and execution-boundary columns make those cases explicit.",
        "Max columns are checked / unchecked M/s. The GPU column uses synchronized/per-turn GPU rows only. Single-thread SIMD/JIT rows remain in the single-core column; the SIMD/multicore column requires an explicit worker, thread, pool, or parallel marker. Multi-turn/fused GPU maxima are retained under gpu_batched in the JSON and in the ranked throughput table; Mech's 3,729.673 M/s one-submit control is a device-resident ceiling, not an equivalent synchronized GPU lane.",
        "",
    ]
    for row in report["rows"]:
        edit = row["baseline_to_advanced"]
        lines.append(f"- **{row['language']}**: {row['note']} Baseline -> advanced touches **{edit['changed_line_slots']} lines / {edit['changed_chars']} characters**.")
    support = report["mech_backend_support_delta"]
    lines += ["", f"## Mech backend support footprint", "", f"The high-level Mech source delta is zero, but the native-Metal backend support changed **{support['changed_line_slots']} line slots** ({support['added_lines']} added / {support['deleted_lines']} deleted) across the backend files in the report JSON. This is intentionally reported separately: generated WGSL/MSL is a build artifact, not a second user program.", "", "The Mech row deliberately reports zero high-level source edits: the same `.mec` recurrence feeds the scalar, SIMD, JIT, WGPU, and native-Metal backends. Conversely, Taichi, Julia, Rust, and LuaJIT advanced rows include their source-level layout or execution changes.", ""]
    return "\n".join(lines)


def svg(report: dict) -> str:
    rows = sorted(report["rows"], key=lambda row: row["baseline_to_advanced"]["changed_line_slots"])
    width, left, right, top, row_height, bottom = 1700, 240, 140, 120, 40, 90
    chart_width = width - left - right
    max_lines = max(1, max(row["baseline_to_advanced"]["changed_line_slots"] for row in rows))
    max_chars = max(1, max(row["baseline_to_advanced"]["changed_chars"] for row in rows))
    panel_width = (chart_width - 80) / 2
    height = top + row_height * len(rows) + bottom

    def esc(value: object) -> str:
        return html.escape(str(value), quote=True)

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#91a0b5}.grid{stroke:#263246;stroke-width:1}.label{font-size:14px}.value{font-size:13px;font-variant-numeric:tabular-nums}</style>',
        '<text x="42" y="42" font-size="26" font-weight="700">Parallel EKF source-edit cost</text>',
        '<text x="42" y="68" class="muted" font-size="15">Changed source between baseline and advanced variants; zero means the runtime/backend changed instead of the program source</text>',
        f'<text x="{left}" y="98" font-size="15" font-weight="600">Changed line slots</text>',
        f'<text x="{left + panel_width + 80}" y="98" font-size="15" font-weight="600">Changed characters</text>',
    ]
    for tick in range(0, 6):
        frac = tick / 5
        lx = left + panel_width * frac
        cx = left + panel_width + 80 + panel_width * frac
        lines.append(f'<line x1="{lx:.1f}" y1="{top - 12}" x2="{lx:.1f}" y2="{height - bottom}" class="grid"/>')
        lines.append(f'<line x1="{cx:.1f}" y1="{top - 12}" x2="{cx:.1f}" y2="{height - bottom}" class="grid"/>')
        lines.append(f'<text x="{lx:.1f}" y="{height - bottom + 22}" text-anchor="middle" class="muted" font-size="12">{int(max_lines * frac)}</text>')
        lines.append(f'<text x="{cx:.1f}" y="{height - bottom + 22}" text-anchor="middle" class="muted" font-size="12">{int(max_chars * frac):,}</text>')
    for index, row in enumerate(rows):
        y = top + index * row_height
        color = COLORS[row["language"]]
        line_value = row["baseline_to_advanced"]["changed_line_slots"]
        char_value = row["baseline_to_advanced"]["changed_chars"]
        line_width = panel_width * line_value / max_lines
        char_width = panel_width * char_value / max_chars
        lines.append(f'<text x="{left - 14}" y="{y + 17}" text-anchor="end" class="label">{esc(row["language"])}</text>')
        lines.append(f'<rect x="{left}" y="{y + 3}" width="{max(1, line_width):.1f}" height="22" rx="3" fill="{color}" opacity="0.9"/>')
        lines.append(f'<text x="{min(left + line_width + 8, left + panel_width - 8):.1f}" y="{y + 18}" class="value">{line_value}</text>')
        char_x = left + panel_width + 80
        lines.append(f'<rect x="{char_x}" y="{y + 3}" width="{max(1, char_width):.1f}" height="22" rx="3" fill="{color}" opacity="0.9"/>')
        lines.append(f'<text x="{min(char_x + char_width + 8, char_x + panel_width - 8):.1f}" y="{y + 18}" class="value">{char_value:,}</text>')
    support = report["mech_backend_support_delta"]
    lines.append(f'<text x="42" y="{height - 35}" class="muted" font-size="12">Mech uses the same high-level `.mec` source for every backend; native-Metal support changed {support["changed_line_slots"]} backend line slots and is intentionally not counted as program edits.</text>')
    lines.append('</svg>')
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("cross_language", type=Path)
    parser.add_argument("native", type=Path)
    parser.add_argument("taichi", type=Path)
    parser.add_argument("lua", type=Path)
    parser.add_argument("output_directory", type=Path)
    parser.add_argument("--strict-mech", type=Path)
    parser.add_argument("--strict-halide", type=Path)
    parser.add_argument("--strict-julia", type=Path)
    parser.add_argument("--pure-python", type=Path)
    parser.add_argument("--throughput-table", type=Path)
    args = parser.parse_args()
    strict_mech_path = args.strict_mech or (args.output_directory / "apple-m1-mech-halide-strict-2026-08-31.json")
    strict_halide_path = args.strict_halide or (args.output_directory / "apple-m1-halide-metal-strict-2026-08-31.json")
    strict_julia_path = args.strict_julia or (args.output_directory / "apple-m1-julia-metal-2026-08-31.json")
    pure_python_path = args.pure_python or (args.output_directory / "apple-m1-pure-python-2026-09-01.json")
    throughput_table_path = args.throughput_table or (args.output_directory / "parallel-ekf-throughput-table.md")
    maxima = performance_maxima(throughput_table_path) if throughput_table_path.exists() else None
    report = build_report(
        json.loads(args.cross_language.read_text(encoding="utf-8")),
        json.loads(args.native.read_text(encoding="utf-8")),
        json.loads(args.taichi.read_text(encoding="utf-8")),
        json.loads(args.lua.read_text(encoding="utf-8")),
        json.loads((args.output_directory / "apple-m1-minimal-source-2026-08-31.json").read_text(encoding="utf-8"))
        if (args.output_directory / "apple-m1-minimal-source-2026-08-31.json").exists()
        else None,
        json.loads(strict_mech_path.read_text(encoding="utf-8")) if strict_mech_path.exists() else None,
        json.loads(strict_halide_path.read_text(encoding="utf-8")) if strict_halide_path.exists() else None,
        json.loads(strict_julia_path.read_text(encoding="utf-8")) if strict_julia_path.exists() else None,
        json.loads(pure_python_path.read_text(encoding="utf-8")) if pure_python_path.exists() else None,
        maxima,
    )
    args.output_directory.mkdir(parents=True, exist_ok=True)
    (args.output_directory / "parallel-ekf-source-diff-report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    (args.output_directory / "parallel-ekf-source-diff-report.md").write_text(markdown(report), encoding="utf-8")
    (args.output_directory / "parallel-ekf-source-edit-cost.svg").write_text(svg(report), encoding="utf-8")


if __name__ == "__main__":
    main()
