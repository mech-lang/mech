#!/usr/bin/env python3
"""Render the matched Mech/Rust result without hiding the raw samples."""

from __future__ import annotations

import html
import json
import math
import statistics
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
EVIDENCE = (
    ROOT
    / "benchmarks/archive/compute/parallel-ekf/results"
    / "apple-m1-fused-reference-controls-2026-08-31.json"
)
OUTPUT = HERE / "charts/matched-mech-rust-variability.svg"
REPRESENTATIVE_OUTPUT = HERE / "charts/representative-checked-variability.svg"
RESULTS = ROOT / "benchmarks/archive/compute/parallel-ekf/results"


def esc(value: object) -> str:
    return html.escape(str(value), quote=True)


def load(name: str) -> dict:
    return json.loads((RESULTS / name).read_text(encoding="utf-8"))


def representative_rows() -> list[tuple[str, str, list[float]]]:
    direct = load("apple-m1-mech-metal-2026-09-04.json")
    advanced_mojo = load("apple-m1-mojo-advanced-2026-09-04.json")
    native = load("apple-m1-mech-taichi-native-metal-2026-08-31.json")
    fused = load("apple-m1-fused-reference-controls-2026-08-31.json")
    halide = load("apple-m1-halide-metal-strict-2026-08-31.json")
    futhark = load("apple-m1-futhark-ispc-fixed-2026-08-31.json")
    julia = load("apple-m1-julia-threaded-2026-08-31.json")
    numba = load("apple-m1-numpy-numba-2026-08-31.json")
    pypy = load("apple-m1-pypy-2026-09-05.json")

    native_rows = {row["label"]: row for row in native["rows"]}
    pypy_lanes = pypy["lanes"]
    return [
        (
            "Mech direct Metal · 500k×40",
            "#f4c430",
            direct["rows"]["Mech direct Metal generated from generic scalar IR"]
            ["checked"]["samples_million_ekf_turns_per_second"],
        ),
        (
            "Mojo native Metal · 500k×40",
            "#60a5fa",
            advanced_mojo["rows"]["Mojo native Metal resident kernel"]
            ["checked"]["samples_million_ekf_turns_per_second"],
        ),
        (
            "Taichi native Metal · 500k×40",
            "#e36b6b",
            native_rows["Taichi native Metal, checked"]["samples"],
        ),
        (
            "Rust packed SIMD · 500k×40",
            "#dea584",
            fused["rows"]["rust_fused_checked"]["throughput_millions"],
        ),
        (
            "Mech SIMD/JIT · 500k×40",
            "#f4c430",
            fused["rows"]["mech_fused_checked"]["throughput_millions"],
        ),
        (
            "Mojo SIMD-4 · 500k×40",
            "#60a5fa",
            advanced_mojo["rows"]["Mojo fused SIMD-4, 8 workers"]
            ["checked"]["samples_million_ekf_turns_per_second"],
        ),
        (
            "Halide native Metal · 500k×40",
            "#ff8f00",
            [
                value / 1_000_000
                for value in halide["rows"]["Halide GPU Metal checked"]["throughput"]
            ],
        ),
        (
            "Futhark ISPC, 8 workers · 500k×40",
            "#e94f37",
            futhark["rows"]["checked"]["throughput_millions"],
        ),
        (
            "Julia SIMD, 8 workers · 500k×40",
            "#9558b2",
            julia["rows"]["checked"]["throughput_millions"],
        ),
        (
            "NumPy/Numba, 8 workers · 500k×40",
            "#4d77cf",
            numba["rows"]["checked"]["throughput_millions"],
        ),
        (
            "PyPy optimized scalar · 10k×20",
            "#22c55e",
            [
                value / 1_000_000
                for value in pypy_lanes["pypy_optimized_checked"]
                ["throughput_ekf_turns_per_second"]
            ],
        ),
        (
            "CPython textbook scalar · 10k×20",
            "#4d77cf",
            [
                value / 1_000_000
                for value in pypy_lanes["cpython_textbook_checked"]
                ["throughput_ekf_turns_per_second"]
            ],
        ),
    ]


