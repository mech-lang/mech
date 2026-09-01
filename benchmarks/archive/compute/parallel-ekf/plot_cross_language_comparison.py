#!/usr/bin/env python3
"""Render checked and unchecked cross-language EKF throughput charts.

The benchmark archive intentionally keeps raw stdout in separate evidence
files. This renderer joins those files by the stable summary labels and emits
one SVG per contract, so a new run cannot silently change the axis or mix
checked and unchecked rows.
"""

from __future__ import annotations

import argparse
import html
import json
import math
from pathlib import Path


COLORS = {
    "Mech": "#f4c430",      # Mech brand yellow
    "Rust": "#dea584",      # Rust orange
    "NumPy": "#4d77cf",     # NumPy blue
    "Julia": "#9558b2",     # Julia purple
    "Lua": "#000080",       # Lua navy
    "LuaJIT": "#5ba37f",    # LuaJIT green
    "Taichi": "#e36b6b",    # Taichi red
    "Halide": "#ff8f00",    # Halide orange
    "Futhark": "#e94f37",   # Futhark red
}


def esc(value: object) -> str:
    return html.escape(str(value), quote=True)


def is_gpu_row(row: dict[str, object]) -> bool:
    label = str(row["label"])
    return "GPU" in label or "native Metal" in label


def _median_mech_throughputs(cross_language: dict) -> dict[str, float]:
    """Recover backend-only Mech flavors from backend-setting stdout."""
    import re
    import statistics

    outputs: list[str] = []
    outputs.extend(
        cross_language.get("runs", {})
        .get("mech_backend_settings", {})
        .get("measured_stdout", [])
    )
    values: dict[str, list[float]] = {}
    for text in outputs:
        for match in re.finditer(
            r"^(Mech .+? throughput|GPU .+? throughput): ([0-9.]+) million",
            text,
            flags=re.MULTILINE,
        ):
            values.setdefault(match.group(1)[: -len(" throughput")], []).append(float(match.group(2)))
    return {label: statistics.median(samples) for label, samples in values.items()}


