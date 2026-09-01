#!/usr/bin/env python3
"""Render a dependency-free SVG from a parallel-EKF evidence JSON file."""

from __future__ import annotations

import argparse
import html
import json
from pathlib import Path


ORDER = [
    ("Mech scalar", "mech"),
    ("Mech scalar unchecked", "mech"),
    ("Mech Cranelift JIT", "mech"),
    ("Mech Cranelift SIMD-JIT", "mech"),
    ("Mech Cranelift SIMD-JIT checked fast", "mech"),
    ("Mech Cranelift SIMD-JIT unchecked fast", "mech"),
    ("Rust optimized fixed-shape", "rust"),
    ("Rust packed SIMD checked", "rust"),
    ("Rust packed SIMD unchecked", "rust"),
    ("Julia generic checked", "julia"),
    ("Julia generic unchecked", "julia"),
    ("Julia fixed-shape checked", "julia"),
    ("Julia fixed-shape unchecked", "julia"),
    ("Julia fixed-shape SIMD checked", "julia"),
    ("Julia fixed-shape SIMD unchecked", "julia"),
    ("Julia SIMD.jl intrinsics checked", "julia"),
    ("Julia SIMD.jl intrinsics unchecked", "julia"),
    ("NumPy scalar outer loop", "numpy"),
    ("NumPy vectorized fixed-shape checked", "numpy"),
    ("NumPy vectorized fixed-shape unchecked", "numpy"),
    ("LuaJIT scalar outer loop", "luajit"),
    ("LuaJIT fixed-shape flat checked", "luajit"),
    ("LuaJIT fixed-shape flat unchecked", "luajit"),
    ("Mech GPU, checked one-turn API call", "gpu"),
    ("Mech GPU, checked repeated API call", "gpu"),
    ("Mech GPU, unchecked one-turn API call", "gpu"),
    ("Mech GPU, unchecked repeated dispatches", "gpu"),
    ("Mech GPU, unchecked one submission", "gpu"),
]

CHECKED_ORDER = [
    ("Mech scalar", "mech"),
    ("Mech Cranelift JIT", "mech"),
    ("Mech Cranelift SIMD-JIT", "mech"),
    ("Mech Cranelift SIMD-JIT checked fast", "mech"),
    ("Rust packed SIMD checked", "rust"),
    ("Julia generic checked", "julia"),
    ("Julia fixed-shape checked", "julia"),
    ("Julia fixed-shape SIMD checked", "julia"),
    ("Julia SIMD.jl intrinsics checked", "julia"),
    ("NumPy vectorized fixed-shape checked", "numpy"),
    ("LuaJIT fixed-shape flat checked", "luajit"),
    ("Mech GPU, checked one-turn API call", "gpu"),
    ("Mech GPU, checked repeated API call", "gpu"),
]

COLORS = {
    "mech": "#40d4b0",
    "rust": "#f3a847",
    "julia": "#b68cff",
    "numpy": "#66a6ff",
    "luajit": "#000080",
    "python": "#66a6ff",
    "gpu": "#56c7e8",
}


def esc(value: object) -> str:
    return html.escape(str(value), quote=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("evidence", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument(
        "--checked-only",
        action="store_true",
        help="plot only controls that execute the integrity checks",
    )
    args = parser.parse_args()
    evidence = json.loads(args.evidence.read_text(encoding="utf-8"))
    scalar = evidence["summary"]["scalar_outer_loop"]
    backend = evidence["summary"]["mech_backends_million_ekf_turns_per_second"]
    configuration = evidence["configuration"]
    order = CHECKED_ORDER if args.checked_only else ORDER
    values = {
        label: float(
            backend[label]
            if label in backend
            else scalar[label]["ekf_turns_per_second"] / 1_000_000
        )
        for label, _ in order
    }
    rust_reference = (
        values["Rust packed SIMD checked"]
        if args.checked_only
        else values["Rust packed SIMD unchecked"]
    )

    width = 1500
    left = 430
    right = 120
    top = 120
    row_height = 31
    bottom = 100
    chart_width = width - left - right
    max_value = max(60.0, ((max(values.values()) + 9.999) // 10) * 10)
    height = top + row_height * len(order) + bottom

    def x(value: float) -> float:
        return left + chart_width * value / max_value

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5} .muted{fill:#91a0b5} .grid{stroke:#263246;stroke-width:1} .axis{fill:#91a0b5;font-size:13px} .label{font-size:14px} .value{font-size:13px;font-variant-numeric:tabular-nums}</style>',
        f'<text x="52" y="42" font-size="26" font-weight="700">Parallel EKF steady-state throughput{" (checked only)" if args.checked_only else ""}</text>',
        f'<text x="52" y="68" class="muted" font-size="15">Apple M1 | backend: {configuration["backend_instances"]:,} filters x {configuration["backend_cpu_turns"]} turns; language: {configuration["scalar_instances"]:,} filters x {configuration["scalar_turns"]} turns | median of {configuration["samples"]} isolated process samples | million EKF turns per second{" | integrity checks enabled" if args.checked_only else ""}</text>',
    ]
    for tick in range(0, int(max_value) + 1, 10):
        tick_x = x(tick)
        lines.append(f'<line x1="{tick_x:.1f}" y1="{top - 18}" x2="{tick_x:.1f}" y2="{height - bottom + 4}" class="grid"/>')
        lines.append(f'<text x="{tick_x:.1f}" y="{height - bottom + 28}" text-anchor="middle" class="axis">{tick}</text>')
    lines.append(f'<text x="{left + chart_width / 2:.1f}" y="{height - 28}" text-anchor="middle" class="muted" font-size="14">million EKF turns/s</text>')

    ceiling_x = x(rust_reference)
    lines.append(f'<line x1="{ceiling_x:.1f}" y1="{top - 25}" x2="{ceiling_x:.1f}" y2="{height - bottom + 4}" stroke="#f3a847" stroke-width="2" stroke-dasharray="7 6"/>')
    reference_label = "Rust packed-SIMD checked reference" if args.checked_only else "Rust packed-SIMD unchecked reference"
    lines.append(f'<text x="{ceiling_x + 8:.1f}" y="{top - 29}" fill="#f3a847" font-size="13">{reference_label}: {rust_reference:.2f}</text>')

    for index, (label, family) in enumerate(order):
        value = values[label]
        y = top + index * row_height
        bar_width = max(1.0, chart_width * value / max_value)
        color = COLORS[family]
        lines.append(f'<text x="{left - 16}" y="{y + 19}" text-anchor="end" class="label">{esc(label)}</text>')
        lines.append(f'<rect x="{left}" y="{y + 5}" width="{bar_width:.1f}" height="19" rx="3" fill="{color}" opacity="0.9"/>')
        value_x = min(left + bar_width + 9, width - right + 10)
        lines.append(f'<text x="{value_x:.1f}" y="{y + 19}" class="value">{value:.2f}</text>')

    note = "Checked controls only; unchecked-only controls are omitted. " if args.checked_only else ""
    lines.append(f'<text x="52" y="{height - 55}" class="muted" font-size="12">{note}Checked GPU rows synchronize and validate each turn; the unchecked row batches all turns in one device submission. Parse, compilation, setup, and final readback are excluded.</text>')
    lines.append('</svg>')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
