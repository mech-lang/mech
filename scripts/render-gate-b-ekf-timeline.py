#!/usr/bin/env python3
"""Render Gate B EKF ordered samples as stacked SVG small multiples."""

from __future__ import annotations

import argparse
import html
import json
import statistics
from pathlib import Path


LANES = (
    ("rust-fixed-fused", "Rust fixed fused", "#34d399"),
    ("mech-resident-fused", "Mech fused", "#22d3ee"),
    ("rust-raw", "Rust generic", "#55c2a5"),
    ("julia-staticarrays", "Julia fixed", "#c084fc"),
    ("mech-resident-complete", "Mech retained", "#ffb454"),
    ("julia-persistent", "Julia dynamic", "#a78bfa"),
    ("luajit-fixed-preallocated", "LuaJIT fixed", "#ec4899"),
    ("luajit-scalar", "LuaJIT generic", "#f472b6"),
    ("mech-current-atomic", "Mech atomic", "#60a5fa"),
    ("lua-fixed-preallocated", "Lua fixed", "#f97316"),
    ("numpy-persistent", "NumPy", "#facc15"),
    ("lua-scalar", "Lua generic", "#fb7185"),
    ("python-fixed-preallocated", "Python fixed", "#cbd5e1"),
    ("python-scalar", "Python generic", "#94a3b8"),
)


def fmt_ns(value: float) -> str:
    if value < 1_000:
        return f"{value:.1f} ns"
    return f"{value / 1_000:.2f} us"


def fmt_hz(value: float) -> str:
    if value >= 1_000_000:
        return f"{value / 1_000_000:.2f} MHz"
    if value >= 1_000:
        return f"{value / 1_000:.1f} kHz"
    return f"{value:.0f} Hz"


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    return ordered[round((len(ordered) - 1) * fraction)]


def shared_scale_svg(
    payload: dict,
    grouped: dict[str, list[dict]],
    lanes: tuple[tuple[str, str, str], ...],
) -> str:
    width = 1280
    height = 670
    left = 150
    right = 50
    top = 240
    plot_height = 330
    plot_width = width - left - right
    rates_by_lane = {
        lane: [1.0e9 * row["turns"] / row["elapsed_ns"] for row in grouped[lane]]
        for lane, _, _ in lanes
    }
    maximum = max(max(rates) for rates in rates_by_lane.values()) * 1.05
    samples = payload["protocol"]["samples"]
    turns_per_sample = payload["protocol"]["turns_per_sample"]
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#090d14"/>',
        '<style>text{font-family:Inter,ui-sans-serif,system-ui,sans-serif;letter-spacing:0}.title{fill:#f8fafc;font-size:28px;font-weight:700}.sub{fill:#94a3b8;font-size:14px}.legend{fill:#cbd5e1;font-size:12px}.axis{fill:#94a3b8;font-size:12px}.grid{stroke:#263244;stroke-width:1}.line{fill:none;stroke-width:2.25;stroke-linejoin:round;stroke-linecap:round}</style>',
        '<text class="title" x="54" y="48">Gate B EKF throughput on one shared linear scale</text>',
        '<text class="sub" x="54" y="76">Every line uses the same EKF-turns-per-second axis across 245,760 cumulative EKF turns.</text>',
        f'<rect x="{left}" y="{top}" width="{plot_width}" height="{plot_height}" rx="3" fill="#0f1622" stroke="#263244"/>',
    ]
    for index, (lane, label, color) in enumerate(lanes):
        column = index % 4
        row = index // 4
        x = 54 + column * 300
        y = 116 + row * 27
        median = statistics.median(rates_by_lane[lane])
        parts.append(f'<line x1="{x}" y1="{y - 4}" x2="{x + 22}" y2="{y - 4}" stroke="{color}" stroke-width="3"/>')
        parts.append(f'<text class="legend" x="{x + 30}" y="{y}">{html.escape(label)} {fmt_hz(median)}</text>')
    for tick in range(0, 5):
        value = maximum * tick / 4
        y = top + plot_height * (1.0 - tick / 4)
        parts.append(f'<line class="grid" x1="{left}" y1="{y:.1f}" x2="{left + plot_width}" y2="{y:.1f}"/>')
        parts.append(f'<text class="axis" x="{left - 12}" y="{y + 4:.1f}" text-anchor="end">{fmt_hz(value)}</text>')
        x = left + plot_width * tick / 4
        turns = round(samples * turns_per_sample * tick / 4)
        parts.append(f'<line class="grid" x1="{x:.1f}" y1="{top}" x2="{x:.1f}" y2="{top + plot_height}" opacity="0.45"/>')
        parts.append(f'<text class="axis" x="{x:.1f}" y="{top + plot_height + 24}" text-anchor="middle">{turns / 1_000:.0f}k</text>')
    for lane, _, color in lanes:
        points = []
        rates = rates_by_lane[lane]
        for index, rate in enumerate(rates):
            x = left + plot_width * index / max(1, len(rates) - 1)
            y = top + plot_height * (1.0 - rate / maximum)
            points.append(f"{x:.2f},{y:.2f}")
        parts.append(f'<polyline class="line" stroke="{color}" points="{" ".join(points)}"/>')
    parts.extend(
        (
            f'<text class="axis" x="{left + plot_width / 2}" y="{height - 26}" text-anchor="middle">cumulative EKF turns</text>',
            f'<text class="axis" x="28" y="{top + plot_height / 2}" text-anchor="middle" transform="rotate(-90 28 {top + plot_height / 2})">EKF turns per second (Hz)</text>',
            '</svg>',
        )
    )
    return "\n".join(parts) + "\n"