def load_rows(
    cross_language: dict,
    runtime: dict,
    native: dict,
    lua: dict | None = None,
    taichi_optimized: dict | None = None,
    minimal: dict | None = None,
    julia_threaded: dict | None = None,
    numpy_numba: dict | None = None,
    simd_controls: dict | None = None,
    julia_gpu: dict | None = None,
    halide_gpu: dict | None = None,
    numpy_gpu: dict | None = None,
    futhark_fixed: dict | None = None,
    mech_persistent: dict | None = None,
    fused_references: dict | None = None,
) -> list[dict[str, object]]:
    cross_scalar = cross_language["summary"]["scalar_outer_loop"]
    cross_mech = cross_language["summary"]["mech_backends_million_ekf_turns_per_second"]
    printed_mech = _median_mech_throughputs(cross_language)
    runtime_rows = {row["label"]: row for row in runtime["rows"]}
    native_rows = {row["label"]: row for row in native["rows"]}

    def scalar(label: str, family: str, mode: str) -> dict[str, object]:
        key = f"{label} {mode}"
        if key not in cross_scalar:
            key = label
        return {
            "label": f"{label}, {mode}",
            "family": family,
            "mode": mode,
            "throughput": cross_scalar[key]["ekf_turns_per_second"] / 1_000_000,
        }

    def mech_backend(label: str, family: str, mode: str) -> dict[str, object]:
        aliases = {
            "Mech SIMD": "Mech SIMD (4xf32)",
            "Mech GPU, checked one-turn": "Mech GPU, checked one-turn API call",
            "Mech GPU, checked repeated": "Mech GPU, checked repeated API call",
            "Mech GPU, unchecked one-turn": "Mech GPU, unchecked one-turn API call",
            "Mech GPU, unchecked repeated": "Mech GPU, unchecked repeated dispatches",
            "Mech GPU, unchecked one-submit": "Mech GPU, unchecked one submission",
            "Mech GPU, unchecked ping-pong one-turn": "GPU unchecked ping-pong one-turn",
            "Mech GPU, unchecked in-place one-turn": "GPU unchecked in-place one-turn",
            "Mech GPU, unchecked in-place repeated": "GPU unchecked in-place repeated",
        }
        key = aliases.get(label, label)
        # JIT lanes are stored in the scalar summary because their resident
        # CPU loop is the same harness as the language controls. Backend-only
        # lanes remain in the backend summary.
        if key in cross_scalar:
            throughput = cross_scalar[key]["ekf_turns_per_second"] / 1_000_000
        else:
            throughput = cross_mech.get(key)
        if throughput is None:
            throughput = printed_mech[key]
        return {
            "label": label,
            "family": family,
            "mode": mode,
            "throughput": throughput,
        }

    rows = [
        scalar("Mech scalar", "Mech", "checked"),
        scalar("Mech scalar", "Mech", "unchecked"),
        mech_backend("Mech SIMD", "Mech", "checked"),
        mech_backend("Mech Cranelift JIT", "Mech", "checked"),
        mech_backend("Mech Cranelift JIT checked fast", "Mech", "checked"),
        mech_backend("Mech Cranelift JIT unchecked", "Mech", "unchecked"),
        mech_backend("Mech Cranelift JIT unchecked fast", "Mech", "unchecked"),
        mech_backend("Mech Cranelift SIMD-JIT", "Mech", "checked"),
        mech_backend("Mech Cranelift SIMD-JIT checked fast", "Mech", "checked"),
        mech_backend("Mech Cranelift SIMD-JIT parallel", "Mech", "checked"),
        mech_backend("Mech Cranelift SIMD-JIT unchecked", "Mech", "unchecked"),
        mech_backend("Mech Cranelift SIMD-JIT unchecked fast", "Mech", "unchecked"),
        mech_backend("Mech Cranelift SIMD-JIT parallel unchecked fast", "Mech", "unchecked"),
        mech_backend("Mech GPU, checked one-turn", "Mech", "checked"),
        mech_backend("Mech GPU, checked repeated", "Mech", "checked"),
        mech_backend("Mech GPU, unchecked one-turn", "Mech", "unchecked"),
        mech_backend("Mech GPU, unchecked repeated", "Mech", "unchecked"),
        mech_backend("Mech GPU, unchecked ping-pong one-turn", "Mech", "unchecked"),
        mech_backend("Mech GPU, unchecked in-place one-turn", "Mech", "unchecked"),
        mech_backend("Mech GPU, unchecked in-place repeated", "Mech", "unchecked"),
        mech_backend("Mech GPU, unchecked one-submit", "Mech", "unchecked"),
        {
            "label": "Mech SIMD/JIT CPU, 8 workers",
            "family": "Mech",
            "mode": "checked",
            "throughput": runtime_rows["Mech SIMD/JIT CPU, checked (8 workers)"]["throughput_millions"],
        },
        {
            "label": "Mech SIMD/JIT CPU, 8 workers",
            "family": "Mech",
            "mode": "unchecked",
            "throughput": runtime_rows["Mech SIMD/JIT CPU, unchecked (8 workers)"]["throughput_millions"],
        },
        {
            "label": "Taichi LLVM CPU, 8 workers",
            "family": "Taichi",
            "mode": "checked",
            "throughput": runtime_rows["Taichi LLVM CPU, checked (8 workers)"]["throughput_millions"],
        },
        {
            "label": "Taichi LLVM CPU, 8 workers",
            "family": "Taichi",
            "mode": "unchecked",
            "throughput": runtime_rows["Taichi LLVM CPU, unchecked (8 workers)"]["throughput_millions"],
        },
        {
            "label": "Mech GPU, WGPU per-turn",
            "family": "Mech",
            "mode": "checked",
            "throughput": runtime_rows["Mech WGPU GPU, checked"]["throughput_millions"],
        },
        {
            "label": "Mech GPU, WGPU per-turn",
            "family": "Mech",
            "mode": "unchecked",
            "throughput": runtime_rows["Mech WGPU GPU, unchecked"]["throughput_millions"],
        },
        {
            "label": "Mech GPU, native Metal",
            "family": "Mech",
            "mode": "checked",
            "throughput": native_rows["Mech native Metal, checked"]["throughput_millions"],
        },
        {
            "label": "Mech GPU, native Metal",
            "family": "Mech",
            "mode": "unchecked",
            "throughput": native_rows["Mech native Metal, unchecked"]["throughput_millions"],
        },
        scalar("Rust packed SIMD", "Rust", "checked"),
        scalar("Rust optimized fixed-shape", "Rust", "unchecked"),
        scalar("Rust packed SIMD", "Rust", "unchecked"),
        scalar("Julia generic", "Julia", "checked"),
        scalar("Julia generic", "Julia", "unchecked"),
        scalar("Julia fixed-shape", "Julia", "checked"),
        scalar("Julia fixed-shape", "Julia", "unchecked"),
        scalar("Julia fixed-shape SIMD", "Julia", "checked"),
        scalar("Julia fixed-shape SIMD", "Julia", "unchecked"),
        scalar("Julia SIMD.jl intrinsics", "Julia", "checked"),
        scalar("Julia SIMD.jl intrinsics", "Julia", "unchecked"),
        scalar("NumPy vectorized fixed-shape", "NumPy", "checked"),
        scalar("NumPy vectorized fixed-shape", "NumPy", "unchecked"),
        scalar("NumPy scalar outer loop", "NumPy", "unchecked"),
        scalar("LuaJIT fixed-shape flat", "LuaJIT", "checked"),
        scalar("LuaJIT fixed-shape flat", "LuaJIT", "unchecked"),
        scalar("LuaJIT scalar outer loop", "LuaJIT", "unchecked"),
        {
            "label": "Taichi GPU, native Metal",
            "family": "Taichi",
            "mode": "checked",
            "throughput": native_rows["Taichi native Metal, checked"]["throughput_millions"],
        },
        {
            "label": "Taichi GPU, native Metal",
            "family": "Taichi",
            "mode": "unchecked",
            "throughput": native_rows["Taichi native Metal, unchecked"]["throughput_millions"],
        },
    ]
    if lua is not None:
        rows.extend(
            {
                "label": row["label"],
                "family": "Lua",
                "mode": row["mode"],
                "throughput": row["throughput_millions"],
            }
            for row in lua["rows"]
        )
    if taichi_optimized is not None:
        rows.extend(
            {
                "label": row["label"],
                "family": "Taichi",
                "mode": row["mode"],
                "throughput": row["throughput_millions"],
            }
            for row in taichi_optimized["rows"]
        )
    if minimal is not None:
        for label, family in (
            ("Halide checked", "Halide"),
            ("Halide unchecked", "Halide"),
            ("Futhark multicore 8 threads checked", "Futhark"),
            ("Futhark multicore 8 threads unchecked", "Futhark"),
        ):
            row = minimal.get("rows", {}).get(label)
            if row is not None and "throughput" in row:
                import statistics

                rows.append(
                    {
                        "label": family + ", " + ("unchecked" if label.endswith("unchecked") else "checked"),
                        "family": family,
                        "mode": "unchecked" if label.endswith("unchecked") else "checked",
                        "throughput": statistics.median(row["throughput"]) / 1_000_000,
                    }
                )
    if julia_threaded is not None:
        for mode in ("checked", "unchecked"):
            row = julia_threaded.get("rows", {}).get(mode)
            if row is not None and "throughput_millions" in row:
                import statistics

                rows.append(
                    {
                        "label": "Julia SIMD.jl, 8 workers",
                        "family": "Julia",
                        "mode": mode,
                        "throughput": statistics.median(row["throughput_millions"]),
                    }
                )
    if numpy_numba is not None:
        import statistics

        for mode in ("checked", "unchecked"):
            row = numpy_numba.get("rows", {}).get(mode)
            if row is not None and "throughput_millions" in row:
                rows.append(
                    {
                        "label": "NumPy/Numba parallel JIT, 8 workers",
                        "family": "NumPy",
                        "mode": mode,
                        "throughput": statistics.median(row["throughput_millions"]),
                    }
                )
    if simd_controls is not None:
        import statistics

        for label, family in (
            ("Halide JIT SIMD 8 workers", "Halide"),
            ("Futhark ISPC SIMD 8 workers", "Futhark"),
        ):
            if family == "Futhark" and futhark_fixed is not None:
                # The old row is a 10k x 20 dynamic-mode run. Once matched
                # fixed-mode evidence is supplied, retaining it unlabelled
                # would recreate the misleading ranking this control fixes.
                continue
            for mode in ("checked", "unchecked"):
                row = simd_controls.get("rows", {}).get(f"{label} {mode}")
                if row is not None and "throughput_millions" in row:
                    rows.append(
                        {
                            "label": f"{label}, {mode}",
                            "family": family,
                            "mode": mode,
                            "throughput": statistics.median(row["throughput_millions"]),
                        }
                    )
    if julia_gpu is not None:
        import statistics

        for mode in ("checked", "unchecked"):
            row = julia_gpu.get("rows", {}).get(mode)
            if row is not None and "throughput_millions" in row:
                rows.append(
                    {
                        "label": "Julia GPU, native Metal",
                        "family": "Julia",
                        "mode": mode,
                        "throughput": statistics.median(row["throughput_millions"]),
                    }
                )
    if halide_gpu is not None:
        import statistics

        strict_halide = "checked_fault_observation" in halide_gpu.get("configuration", {})
        halide_label = (
            "Halide GPU, native Metal (strict fault-observing)"
            if strict_halide
            else "Halide GPU, native Metal (fused kernel control)"
        )
        for mode in ("checked", "unchecked"):
            row = halide_gpu.get("rows", {}).get(f"Halide GPU Metal {mode}")
            if row is not None and "throughput" in row:
                rows.append(
                    {
                        "label": halide_label,
                        "family": "Halide",
                        "mode": mode,
                        "throughput": statistics.median(row["throughput"]) / 1_000_000,
                    }
                )
    # NumPy has no native GPU backend on the Apple M1.  Keep the evidence file
    # in the report inputs so the absence is auditable, but never turn an
    # unavailable backend into a zero-throughput chart row.
    _ = numpy_gpu
    if futhark_fixed is not None:
        import statistics

        # The raw evidence also retains dynamic-mode runs for diagnostics, but
        # the chart presents one canonical matched Futhark ISPC result rather
        # than separate dynamic and fixed rows. The fixed entry point is the
        # representative steady-state comparison.
        for mode in ("checked", "unchecked"):
            row = futhark_fixed.get("rows", {}).get(mode)
            if row is not None and "throughput_millions" in row:
                rows.append(
                    {
                        "label": "Futhark ISPC fixed-mode, 8 workers (500k x 40)",
                        "family": "Futhark",
                        "mode": mode,
                        "throughput": statistics.median(row["throughput_millions"]),
                    }
                )
    if mech_persistent is not None:
        import statistics

        for key in ("persistent_per_turn_unchecked_fast", "fused_unchecked_block"):
            row = mech_persistent.get("rows", {}).get(key)
            if row is not None and "throughput_millions" in row:
                rows.append(
                    {
                        "label": row.get("label", key),
                        "family": "Mech",
                        "mode": row.get("mode", "unchecked"),
                        "throughput": statistics.median(row["throughput_millions"]),
                    }
                )
    if fused_references is not None:
        import statistics

        for row in fused_references.get("rows", {}).values():
            if "throughput_millions" in row:
                rows.append(
                    {
                        "label": row["label"],
                        "family": row["family"],
                        "mode": row["mode"],
                        "throughput": statistics.median(row["throughput_millions"]),
                    }
                )
    return rows