def render_representative() -> None:
    rows = representative_rows()
    rows.sort(key=lambda row: statistics.median(row[2]), reverse=True)
    width, row_height = 1560, 48
    left, right, top, bottom = 455, 90, 125, 115
    height = top + len(rows) * row_height + bottom
    chart_width = width - left - right
    minimum, maximum = 0.02, 500.0
    log_min = math.log10(minimum)
    log_span = math.log10(maximum) - log_min

    def x(value: float) -> float:
        return left + chart_width * (math.log10(value) - log_min) / log_span

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="rep-title rep-desc">',
        '<title id="rep-title">Representative checked EKF throughput with raw samples and ranges</title>',
        '<desc id="rep-desc">Representative checked implementation lanes on the same Apple M1. Every raw process sample is shown, along with the median and observed minimum-to-maximum range. Workload sizes are printed in the labels.</desc>',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#91a0b5}.grid{stroke:#263246;stroke-width:1}.minor-grid{stroke:#1b2536;stroke-width:1}.axis{fill:#91a0b5;font-size:13px}.label{font-size:15px}.value{font-size:13px;font-variant-numeric:tabular-nums}.whisker{stroke:#e8edf5;stroke-width:2.5}</style>',
        '<text x="42" y="38" font-size="25" font-weight="700">Representative checked EKF lanes: medians, observed ranges, and every sample</text>',
        '<text x="42" y="66" class="muted" font-size="15">Same Apple M1 · retained measurement campaigns · different implementations and workload boundaries · logarithmic scale</text>',
    ]

    ticks = [0.02, 0.05, 0.1, 0.2, 0.5, 1, 2, 5, 10, 20, 50, 100, 200, 500]
    for tick in ticks:
        tick_x = x(tick)
        major = tick in {0.1, 1, 10, 100}
        css = "grid" if major else "minor-grid"
        lines.append(
            f'<line x1="{tick_x:.1f}" y1="{top - 15}" x2="{tick_x:.1f}" y2="{height - bottom}" class="{css}"/>'
        )
        lines.append(
            f'<text x="{tick_x:.1f}" y="{height - bottom + 25}" text-anchor="middle" class="axis">{tick:g}</text>'
        )

    offsets = (-7, -3, 0, 3, 7, -5, 5)
    for index, (label, color, samples) in enumerate(rows):
        y = top + index * row_height + 17
        median = statistics.median(samples)
        low, high = min(samples), max(samples)
        lines.append(
            f'<text x="{left - 18}" y="{y + 5}" text-anchor="end" class="label">{esc(label)}</text>'
        )
        lines.append(
            f'<line x1="{x(low):.1f}" y1="{y}" x2="{x(high):.1f}" y2="{y}" class="whisker"/>'
        )
        for endpoint in (low, high):
            lines.append(
                f'<line x1="{x(endpoint):.1f}" y1="{y - 8}" x2="{x(endpoint):.1f}" y2="{y + 8}" class="whisker"/>'
            )
        for sample, offset in zip(sorted(samples), offsets):
            lines.append(
                f'<circle cx="{x(sample):.1f}" cy="{y + offset}" r="4.5" fill="{color}" stroke="#080c14" stroke-width="1.5"><title>{esc(label)} sample: {sample:.3f} million turns/s</title></circle>'
            )
        mx = x(median)
        lines.append(
            f'<path d="M {mx:.1f} {y - 8} L {mx + 8:.1f} {y} L {mx:.1f} {y + 8} L {mx - 8:.1f} {y} Z" fill="#e8edf5" stroke="#080c14" stroke-width="1.5"><title>{esc(label)} median: {median:.3f} million turns/s</title></path>'
        )
        value_x = min(x(high) + 12, width - right + 8)
        anchor = "start" if value_x < width - right else "end"
        lines.append(
            f'<text x="{value_x:.1f}" y="{y + 5}" text-anchor="{anchor}" class="value">{median:.3f} · n={len(samples)}</text>'
        )

    lines.extend(
        [
            f'<text x="{left + chart_width / 2:.1f}" y="{height - 48}" text-anchor="middle" class="muted" font-size="14">million EKF turns per second (log scale)</text>',
            f'<text x="42" y="{height - 18}" class="muted" font-size="13">Circles: retained process samples · diamond: median · whisker: observed min–max. Descriptive ranges, not confidence intervals; small cross-row gaps are not ranks.</text>',
            '</svg>',
        ]
    )
    REPRESENTATIVE_OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    REPRESENTATIVE_OUTPUT.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    evidence = json.loads(EVIDENCE.read_text(encoding="utf-8"))
    rows = [
        (
            "Mech SIMD/JIT",
            "#f4c430",
            evidence["rows"]["mech_fused_checked"]["throughput_millions"],
        ),
        (
            "Rust packed SIMD",
            "#dea584",
            evidence["rows"]["rust_fused_checked"]["throughput_millions"],
        ),
    ]

    width, height = 1280, 470
    left, right, top, bottom = 245, 70, 105, 135
    chart_width = width - left - right
    maximum = 160.0

    def x(value: float) -> float:
        return left + chart_width * value / maximum

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">',
        '<title id="title">Matched checked Mech and Rust SIMD throughput</title>',
        '<desc id="desc">All three independent process samples for Mech and Rust, with the median shown as a diamond and the observed minimum-to-maximum range shown as a whisker.</desc>',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#91a0b5}.grid{stroke:#263246;stroke-width:1}.axis{fill:#91a0b5;font-size:13px}.label{font-size:17px}.value{font-size:14px;font-variant-numeric:tabular-nums}.whisker{stroke:#e8edf5;stroke-width:3}</style>',
        '<text x="42" y="38" font-size="25" font-weight="700">Matched checked SIMD throughput: raw samples, median, and range</text>',
        '<text x="42" y="66" class="muted" font-size="15">Apple M1 · 500,000 filters × 40 turns · four-wide SIMD · eight workers · fused block · n=3 per implementation</text>',
    ]

    for tick in range(0, 161, 20):
        tick_x = x(float(tick))
        lines.append(
            f'<line x1="{tick_x:.1f}" y1="{top - 12}" x2="{tick_x:.1f}" y2="{height - bottom}" class="grid"/>'
        )
        lines.append(
            f'<text x="{tick_x:.1f}" y="{height - bottom + 24}" text-anchor="middle" class="axis">{tick}</text>'
        )

    for index, (label, color, samples) in enumerate(rows):
        y = 155 + index * 92
        median = statistics.median(samples)
        low, high = min(samples), max(samples)
        lines.append(
            f'<text x="{left - 24}" y="{y + 6}" text-anchor="end" class="label">{esc(label)}</text>'
        )
        lines.append(
            f'<line x1="{x(low):.1f}" y1="{y}" x2="{x(high):.1f}" y2="{y}" class="whisker"/>'
        )
        for endpoint in (low, high):
            lines.append(
                f'<line x1="{x(endpoint):.1f}" y1="{y - 11}" x2="{x(endpoint):.1f}" y2="{y + 11}" class="whisker"/>'
            )
        offsets = (-9, 0, 9)
        for sample, offset in zip(sorted(samples), offsets):
            lines.append(
                f'<circle cx="{x(sample):.1f}" cy="{y + offset}" r="6" fill="{color}" stroke="#080c14" stroke-width="2"><title>{esc(label)} sample: {sample:.3f} million turns/s</title></circle>'
            )
        mx = x(median)
        lines.append(
            f'<path d="M {mx:.1f} {y - 10} L {mx + 10:.1f} {y} L {mx:.1f} {y + 10} L {mx - 10:.1f} {y} Z" fill="#e8edf5" stroke="#080c14" stroke-width="2"><title>{esc(label)} median: {median:.3f} million turns/s</title></path>'
        )
        lines.append(
            f'<text x="{left}" y="{y + 34}" class="value">median {median:.3f} · observed range {low:.3f}–{high:.3f} M turns/s</text>'
        )

    lines.extend(
        [
            f'<text x="{left + chart_width / 2:.1f}" y="{height - 80}" text-anchor="middle" class="muted" font-size="14">million EKF turns per second (zero-based scale)</text>',
            '<text x="42" y="435" class="muted" font-size="13">Circles: retained process samples · diamond: median · whisker: observed min–max. The whisker is descriptive, not a confidence interval.</text>',
            '</svg>',
        ]
    )
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    render_representative()


if __name__ == "__main__":
    main()