def write_html_fragment(path: Path, root_id: str, svg: str) -> None:
    fragment = (
        f'<div id="{root_id}">\n'
        '<style>\n'
        f'#{root_id}{{width:100%;overflow-x:auto;color-scheme:dark}}\n'
        f'#{root_id} svg{{display:block;width:100%;min-width:720px;height:auto}}\n'
        '</style>\n'
        f'{svg}'
        '</div>\n'
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(fragment, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--html-output", type=Path)
    parser.add_argument("--shared-output", type=Path)
    parser.add_argument("--shared-html-output", type=Path)
    args = parser.parse_args()
    payload = json.loads(args.input.read_text(encoding="utf-8"))
    present = {row["lane"] for row in payload["samples"]}
    lanes = tuple(lane for lane in LANES if lane[0] in present)
    if not lanes:
        parser.error("input contains none of the supported lanes")
    grouped = {
        lane: sorted(
            (row for row in payload["samples"] if row["lane"] == lane),
            key=lambda row: row["sample"],
        )
        for lane, _, _ in lanes
    }

    width = 1280
    left = 265
    right = 50
    overview_top = 250
    overview_height = 230
    top = 575
    lane_height = 108
    plot_height = 62
    height = top + lane_height * len(lanes) + 70
    plot_width = width - left - right
    values_by_lane = {
        lane: [row["elapsed_ns"] / row["turns"] for row in grouped[lane]]
        for lane, _, _ in lanes
    }
    overview_max = max(max(values) for values in values_by_lane.values()) * 1.05
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#090d14"/>',
        '<style>text{font-family:Inter,ui-sans-serif,system-ui,sans-serif;letter-spacing:0}.title{fill:#f8fafc;font-size:28px;font-weight:700}.sub{fill:#94a3b8;font-size:14px}.name{fill:#e2e8f0;font-size:16px;font-weight:650}.stat{fill:#94a3b8;font-size:12px}.axis{fill:#64748b;font-size:11px}.grid{stroke:#263244;stroke-width:1}.base{stroke:#64748b;stroke-width:1;stroke-dasharray:4 5}.line{fill:none;stroke-width:2;stroke-linejoin:round;stroke-linecap:round}</style>',
        '<text class="title" x="54" y="48">Gate B EKF latency: absolute performance and pause shape</text>',
        '<text class="sub" x="54" y="76">60 ordered steady-state episodes per lane; 4,096 turns per episode; setup and reset excluded; GC enabled</text>',
        '<text class="sub" x="54" y="99">Top: every runtime on one linear scale. Bottom: adaptive per-lane scales reveal pauses that the absolute view compresses.</text>',
        f'<rect x="{left}" y="{overview_top}" width="{plot_width}" height="{overview_height}" rx="3" fill="#0f1622" stroke="#263244"/>',
        f'<text class="name" x="54" y="{overview_top + 24}">Shared linear scale</text>',
        f'<text class="stat" x="54" y="{overview_top + 46}">absolute time per turn</text>',
    ]
    samples = payload["protocol"]["samples"]
    for index, (lane, label, color) in enumerate(lanes):
        column = index % 4
        row = index // 4
        x = 54 + column * 300
        y = 132 + row * 25
        median = statistics.median(values_by_lane[lane])
        parts.append(f'<line x1="{x}" y1="{y - 4}" x2="{x + 22}" y2="{y - 4}" stroke="{color}" stroke-width="3"/>')
        parts.append(f'<text class="stat" x="{x + 30}" y="{y}">{html.escape(label)} {fmt_ns(median)}</text>')

    for tick in range(0, 5):
        value = overview_max * tick / 4
        y = overview_top + overview_height * (1.0 - tick / 4)
        parts.append(f'<line class="grid" x1="{left}" y1="{y:.1f}" x2="{left + plot_width}" y2="{y:.1f}"/>')
        parts.append(f'<text class="axis" x="{left - 12}" y="{y + 4:.1f}" text-anchor="end">{value / 1_000:.0f} us</text>')

    for tick in range(0, 5):
        x = left + plot_width * tick / 4
        turns = round(samples * payload["protocol"]["turns_per_sample"] * tick / 4)
        parts.append(f'<line class="grid" x1="{x:.1f}" y1="{overview_top}" x2="{x:.1f}" y2="{overview_top + overview_height}" opacity="0.45"/>')
        parts.append(f'<text class="axis" x="{x:.1f}" y="{overview_top + overview_height + 20}" text-anchor="middle">{turns / 1_000:.0f}k</text>')
        parts.append(f'<line class="grid" x1="{x:.1f}" y1="{top - 16}" x2="{x:.1f}" y2="{height - 58}" opacity="0.45"/>')
        parts.append(f'<text class="axis" x="{x:.1f}" y="{height - 34}" text-anchor="middle">{turns / 1_000:.0f}k</text>')

    for lane, _, color in lanes:
        points = []
        for index, value in enumerate(values_by_lane[lane]):
            x = left + plot_width * index / max(1, len(values_by_lane[lane]) - 1)
            y = overview_top + overview_height * (1.0 - value / overview_max)
            points.append(f"{x:.2f},{y:.2f}")
        parts.append(f'<polyline class="line" stroke="{color}" points="{" ".join(points)}"/>')

    parts.append(f'<text class="axis" x="{left + plot_width / 2}" y="{overview_top + overview_height + 42}" text-anchor="middle">cumulative EKF turns</text>')
    parts.append(f'<text class="axis" x="64" y="{top - 38}">Adaptive relative scale around each lane median</text>')
    parts.append(f'<circle cx="355" cy="{top - 42}" r="4" fill="#ef4444"/><text class="axis" x="366" y="{top - 38}">reported GC interval (none)</text><circle cx="560" cy="{top - 42}" r="3" fill="#f59e0b"/><text class="axis" x="570" y="{top - 38}">Lua heap drop (inferred collection)</text>')

    for lane_index, (lane, label, color) in enumerate(lanes):
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
            '</svg>',
        )
    )
    svg = "\n".join(parts) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(svg, encoding="utf-8")
    if args.html_output:
        write_html_fragment(args.html_output, "gate-b-ekf-timeline", svg)
    if args.shared_output:
        shared_svg = shared_scale_svg(payload, grouped, lanes)
        args.shared_output.parent.mkdir(parents=True, exist_ok=True)
        args.shared_output.write_text(shared_svg, encoding="utf-8")
        if args.shared_html_output:
            write_html_fragment(
                args.shared_html_output,
                "gate-b-ekf-shared-scale",
                shared_svg,
            )
    elif args.shared_html_output:
        parser.error("--shared-html-output requires --shared-output")
    print(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