def render(
    rows: list[dict[str, object]],
    mode: str,
    output: Path,
    scalar_instances: int,
    scalar_turns: int,
    backend_instances: int,
    backend_turns: int,
    runtime_instances: int,
    runtime_turns: int,
) -> None:
    visible = [row for row in rows if row["mode"] == mode]
    visible.sort(key=lambda row: (float(row["throughput"]), str(row["label"])))
    width = 1700
    left = 550
    right = 140
    top = 120
    row_height = 31
    bottom = 100
    chart_width = width - left - right
    # Use the complete checked+unchecked row set so the two output charts have
    # identical logarithmic extents and remain directly comparable.
    positive_values = [float(row["throughput"]) for row in rows if float(row["throughput"]) > 0.0]
    if not positive_values:
        raise ValueError("logarithmic throughput axis requires at least one positive value")
    min_value = 10.0 ** math.floor(math.log10(min(positive_values)))
    max_value = 10.0 ** math.ceil(math.log10(max(positive_values)))
    if min_value == max_value:
        max_value *= 10.0
    log_min = math.log10(min_value)
    log_span = math.log10(max_value) - log_min
    height = top + row_height * len(visible) + bottom

    def x(value: float) -> float:
        if value <= 0.0:
            return left
        return left + chart_width * (math.log10(value) - log_min) / log_span

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<defs>',
        *(
            f'<pattern id="gpu-{family.lower()}" width="8" height="8" patternUnits="userSpaceOnUse" patternTransform="rotate(30)"><rect width="8" height="8" fill="{color}"/><path d="M0 0V8" stroke="#ffffff" stroke-opacity="0.38" stroke-width="2"/></pattern>'
            for family, color in COLORS.items()
        ),
        '</defs>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5} .muted{fill:#91a0b5} .grid{stroke:#263246;stroke-width:1} .minor-grid{stroke:#1b2536;stroke-width:1} .axis{fill:#91a0b5;font-size:13px} .label{font-size:14px} .value{font-size:13px;font-variant-numeric:tabular-nums}</style>',
        f'<text x="52" y="42" font-size="26" font-weight="700">Cross-language EKF runtime throughput ({esc(mode)}; slowest to fastest)</text>',
        f'<text x="52" y="68" class="muted" font-size="15">Apple M1 | CPU/language: {scalar_instances:,}x{scalar_turns}; Mech backend: {backend_instances:,}x{backend_turns}; matched runtime/native controls: {runtime_instances:,}x{runtime_turns} | steady-state, sorted</text>',
    ]
    first_exponent = math.floor(math.log10(min_value))
    last_exponent = math.ceil(math.log10(max_value))
    for exponent in range(first_exponent, last_exponent + 1):
        decade = 10.0 ** exponent
        for multiplier in (1, 2, 5):
            tick = multiplier * decade
            if tick < min_value or tick > max_value:
                continue
            tick_x = x(tick)
            major = multiplier == 1
            grid_class = "grid" if major else "minor-grid"
            lines.append(f'<line x1="{tick_x:.1f}" y1="{top - 18}" x2="{tick_x:.1f}" y2="{height - bottom + 4}" class="{grid_class}"/>')
            lines.append(f'<text x="{tick_x:.1f}" y="{height - bottom + 28}" text-anchor="middle" class="axis">{tick:g}</text>')
    lines.append(f'<text x="{left + chart_width / 2:.1f}" y="{height - 28}" text-anchor="middle" class="muted" font-size="14">million EKF turns per second (log scale)</text>')

    legend = list(COLORS)
    legend_x = width - right - 460
    for index, family in enumerate(legend):
        x_pos = legend_x + (index % 4) * 115
        y_pos = 27 + (index // 4) * 22
        lines.append(f'<rect x="{x_pos}" y="{y_pos - 11}" width="14" height="14" rx="2" fill="{COLORS[family]}"/>')
        lines.append(f'<text x="{x_pos + 22}" y="{y_pos}" font-size="13">{family}</text>')
    key_y = 93
    lines.append(f'<rect x="{legend_x}" y="{key_y - 11}" width="14" height="14" rx="2" fill="{COLORS["Mech"]}"/>')
    lines.append(f'<text x="{legend_x + 22}" y="{key_y}" font-size="13">CPU solid</text>')
    lines.append(f'<rect x="{legend_x + 115}" y="{key_y - 11}" width="14" height="14" rx="2" fill="url(#gpu-mech)"/>')
    lines.append(f'<text x="{legend_x + 137}" y="{key_y}" font-size="13">GPU hatched</text>')

    for index, row in enumerate(visible):
        value = float(row["throughput"])
        y = top + index * row_height
        bar_width = max(1.0, x(value) - left)
        color = COLORS[str(row["family"])]
        fill = f'url(#gpu-{str(row["family"]).lower()})' if is_gpu_row(row) else color
        lines.append(f'<text x="{left - 16}" y="{y + 19}" text-anchor="end" class="label">{esc(row["label"])}</text>')
        lines.append(f'<rect x="{left}" y="{y + 5}" width="{bar_width:.1f}" height="19" rx="3" fill="{fill}" opacity="0.9"/>')
        value_x = min(left + bar_width + 9, width - right + 10)
        lines.append(f'<text x="{value_x:.1f}" y="{y + 19}" class="value">{value:.2f}</text>')

    note = "Rows are ordered by throughput from slowest to fastest. Hatched bars are GPU lanes; solid bars are CPU lanes. Checked rows include candidate validation/publication; unchecked rows explicitly omit those guarantees. "
    note += "Native Metal rows are direct command submission; WGPU rows are retained as a portable transport control. "
    note += "Compilation, allocation, warmup, and final readback are excluded from the timed region."
    lines.append(f'<text x="52" y="{height - 55}" class="muted" font-size="12">{esc(note)}</text>')
    lines.append('</svg>')
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")


def markdown_table(
    rows: list[dict[str, object]],
    output: Path,
    scalar_instances: int,
    scalar_turns: int,
    backend_instances: int,
    backend_turns: int,
    runtime_instances: int,
    runtime_turns: int,
) -> None:
    """Write the exact chart rows as ranked tables for audit-friendly review."""
    lines = [
        "# Parallel EKF throughput table",
        "",
        "This table is generated from the same retained evidence and row set as the SVG charts. Each contract is ranked independently from slowest to fastest; checked and unchecked values are never mixed in one rank.",
        "",
        f"Workloads: CPU/language {scalar_instances:,} filters x {scalar_turns} turns; Mech backend {backend_instances:,} filters x {backend_turns} CPU turns; matched runtime/native controls {runtime_instances:,} filters x {runtime_turns} turns. Setup, compilation, allocation, warmup, and final readback are outside the timed region.",
        "",
    ]
    for mode in ("checked", "unchecked"):
        visible = [row for row in rows if row["mode"] == mode]
        visible.sort(key=lambda row: (float(row["throughput"]), str(row["label"])))
        lines.extend(
            [
                f"## {mode.title()} (slowest to fastest)",
                "",
                "| Rank | Runtime/lane | Family | Million EKF turns/s |",
                "| ---: | --- | --- | ---: |",
            ]
        )
        for rank, row in enumerate(visible, start=1):
            lines.append(
                f"| {rank} | {row['label']} | {row['family']} | {float(row['throughput']):.3f} |"
            )
        lines.append("")
    lines.extend(
        [
            "Checked rows include candidate validation/publication. Unchecked rows explicitly omit those guarantees. The GPU one-submit row is a fused unchecked control and is therefore shown only in the unchecked section.",
            "Futhark GPU has no numeric row on this Apple M1: Futhark 0.27 exposes CUDA/OpenCL backends but no Metal backend; the generated OpenCL kernel is rejected by Apple's driver.",
            "NumPy GPU has no numeric row on this Apple M1: plain NumPy has no GPU backend and CuPy requires CUDA/NVIDIA. The capability result is retained separately.",
            "",
        ]
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("cross_language", type=Path)
    parser.add_argument("runtime", type=Path)
    parser.add_argument("native", type=Path)
    parser.add_argument("output_directory", type=Path)
    parser.add_argument("lua", type=Path, nargs="?", help="plain Lua evidence JSON")
    parser.add_argument("--taichi-optimized", type=Path, help="optimized Taichi evidence JSON")
    parser.add_argument("--minimal-source", type=Path, help="Halide/Futhark/NumPy minimal-control evidence JSON")
    parser.add_argument("--julia-threaded", type=Path, help="threaded Julia SIMD evidence JSON")
    parser.add_argument("--numpy-numba", type=Path, help="NumPy/Numba threaded JIT evidence JSON")
    parser.add_argument("--simd-controls", type=Path, help="Halide and Futhark SIMD evidence JSON")
    parser.add_argument("--julia-gpu", type=Path, help="Julia Metal GPU evidence JSON")
    parser.add_argument("--halide-gpu", type=Path, help="Halide native Metal GPU evidence JSON")
    parser.add_argument("--numpy-gpu", type=Path, help="NumPy GPU capability evidence JSON")
    parser.add_argument("--futhark-fixed", type=Path, help="fixed-mode Futhark ISPC evidence JSON")
    parser.add_argument("--mech-persistent", type=Path, help="persistent Mech SIMD/JIT evidence JSON")
    parser.add_argument(
        "--fused-references",
        type=Path,
        help="fused worker-local Rust/Julia/Numba evidence JSON",
    )
    args = parser.parse_args()
    cross_language = json.loads(args.cross_language.read_text(encoding="utf-8"))
    runtime = json.loads(args.runtime.read_text(encoding="utf-8"))
    native = json.loads(args.native.read_text(encoding="utf-8"))
    lua = json.loads(args.lua.read_text(encoding="utf-8")) if args.lua else None
    taichi_optimized = (
        json.loads(args.taichi_optimized.read_text(encoding="utf-8"))
        if args.taichi_optimized
        else None
    )
    minimal = (
        json.loads(args.minimal_source.read_text(encoding="utf-8"))
        if args.minimal_source
        else None
    )
    julia_threaded = (
        json.loads(args.julia_threaded.read_text(encoding="utf-8"))
        if args.julia_threaded
        else None
    )
    numpy_numba = (
        json.loads(args.numpy_numba.read_text(encoding="utf-8"))
        if args.numpy_numba
        else None
    )
    simd_controls = (
        json.loads(args.simd_controls.read_text(encoding="utf-8"))
        if args.simd_controls
        else None
    )
    julia_gpu = (
        json.loads(args.julia_gpu.read_text(encoding="utf-8"))
        if args.julia_gpu
        else None
    )
    halide_gpu = (
        json.loads(args.halide_gpu.read_text(encoding="utf-8"))
        if args.halide_gpu
        else None
    )
    numpy_gpu = (
        json.loads(args.numpy_gpu.read_text(encoding="utf-8"))
        if args.numpy_gpu
        else None
    )
    futhark_fixed = (
        json.loads(args.futhark_fixed.read_text(encoding="utf-8"))
        if args.futhark_fixed
        else None
    )
    mech_persistent = (
        json.loads(args.mech_persistent.read_text(encoding="utf-8"))
        if args.mech_persistent
        else None
    )
    fused_references = (
        json.loads(args.fused_references.read_text(encoding="utf-8"))
        if args.fused_references
        else None
    )
    rows = load_rows(
        cross_language,
        runtime,
        native,
        lua,
        taichi_optimized,
        minimal,
        julia_threaded,
        numpy_numba,
        simd_controls,
        julia_gpu,
        halide_gpu,
        numpy_gpu,
        futhark_fixed,
        mech_persistent,
        fused_references,
    )
    configuration = cross_language["configuration"]
    render(
        rows,
        "checked",
        args.output_directory / "parallel-ekf-cross-language-checked.svg",
        configuration["scalar_instances"],
        configuration["scalar_turns"],
        configuration["backend_instances"],
        configuration["backend_cpu_turns"],
        runtime["configuration"]["instances"],
        runtime["configuration"]["turns"],
    )
    markdown_table(
        rows,
        args.output_directory / "parallel-ekf-throughput-table.md",
        configuration["scalar_instances"],
        configuration["scalar_turns"],
        configuration["backend_instances"],
        configuration["backend_cpu_turns"],
        runtime["configuration"]["instances"],
        runtime["configuration"]["turns"],
    )
    render(
        rows,
        "unchecked",
        args.output_directory / "parallel-ekf-cross-language-unchecked.svg",
        configuration["scalar_instances"],
        configuration["scalar_turns"],
        configuration["backend_instances"],
        configuration["backend_cpu_turns"],
        runtime["configuration"]["instances"],
        runtime["configuration"]["turns"],
    )


if __name__ == "__main__":
    main()
