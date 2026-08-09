#!/usr/bin/env python3
"""Render Gate B EKF ordered samples as stacked SVG small multiples."""

from __future__ import annotations

import argparse
import html
import json
import statistics
from pathlib import Path


LANES = (
    ("rust-raw", "Raw Rust", "#55c2a5"),
    ("mech-resident-complete", "Mech retained", "#ffb454"),
    ("julia-persistent", "Julia", "#a78bfa"),
    ("luajit-scalar", "LuaJIT", "#f472b6"),
    ("mech-current-atomic", "Mech atomic", "#60a5fa"),
    ("numpy-persistent", "NumPy", "#facc15"),
    ("lua-scalar", "Lua", "#fb7185"),
    ("python-scalar", "Python", "#94a3b8"),
)


def fmt_ns(value: float) -> str:
    if value < 1_000:
        return f"{value:.1f} ns"
    return f"{value / 1_000:.2f} us"


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    return ordered[round((len(ordered) - 1) * fraction)]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--html-output", type=Path)
    args = parser.parse_args()
    payload = json.loads(args.input.read_text(encoding="utf-8"))
    grouped = {
        lane: sorted(
            (row for row in payload["samples"] if row["lane"] == lane),
            key=lambda row: row["sample"],
        )
        for lane, _, _ in LANES
    }

    width = 1280
    left = 265
    right = 50
    top = 155
    lane_height = 108
    plot_height = 62
    height = top + lane_height * len(LANES) + 70
    plot_width = width - left - right
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#090d14"/>',
        '<style>text{font-family:Inter,ui-sans-serif,system-ui,sans-serif;letter-spacing:0}.title{fill:#f8fafc;font-size:28px;font-weight:700}.sub{fill:#94a3b8;font-size:14px}.name{fill:#e2e8f0;font-size:16px;font-weight:650}.stat{fill:#94a3b8;font-size:12px}.axis{fill:#64748b;font-size:11px}.grid{stroke:#263244;stroke-width:1}.base{stroke:#64748b;stroke-width:1;stroke-dasharray:4 5}.line{fill:none;stroke-width:2;stroke-linejoin:round;stroke-linecap:round}</style>',
        '<text class="title" x="54" y="58">Gate B EKF latency over cumulative turns</text>',
        '<text class="sub" x="54" y="88">60 ordered steady-state episodes per lane; 4,096 turns per episode; setup and reset excluded; GC enabled</text>',
        '<text class="sub" x="54" y="111">Each lane has an adaptive relative scale around its own median so pauses remain visible across a 350x throughput range.</text>',
    ]
    samples = payload["protocol"]["samples"]
    for tick in range(0, 5):
        x = left + plot_width * tick / 4
        turns = round(samples * payload["protocol"]["turns_per_sample"] * tick / 4)
        parts.append(f'<line class="grid" x1="{x:.1f}" y1="{top - 16}" x2="{x:.1f}" y2="{height - 58}" opacity="0.45"/>')
        parts.append(f'<text class="axis" x="{x:.1f}" y="{height - 34}" text-anchor="middle">{turns / 1_000:.0f}k</text>')

    for lane_index, (lane, label, color) in enumerate(LANES):
        rows = grouped[lane]
        values = [row["elapsed_ns"] / row["turns"] for row in rows]
        median = statistics.median(values)
        p99 = percentile(values, 0.99)
        maximum = max(values)
        minimum = min(values)
        spread = max(maximum / median - 1.0, 1.0 - minimum / median, 0.02)
        lower = 1.0 - spread * 1.15
        upper = 1.0 + spread * 1.15
        y0 = top + lane_index * lane_height
        baseline = y0 + plot_height * (upper - 1.0) / (upper - lower)
        parts.append(f'<text class="name" x="54" y="{y0 + 22}">{html.escape(label)}</text>')
        parts.append(f'<text class="stat" x="54" y="{y0 + 43}">median {fmt_ns(median)}/turn</text>')
        parts.append(f'<text class="stat" x="54" y="{y0 + 61}">p99 {fmt_ns(p99)} | max {maximum / median:.3f}x</text>')
        parts.append(f'<rect x="{left}" y="{y0}" width="{plot_width}" height="{plot_height}" rx="3" fill="#0f1622" stroke="#263244"/>')
        parts.append(f'<line class="base" x1="{left}" y1="{baseline:.2f}" x2="{left + plot_width}" y2="{baseline:.2f}"/>')
        points = []
        for index, (row, value) in enumerate(zip(rows, values)):
            x = left + plot_width * index / max(1, len(rows) - 1)
            relative = value / median
            y = y0 + plot_height * (upper - relative) / (upper - lower)
            points.append(f"{x:.2f},{y:.2f}")
            if (row.get("gc_ns") or 0) > 0:
                parts.append(f'<circle cx="{x:.2f}" cy="{y:.2f}" r="4" fill="#ef4444"/>')
            elif row.get("gc_cycle_inferred"):
                parts.append(f'<circle cx="{x:.2f}" cy="{y:.2f}" r="2.5" fill="#f59e0b"/>')
        parts.append(f'<polyline class="line" stroke="{color}" points="{" ".join(points)}"/>')
        parts.append(f'<text class="axis" x="{left + plot_width - 8}" y="{y0 + 16}" text-anchor="end">+{(upper - 1) * 100:.1f}%</text>')
        parts.append(f'<text class="axis" x="{left + plot_width - 8}" y="{y0 + plot_height - 7}" text-anchor="end">{(lower - 1) * 100:.1f}%</text>')

    parts.extend(
        (
            f'<text class="axis" x="{left + plot_width / 2}" y="{height - 10}" text-anchor="middle">cumulative EKF turns</text>',
            '<circle cx="55" cy="135" r="4" fill="#ef4444"/><text class="axis" x="66" y="139">reported GC interval (none)</text><circle cx="260" cy="135" r="3" fill="#f59e0b"/><text class="axis" x="270" y="139">Lua heap drop (inferred collection)</text>',
            '</svg>',
        )
    )
    svg = "\n".join(parts) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(svg, encoding="utf-8")
    if args.html_output:
        fragment = (
            '<div id="gate-b-ekf-timeline">\n'
            '<style>\n'
            '#gate-b-ekf-timeline{width:100%;overflow-x:auto;color-scheme:dark}\n'
            '#gate-b-ekf-timeline svg{display:block;width:100%;min-width:720px;height:auto}\n'
            '</style>\n'
            f'{svg}'
            '</div>\n'
        )
        args.html_output.parent.mkdir(parents=True, exist_ok=True)
        args.html_output.write_text(fragment, encoding="utf-8")
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
