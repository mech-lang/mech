#!/usr/bin/env python3
"""Render the matched Mech/Taichi CPU/GPU EKF comparison as an SVG."""

from __future__ import annotations

import argparse
import html
import json
from pathlib import Path

from chart_machine_specs import svg_machine_specs

COLORS = {
    "Mech": "#40d4b0",
    "Taichi": "#b68cff",
}


def esc(value: object) -> str:
    return html.escape(str(value), quote=True)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("evidence", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()

    evidence = json.loads(args.evidence.read_text(encoding="utf-8"))
    configuration = evidence["configuration"]
    rows = list(evidence["rows"])
    # Keep checked controls together and rank them fastest first. Unchecked
    # controls follow in their own throughput ranking so the guarantee level
    # is never obscured by a faster unchecked row.
    def rank(row: dict[str, object]) -> tuple[int, float, str]:
        label = str(row["label"]).lower()
        is_unchecked = "unchecked" in label
        return (1 if is_unchecked else 0, -float(row["throughput_millions"]), str(row["label"]))

    rows.sort(key=rank)
    width = 1500
    left = 430
    right = 150
    top = 140
    row_height = 42
    bottom = 100
    chart_width = width - left - right
    max_value = max(240.0, ((max(row["throughput_millions"] for row in rows) + 19.999) // 20) * 20)
    height = top + row_height * len(rows) + bottom
    chart_title = (
        "Backend-matched EKF runtime throughput"
        if "mech_native_metal_backend" in configuration
        else "Matched hardware EKF runtime throughput"
    )

    def x(value: float) -> float:
        return left + chart_width * value / max_value

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5} .muted{fill:#91a0b5} .grid{stroke:#263246;stroke-width:1} .axis{fill:#91a0b5;font-size:13px} .label{font-size:15px} .value{font-size:14px;font-variant-numeric:tabular-nums}</style>',
        f'<text x="52" y="43" font-size="27" font-weight="700">{chart_title}</text>',
        f'<text x="52" y="71" class="muted" font-size="15">Apple M1 | {configuration["instances"]:,} resident filters x {configuration["turns"]} turns | median of {configuration["samples"]} isolated samples | checked fastest to slowest, then unchecked</text>',
    ]
    for tick in range(0, int(max_value) + 1, 20):
        tick_x = x(tick)
        lines.append(f'<line x1="{tick_x:.1f}" y1="{top - 20}" x2="{tick_x:.1f}" y2="{height - bottom + 4}" class="grid"/>')
        lines.append(f'<text x="{tick_x:.1f}" y="{height - bottom + 28}" text-anchor="middle" class="axis">{tick}</text>')
    lines.append(f'<text x="{left + chart_width / 2:.1f}" y="{height - 28}" text-anchor="middle" class="muted" font-size="14">million EKF turns per second</text>')

    legend_x = width - right - 220
    for index, runtime in enumerate(("Mech", "Taichi")):
        x_pos = legend_x + index * 115
        lines.append(f'<rect x="{x_pos}" y="95" width="14" height="14" rx="2" fill="{COLORS[runtime]}"/>')
        lines.append(f'<text x="{x_pos + 22}" y="107" font-size="14">{runtime}</text>')

    for index, row in enumerate(rows):
        value = float(row["throughput_millions"])
        y = top + index * row_height
        bar_width = max(1.0, chart_width * value / max_value)
        color = COLORS[row["runtime"]]
        lines.append(f'<text x="{left - 16}" y="{y + 24}" text-anchor="end" class="label">{esc(row["label"])}</text>')
        lines.append(f'<rect x="{left}" y="{y + 7}" width="{bar_width:.1f}" height="24" rx="3" fill="{color}" opacity="0.9"/>')
        lines.append(f'<text x="{min(left + bar_width + 10, width - right + 12):.1f}" y="{y + 24}" class="value">{value:.2f}</text>')

    note = "Checked rows validate and publish only valid candidates; unchecked rows omit those checks. "
    note += "Every GPU row submits and waits once per turn; setup, compilation, allocation, warmup, and final readback are excluded."
    if "mech_native_metal_backend" in configuration:
        note += " Native Metal rows use direct Metal command submission; WGPU-over-Metal rows are shown only as a transport control."
    lines.append(f'<text x="52" y="{height - 96}" class="muted" font-size="12">{esc(note)}</text>')
    lines.append(svg_machine_specs(width, height, right=right, bottom=18))
    lines.append('</svg>')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
