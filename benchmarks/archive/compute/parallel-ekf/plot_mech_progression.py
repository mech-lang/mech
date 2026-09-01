#!/usr/bin/env python3
"""Render the Mech-only checked/unchecked execution-lane progression."""

from __future__ import annotations

import argparse
import html
import json
import re
import statistics
from pathlib import Path

from chart_machine_specs import svg_machine_specs

def esc(value: object) -> str:
    return html.escape(str(value), quote=True)


def printed_mech(cross: dict, run_name: str) -> dict[str, float]:
    values: dict[str, list[float]] = {}
    for text in cross.get("runs", {}).get(run_name, {}).get("measured_stdout", []):
        for match in re.finditer(
            r"^(Mech .+? throughput|GPU .+? throughput): ([0-9.]+) million",
            text,
            flags=re.MULTILINE,
        ):
            label = match.group(1)[: -len(" throughput")]
            values.setdefault(label, []).append(float(match.group(2)))
    return {label: statistics.median(samples) for label, samples in values.items()}


def load_rows(
    cross: dict,
    runtime: dict,
    native: dict,
    historical: dict,
    mech_simd_jit: dict | None = None,
) -> list[dict[str, object]]:
    scalar_values = printed_mech(cross, "mech_scalar_settings")
    backend_values = printed_mech(cross, "mech_backend_settings")
    runtime_rows = {row["label"]: row for row in runtime["rows"]}
    native_rows = {row["label"]: row for row in native["rows"]}
    old = historical["summary"]["mech_backends_million_ekf_turns_per_second"]
    optimized_rows = (mech_simd_jit or {}).get("rows", {})

    def optimized(mode: str, fallback: float | None) -> float | None:
        row = optimized_rows.get(mode)
        if row is None or "throughput_millions" not in row:
            return fallback
        return statistics.median(row["throughput_millions"])

    def row(label: str, checked: float | None, unchecked: float | None, note: str = "") -> dict[str, object]:
        return {"label": label, "checked": checked, "unchecked": unchecked, "note": note}

    return [
        row("resident scalar CPU", scalar_values["Mech scalar"], None),
        row("resident SIMD CPU (4 lanes)", scalar_values["Mech SIMD"], None),
        row("Cranelift JIT", scalar_values["Mech Cranelift JIT"], scalar_values["Mech Cranelift JIT unchecked"]),
        row(
            "Cranelift SIMD-JIT",
            optimized("checked", scalar_values["Mech Cranelift SIMD-JIT"]),
            optimized("unchecked", scalar_values["Mech Cranelift SIMD-JIT unchecked"]),
        ),
        row(
            "Cranelift SIMD-JIT, 8 workers",
            runtime_rows["Mech SIMD/JIT CPU, checked (8 workers)"]["throughput_millions"],
            runtime_rows["Mech SIMD/JIT CPU, unchecked (8 workers)"]["throughput_millions"],
        ),
        row(
            "WGPU, synchronized per-turn",
            runtime_rows["Mech WGPU GPU, checked"]["throughput_millions"],
            runtime_rows["Mech WGPU GPU, unchecked"]["throughput_millions"],
        ),
        row(
            "WGPU, one checked submission/turn",
            backend_values["GPU unchecked one-submit"],
            None,
            "100k-filter control",
        ),
        row(
            "native Metal, direct MSL",
            native_rows["Mech native Metal, checked"]["throughput_millions"],
            native_rows["Mech native Metal, unchecked"]["throughput_millions"],
            "500k filters, 40 synchronized turns",
        ),
        row(
            "WGPU, fused device batch",
            None,
            old["Mech GPU, 120 turns/submission"],
            "historical 2026-08-14 fused control; no per-turn boundary",
        ),
    ]


