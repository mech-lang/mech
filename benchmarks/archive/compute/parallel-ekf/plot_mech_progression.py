#!/usr/bin/env python3
"""Render the Mech-only checked/unchecked execution-lane progression."""

from __future__ import annotations

import argparse
import html
import json
import math
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
            r"^(Mech .+? throughput|GPU .+? throughput)(?: \([^)]*\))?: ([0-9.]+) million",
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
    mech_trials: dict | None = None,
    strict_mech: dict | None = None,
) -> list[dict[str, object]]:
    scalar_values = printed_mech(cross, "mech_scalar_settings")
    backend_values = printed_mech(cross, "mech_backend_settings")
    runtime_rows = {row["label"]: row for row in runtime["rows"]}
    native_rows = {row["label"]: row for row in native["rows"]}
    old = historical["summary"]["mech_backends_million_ekf_turns_per_second"]
    optimized_rows = (mech_simd_jit or {}).get("rows", {})
    trial_rows = (mech_trials or {}).get("rows", {})
    strict_rows = (strict_mech or {}).get("rows", {})

    def optimized(mode: str, fallback: float | None) -> float | None:
        row = optimized_rows.get(mode)
        if row is None or "throughput_millions" not in row:
            return fallback
        return statistics.median(row["throughput_millions"])

    def row(label: str, checked: float | None, unchecked: float | None, note: str = "") -> dict[str, object]:
        return {"label": label, "checked": checked, "unchecked": unchecked, "note": note}

    def trial(label: str, fallback: float | None = None) -> float | None:
        evidence = trial_rows.get(label)
        if isinstance(evidence, dict):
            value = evidence.get("median_throughput_millions")
            if value is not None:
                return float(value)
        return fallback

    return [
        row(
            "native Metal, strict retained-state",
            strict_rows.get("Mech native Metal checked", {}).get("median_million_turns_per_second"),
            strict_rows.get("Mech native Metal unchecked", {}).get("median_million_turns_per_second"),
            "500k filters x 40 synchronized turns",
        ),
        row(
            "native Metal, direct MSL",
            native_rows["Mech native Metal, checked"]["throughput_millions"],
            native_rows["Mech native Metal, unchecked"]["throughput_millions"],
            "500k filters x 40 synchronized turns",
        ),
        row(
            "WGPU, synchronized per-turn",
            runtime_rows["Mech WGPU GPU, checked"]["throughput_millions"],
            runtime_rows["Mech WGPU GPU, unchecked"]["throughput_millions"],
            "500k filters x 40; portable transport",
        ),
        row(
            "Cranelift SIMD-JIT, 8 workers",
            runtime_rows["Mech SIMD/JIT CPU, checked (8 workers)"]["throughput_millions"],
            runtime_rows["Mech SIMD/JIT CPU, unchecked (8 workers)"]["throughput_millions"],
            "500k filters x 40 synchronized turns",
        ),
        row(
            "Cranelift SIMD-JIT, parallel",
            trial("Mech Cranelift SIMD-JIT parallel", backend_values.get("Mech Cranelift SIMD-JIT parallel")),
            trial("Mech Cranelift SIMD-JIT parallel unchecked"),
            "100k filters x 5; per-turn publication",
        ),
        row(
            "Cranelift SIMD-JIT, strict resident",
            optimized("checked", scalar_values.get("Mech Cranelift SIMD-JIT")),
            optimized("unchecked", scalar_values.get("Mech Cranelift SIMD-JIT unchecked")),
            "10k filters x 20; retained strict source",
        ),
        row(
            "Cranelift SIMD-JIT, resident baseline",
            trial("Mech Cranelift SIMD-JIT", scalar_values.get("Mech Cranelift SIMD-JIT")),
            trial("Mech Cranelift SIMD-JIT unchecked", scalar_values.get("Mech Cranelift SIMD-JIT unchecked")),
            "100k filters x 5; generic resident lane",
        ),
        row(
            "Cranelift JIT, resident",
            trial("Mech Cranelift JIT", scalar_values.get("Mech Cranelift JIT")),
            trial("Mech Cranelift JIT unchecked", scalar_values.get("Mech Cranelift JIT unchecked")),
            "100k filters x 5; generic resident lane",
        ),
        row(
            "resident SIMD CPU (4 lanes)",
            trial("Mech SIMD", scalar_values.get("Mech SIMD")),
            trial("Mech SIMD unchecked"),
            "100k filters x 5",
        ),
        row(
            "resident scalar CPU",
            trial("Mech scalar", scalar_values.get("Mech scalar")),
            trial("Mech scalar unchecked", scalar_values.get("Mech scalar unchecked")),
            "100k filters x 5",
        ),
        row(
            "Cranelift SIMD-JIT, fused block",
            trial("Mech Cranelift SIMD-JIT parallel checked fused block"),
            trial("Mech Cranelift SIMD-JIT parallel unchecked fused block"),
            "100k filters x 5; one publication per block",
        ),
        row(
            "GPU API, one-turn",
            trial("GPU checked one-turn", backend_values.get("GPU checked one-turn")),
            trial("GPU unchecked one-turn", backend_values.get("GPU unchecked one-turn")),
            "100k filters x 5; synchronous ping-pong boundary",
        ),
        row(
            "GPU API, repeated dispatches",
            trial("GPU checked repeated", backend_values.get("GPU checked repeated")),
            trial("GPU unchecked repeated", backend_values.get("GPU unchecked repeated")),
            "100k filters x 5; per-turn dispatch loop",
        ),
        row(
            "GPU API, unchecked in-place one-turn",
            None,
            trial("GPU unchecked in-place one-turn", backend_values.get("GPU unchecked in-place one-turn")),
            "100k filters x 5; in-place state",
        ),
        row(
            "GPU API, unchecked in-place repeated",
            None,
            trial("GPU unchecked in-place repeated", backend_values.get("GPU unchecked in-place repeated")),
            "100k filters x 5; in-place dispatch loop",
        ),
        row(
            "GPU API, unchecked one-submit",
            None,
            trial("GPU unchecked one-submit", backend_values.get("GPU unchecked one-submit")),
            "100k filters x 5; device-resident ceiling",
        ),
        row(
            "WGPU, fused device batch (historical)",
            None,
            old["Mech GPU, 120 turns/submission"],
            "historical 2026-08-14; no per-turn boundary",
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
    width, left, right, top, row_height, bottom = 1900, 620, 250, 158, 62, 230
    chart_width = width - left - right
    positive = [
        value
        for item in rows
        for value in (item["checked"], item["unchecked"])
        if value is not None and value > 0.0
    ]
    if not positive:
        raise ValueError("progression chart requires at least one positive throughput")
    ticks = [multiplier * (10.0**exponent) for exponent in range(-4, 7) for multiplier in (1, 2, 5)]
    min_axis = max(tick for tick in ticks if tick <= min(positive))
    max_axis = min(tick for tick in ticks if tick >= max(positive))
    if min_axis == max_axis:
        max_axis *= 2.0
    height = top + row_height * len(rows) + bottom

    def x(value: float) -> float:
        return left + chart_width * (math.log10(value) - math.log10(min_axis)) / (math.log10(max_axis) - math.log10(min_axis))

    def format_tick(value: float) -> str:
        return f"{value:g}"

    def value_label(value: float, end: float) -> str:
        if end + 54 <= width - right:
            return f'<text x="{end + 8:.1f}" y="{{y}}" class="value">{value:.2f}</text>'
        return f'<text x="{width - right - 4:.1f}" y="{{y}}" text-anchor="end" class="value">{value:.2f}</text>'

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#91a0b5}.grid{stroke:#263246;stroke-width:1}.minor-grid{stroke:#1b2536;stroke-width:1}.label{font-size:14px}.value{font-size:13px;font-variant-numeric:tabular-nums}.note{fill:#91a0b5;font-size:11px}</style>',
        '<text x="52" y="42" font-size="26" font-weight="700">Mech EKF execution-lane progression</text>',
        '<text x="52" y="68" class="muted" font-size="15">Apple M1 | Mech execution trials | logarithmic million EKF turns per second axis | checked fastest to slowest; unchecked-only trials follow</text>',
        '<rect x="52" y="96" width="16" height="16" rx="2" fill="#f4c430"/><text x="76" y="109" font-size="13">checked</text>',
        '<rect x="162" y="96" width="16" height="16" rx="2" fill="#a88721"/><text x="186" y="109" font-size="13">unchecked</text>',
    ]
    visible_ticks = [tick for tick in ticks if min_axis <= tick <= max_axis]
    for tick in visible_ticks:
        tx = x(tick)
        major = tick in {1.0, 10.0, 100.0, 1000.0, 10000.0}
        grid_class = "grid" if major else "minor-grid"
        lines.append(f'<line x1="{tx:.1f}" y1="{top - 22}" x2="{tx:.1f}" y2="{height - bottom + 4}" class="{grid_class}"/>')
        lines.append(f'<text x="{tx:.1f}" y="{height - bottom + 28}" text-anchor="middle" class="muted" font-size="13">{format_tick(tick)}</text>')

    for index, item in enumerate(rows):
        y = top + index * row_height
        lines.append(f'<text x="{left - 16}" y="{y + 18}" text-anchor="end" class="label">{esc(item["label"])}</text>')
        if item["note"]:
            lines.append(f'<text x="{left - 16}" y="{y + 39}" text-anchor="end" class="note">{esc(item["note"])}</text>')
        for offset, mode, color in ((3, "checked", "#f4c430"), (29, "unchecked", "#a88721")):
            value = item[mode]
            if value is None:
                lines.append(f'<text x="{left + 5}" y="{y + offset + 13}" class="note">not measured</text>')
                continue
            end = x(value)
            bar_width = max(2.0, end - left)
            lines.append(f'<rect x="{left}" y="{y + offset}" width="{bar_width:.1f}" height="17" rx="2" fill="{color}" opacity="0.95"/>')
            lines.append(value_label(value, end).format(y=y + offset + 14))

    lines.append(f'<text x="{left + chart_width / 2:.1f}" y="{height - bottom + 58}" text-anchor="middle" class="muted" font-size="14">million EKF turns per second (log scale)</text>')
    footer_y = height - bottom + 92
    lines.append(f'<text x="52" y="{footer_y}" class="muted" font-size="12">Checked bars validate candidate publication; unchecked bars omit integrity checks. Missing bars are explicit evidence gaps, not zeroes.</text>')
    lines.append(f'<text x="52" y="{footer_y + 19}" class="muted" font-size="12">Fast-arithmetic-only controls are excluded; the historical fused batch is retained as a device-resident ceiling without a per-turn publication boundary.</text>')
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
    parser.add_argument("--mech-trials", type=Path)
    parser.add_argument("--strict-mech", type=Path)
    args = parser.parse_args()
    rows = load_rows(
        json.loads(args.cross_language.read_text(encoding="utf-8")),
        json.loads(args.runtime.read_text(encoding="utf-8")),
        json.loads(args.native.read_text(encoding="utf-8")),
        json.loads(args.historical.read_text(encoding="utf-8")),
        json.loads(args.mech_simd_jit.read_text(encoding="utf-8"))
        if args.mech_simd_jit
        else None,
        json.loads(args.mech_trials.read_text(encoding="utf-8"))
        if args.mech_trials
        else None,
        json.loads(args.strict_mech.read_text(encoding="utf-8"))
        if args.strict_mech
        else None,
    )
    render(rows, args.output)


if __name__ == "__main__":
    main()
