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
from pathlib import Path


COLORS = {
    "Mech": "#f4c430",      # Mech brand yellow
    "Rust": "#dea584",      # Rust orange
    "Python": "#3776ab",    # Python blue
    "NumPy": "#4d77cf",     # NumPy blue
    "Julia": "#9558b2",     # Julia purple
    "Lua": "#000080",       # Lua navy
    "LuaJIT": "#5ba37f",    # LuaJIT green
    "Taichi": "#e36b6b",    # Taichi red
}


def esc(value: object) -> str:
    return html.escape(str(value), quote=True)


def _median_mech_throughputs(cross_language: dict) -> dict[str, float]:
    """Recover every explicitly printed Mech flavor from retained stdout."""
    import re
    import statistics

    outputs: list[str] = []
    for name in ("mech_scalar_settings", "mech_backend_settings"):
        outputs.extend(cross_language.get("runs", {}).get(name, {}).get("measured_stdout", []))
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
) -> list[dict[str, object]]:
    cross_scalar = cross_language["summary"]["scalar_outer_loop"]
    cross_mech = cross_language["summary"]["mech_backends_million_ekf_turns_per_second"]
    printed_mech = _median_mech_throughputs(cross_language)
    runtime_rows = {row["label"]: row for row in runtime["rows"]}
    native_rows = {row["label"]: row for row in native["rows"]}

    def scalar(label: str, family: str, mode: str) -> dict[str, object]:
        key = label if label in cross_scalar else f"{label} {mode}"
        return {
            "label": f"{label}, {mode}",
            "family": family,
            "mode": mode,
            "throughput": cross_scalar[key]["ekf_turns_per_second"] / 1_000_000,
        }

    def mech_backend(label: str, family: str, mode: str) -> dict[str, object]:
        aliases = {
            "Mech SIMD": "Mech SIMD (4xf32)",
            "GPU single-submit": "Mech GPU, one submission/turn",
            "GPU checked repeated": "Mech GPU, checked repeated turns",
        }
        throughput = cross_mech.get(aliases.get(label, label))
        if throughput is None:
            throughput = printed_mech[label]
        return {
            "label": label,
            "family": family,
            "mode": mode,
            "throughput": throughput,
        }

    rows = [
        mech_backend("Mech scalar", "Mech", "checked"),
        mech_backend("Mech SIMD", "Mech", "checked"),
        mech_backend("Mech Cranelift JIT", "Mech", "checked"),
        mech_backend("Mech Cranelift JIT checked fast", "Mech", "checked"),
        mech_backend("Mech Cranelift JIT unchecked", "Mech", "unchecked"),
        mech_backend("Mech Cranelift JIT unchecked fast", "Mech", "unchecked"),
        mech_backend("Mech Cranelift SIMD-JIT", "Mech", "checked"),
        mech_backend("Mech Cranelift SIMD-JIT checked fast", "Mech", "checked"),
        mech_backend("Mech Cranelift SIMD-JIT unchecked", "Mech", "unchecked"),
        mech_backend("Mech Cranelift SIMD-JIT unchecked fast", "Mech", "unchecked"),
        mech_backend("GPU single-submit", "Mech", "checked"),
        mech_backend("GPU checked repeated", "Mech", "checked"),
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
        scalar("NumPy scalar outer loop", "Python", "unchecked"),
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
    max_value = max(10.0, ((max(float(row["throughput"]) for row in visible) + 19.999) // 20) * 20)
    height = top + row_height * len(visible) + bottom

    def x(value: float) -> float:
        return left + chart_width * value / max_value

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5} .muted{fill:#91a0b5} .grid{stroke:#263246;stroke-width:1} .axis{fill:#91a0b5;font-size:13px} .label{font-size:14px} .value{font-size:13px;font-variant-numeric:tabular-nums}</style>',
        f'<text x="52" y="42" font-size="26" font-weight="700">Cross-language EKF runtime throughput ({esc(mode)})</text>',
        f'<text x="52" y="68" class="muted" font-size="15">Apple M1 | CPU/language: {scalar_instances:,}x{scalar_turns}; Mech backend: {backend_instances:,}x{backend_turns}; matched GPU runtime/native: {runtime_instances:,}x{runtime_turns} | steady-state, sorted</text>',
    ]
    for tick in range(0, int(max_value) + 1, 20):
        tick_x = x(tick)
        lines.append(f'<line x1="{tick_x:.1f}" y1="{top - 18}" x2="{tick_x:.1f}" y2="{height - bottom + 4}" class="grid"/>')
        lines.append(f'<text x="{tick_x:.1f}" y="{height - bottom + 28}" text-anchor="middle" class="axis">{tick}</text>')
    lines.append(f'<text x="{left + chart_width / 2:.1f}" y="{height - 28}" text-anchor="middle" class="muted" font-size="14">million EKF turns per second</text>')

    legend = list(COLORS)
    legend_x = width - right - 460
    for index, family in enumerate(legend):
        x_pos = legend_x + (index % 4) * 115
        y_pos = 27 + (index // 4) * 22
        lines.append(f'<rect x="{x_pos}" y="{y_pos - 11}" width="14" height="14" rx="2" fill="{COLORS[family]}"/>')
        lines.append(f'<text x="{x_pos + 22}" y="{y_pos}" font-size="13">{family}</text>')

    for index, row in enumerate(visible):
        value = float(row["throughput"])
        y = top + index * row_height
        bar_width = max(1.0, chart_width * value / max_value)
        color = COLORS[str(row["family"])]
        lines.append(f'<text x="{left - 16}" y="{y + 19}" text-anchor="end" class="label">{esc(row["label"])}</text>')
        lines.append(f'<rect x="{left}" y="{y + 5}" width="{bar_width:.1f}" height="19" rx="3" fill="{color}" opacity="0.9"/>')
        value_x = min(left + bar_width + 9, width - right + 10)
        lines.append(f'<text x="{value_x:.1f}" y="{y + 19}" class="value">{value:.2f}</text>')

    note = "Checked rows include candidate validation/publication; unchecked rows explicitly omit those guarantees. "
    note += "Native Metal rows are direct command submission; WGPU rows are retained as a portable transport control. "
    note += "Compilation, allocation, warmup, and final readback are excluded from the timed region."
    lines.append(f'<text x="52" y="{height - 55}" class="muted" font-size="12">{esc(note)}</text>')
    lines.append('</svg>')
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("cross_language", type=Path)
    parser.add_argument("runtime", type=Path)
    parser.add_argument("native", type=Path)
    parser.add_argument("output_directory", type=Path)
    parser.add_argument("lua", type=Path, nargs="?", help="plain Lua evidence JSON")
    parser.add_argument("--taichi-optimized", type=Path, help="optimized Taichi evidence JSON")
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
    rows = load_rows(cross_language, runtime, native, lua, taichi_optimized)
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