def render(rows: list[dict[str, object]], output: Path) -> None:
    # Checked throughput defines the ranking. Controls without a checked
    # measurement stay at the bottom rather than being ranked by unchecked
    # throughput.
    rows = sorted(
        rows,
        key=lambda item: (
            item["checked"] is None,
            -(item["checked"] or 0.0),
            -(item["unchecked"] or 0.0),
            str(item["label"]),
        ),
    )
    width, left, right, top, row_height, bottom = 1800, 570, 150, 140, 44, 110
    chart_width = width - left - right
    maximum = max(max(item["checked"] or 0.0, item["unchecked"] or 0.0) for item in rows)
    max_axis = max(20.0, ((maximum + 19.999) // 20) * 20)
    height = top + row_height * len(rows) + bottom

    def x(value: float) -> float:
        return left + chart_width * value / max_axis

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#91a0b5}.grid{stroke:#263246;stroke-width:1}.label{font-size:14px}.value{font-size:13px;font-variant-numeric:tabular-nums}.note{fill:#91a0b5;font-size:11px}</style>',
        '<text x="52" y="42" font-size="26" font-weight="700">Mech EKF execution-lane progression</text>',
        '<text x="52" y="68" class="muted" font-size="15">Apple M1 | paired checked and unchecked steady-state throughput | million EKF turns per second | checked fastest to slowest</text>',
        '<rect x="1220" y="24" width="16" height="16" rx="2" fill="#f4c430"/><text x="1244" y="37" font-size="13">checked</text>',
        '<rect x="1325" y="24" width="16" height="16" rx="2" fill="#fff0a8"/><text x="1349" y="37" font-size="13">unchecked</text>',
    ]
    for tick in range(0, int(max_axis) + 1, 20):
        tx = x(tick)
        lines.append(f'<line x1="{tx:.1f}" y1="{top - 22}" x2="{tx:.1f}" y2="{height - bottom + 4}" class="grid"/>')
        lines.append(f'<text x="{tx:.1f}" y="{height - bottom + 28}" text-anchor="middle" class="muted" font-size="13">{tick}</text>')

    for index, item in enumerate(rows):
        y = top + index * row_height
        lines.append(f'<text x="{left - 16}" y="{y + 17}" text-anchor="end" class="label">{esc(item["label"])}</text>')
        for offset, mode, color in ((2, "checked", "#f4c430"), (22, "unchecked", "#fff0a8")):
            value = item[mode]
            if value is None:
                lines.append(f'<text x="{left + 5}" y="{y + offset + 14}" class="note">not measured</text>')
                continue
            bar_width = max(1.0, chart_width * value / max_axis)
            lines.append(f'<rect x="{left}" y="{y + offset}" width="{bar_width:.1f}" height="15" rx="2" fill="{color}" opacity="0.92"/>')
            lines.append(f'<text x="{min(left + bar_width + 8, width - right + 5):.1f}" y="{y + offset + 13}" class="value">{value:.2f}</text>')
        if item["note"]:
            lines.append(f'<text x="{left + 8}" y="{y + 39}" class="note">{esc(item["note"])}</text>')

    lines.append(f'<text x="{left + chart_width / 2:.1f}" y="{height - 28}" text-anchor="middle" class="muted" font-size="14">million EKF turns per second</text>')
    lines.append('<text x="52" y="%d" class="muted" font-size="12">Checked bars validate candidate publication; unchecked bars omit integrity checks. Missing bars were not measured.</text>' % (height - 96))
    lines.append(svg_machine_specs(width, height, right=right, bottom=18))
    lines.append('</svg>')
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("cross_language", type=Path)
    parser.add_argument("runtime", type=Path)
    parser.add_argument("native", type=Path)
    parser.add_argument("historical", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--mech-simd-jit", type=Path)
    args = parser.parse_args()
    rows = load_rows(
        json.loads(args.cross_language.read_text(encoding="utf-8")),
        json.loads(args.runtime.read_text(encoding="utf-8")),
        json.loads(args.native.read_text(encoding="utf-8")),
        json.loads(args.historical.read_text(encoding="utf-8")),
        json.loads(args.mech_simd_jit.read_text(encoding="utf-8"))
        if args.mech_simd_jit
        else None,
    )
    render(rows, args.output)


if __name__ == "__main__":
    main()
