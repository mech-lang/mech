#!/usr/bin/env python3
"""Render the two publication-facing IROS EKF charts from retained samples."""

from __future__ import annotations

import html
import json
import math
import statistics
from dataclasses import dataclass
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
RESULTS = ROOT / "benchmarks/archive/compute/parallel-ekf/results"
CHARTS = HERE / "charts"


@dataclass(frozen=True)
class Row:
    label: str
    detail: str
    color: str
    samples: list[float]


def esc(value: object) -> str:
    return html.escape(str(value), quote=True)


def load(name: str) -> dict:
    return json.loads((RESULTS / name).read_text(encoding="utf-8"))


def write_svg(name: str, lines: list[str]) -> None:
    CHARTS.mkdir(parents=True, exist_ok=True)
    (CHARTS / name).write_text("\n".join(lines) + "\n", encoding="utf-8")


def sample_offsets(count: int) -> list[int]:
    offsets = {
        1: [0],
        2: [-5, 5],
        3: [-8, 0, 8],
        4: [-9, -3, 3, 9],
        5: [-10, -5, 0, 5, 10],
        7: [-12, -8, -4, 0, 4, 8, 12],
    }
    return offsets[count]


def render_cross_language() -> None:
    fused = load("apple-m1-fused-reference-controls-2026-08-31.json")
    rows = [
        Row(
            "Rust packed SIMD",
            "four-wide SIMD",
            "#dea584",
            fused["rows"]["rust_fused_checked"]["throughput_millions"],
        ),
        Row(
            "Mech SIMD/JIT",
            "four-wide SIMD",
            "#f4c430",
            fused["rows"]["mech_fused_checked"]["throughput_millions"],
        ),
        Row(
            "Julia SIMD.jl",
            "SIMD, eight threads",
            "#9558b2",
            fused["rows"]["julia_fused_checked"]["throughput_millions"],
        ),
        Row(
            "NumPy/Numba",
            "compiled array kernel",
            "#4d77cf",
            fused["rows"]["numba_fused_checked"]["throughput_millions"],
        ),
    ]

    width, height = 1600, 620
    left, right, top = 350, 145, 165
    axis_y = 475
    chart_width = width - left - right
    maximum = 165.0

    def x(value: float) -> float:
        return left + chart_width * value / maximum

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="cross-title cross-desc">',
        '<title id="cross-title">Checked CPU EKF throughput under a matched execution shape</title>',
        '<desc id="cross-desc">Four checked CPU implementations on one Apple M1, each running 500,000 filters for 40 turns with eight workers and a fused worker-local block. Every retained process sample is shown with its median and observed minimum-to-maximum range.</desc>',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#9aa8ba}.grid{stroke:#2a374c;stroke-width:1}.row-guide{stroke:#182235;stroke-width:1}.axis{fill:#9aa8ba;font-size:14px}.label{font-size:18px;font-weight:600}.detail{fill:#9aa8ba;font-size:13px}.value{font-size:14px;font-variant-numeric:tabular-nums}.whisker{stroke:#e8edf5;stroke-width:2.5}.footnote{fill:#aab5c5;font-size:13px}</style>',
        '<text x="42" y="48" font-size="28" font-weight="700">Checked CPU EKF throughput under a matched execution shape</text>',
        '<text x="42" y="80" class="muted" font-size="16">Apple M1 · 500,000 filters × 40 turns · f32 · eight workers · fused worker-local block · checked · n=3</text>',
        '<text x="42" y="112" class="muted" font-size="13">Circles are retained process samples; diamonds are medians; whiskers are observed min–max ranges, not confidence intervals.</text>',
    ]

    for tick in range(0, 161, 20):
        tick_x = x(float(tick))
        lines.append(
            f'<line x1="{tick_x:.1f}" y1="{top - 25}" x2="{tick_x:.1f}" y2="{axis_y}" class="grid"/>'
        )
        lines.append(
            f'<text x="{tick_x:.1f}" y="{axis_y + 27}" text-anchor="middle" class="axis">{tick}</text>'
        )

    for index, row in enumerate(rows):
        y = top + index * 72
        median = statistics.median(row.samples)
        low, high = min(row.samples), max(row.samples)
        lines.append(
            f'<line x1="{left}" y1="{y + 34}" x2="{width - right}" y2="{y + 34}" class="row-guide"/>'
        )
        lines.append(
            f'<text x="{left - 24}" y="{y - 2}" text-anchor="end" class="label">{esc(row.label)}</text>'
        )
        lines.append(
            f'<text x="{left - 24}" y="{y + 19}" text-anchor="end" class="detail">{esc(row.detail)}</text>'
        )
        lines.append(
            f'<line x1="{x(low):.1f}" y1="{y}" x2="{x(high):.1f}" y2="{y}" class="whisker"/>'
        )
        for endpoint in (low, high):
            lines.append(
                f'<line x1="{x(endpoint):.1f}" y1="{y - 11}" x2="{x(endpoint):.1f}" y2="{y + 11}" class="whisker"/>'
            )
        for sample, offset in zip(sorted(row.samples), sample_offsets(len(row.samples))):
            lines.append(
                f'<circle cx="{x(sample):.1f}" cy="{y + offset}" r="6" fill="{row.color}" stroke="#080c14" stroke-width="2"><title>{esc(row.label)} sample: {sample:.3f} million turns/s</title></circle>'
            )
        median_x = x(median)
        lines.append(
            f'<path d="M {median_x:.1f} {y - 10} L {median_x + 10:.1f} {y} L {median_x:.1f} {y + 10} L {median_x - 10:.1f} {y} Z" fill="#ffffff" stroke="#080c14" stroke-width="2"><title>{esc(row.label)} median: {median:.3f} million turns/s</title></path>'
        )
        lines.append(
            f'<text x="{x(high) + 18:.1f}" y="{y + 5}" class="value">{median:.3f} · n={len(row.samples)}</text>'
        )

    lines.extend(
        [
            f'<line x1="{left}" y1="{axis_y}" x2="{width - right}" y2="{axis_y}" class="grid"/>',
            f'<text x="{left + chart_width / 2:.1f}" y="{axis_y + 58}" text-anchor="middle" class="muted" font-size="15">million EKF turns per second</text>',
            '<text x="42" y="570" class="footnote">Mech and Rust additionally match block-atomic rollback. Julia and Numba reject invalid candidates per lane; all measured samples reported zero faults.</text>',
            '<text x="42" y="595" class="footnote">Use this figure for the matched CPU execution shape; use the Mech backend figure for backend reach.</text>',
            '</svg>',
        ]
    )
    write_svg("post-cross-language-comparison.svg", lines)


def render_mech_backends() -> None:
    metal = load("apple-m1-mech-metal-2026-09-04.json")
    runtime = load("apple-m1-mech-taichi-runtime-2026-08-31.json")
    simd = load("apple-m1-mech-simd-jit-neon-strict-2026-09-01.json")
    scalar = load("apple-m1-mech-scalar-checked-matched-2026-09-09.json")
    aot = json.loads(
        (HERE / "results/apple-m1-mech-aot-2026-09-24.json").read_text(encoding="utf-8")
    )
    dylib = json.loads(
        (HERE / "results/apple-m1-aot-vs-rust-dylib-2026-09-24.json").read_text(
            encoding="utf-8"
        )
    )
    runtime_rows = {row["label"]: row for row in runtime["rows"]}
    rows = [
        Row(
            "Direct Metal GPU",
            "500k filters × 40 turns",
            "#f4c430",
            metal["rows"]["Mech direct Metal generated from generic scalar IR"]
            ["checked"]["samples_million_ekf_turns_per_second"],
        ),
        Row(
            "WGPU on Metal",
            "500k filters × 40 turns",
            "#f4c430",
            runtime_rows["Mech WGPU GPU, checked"]["samples"],
        ),
        Row(
            "SIMD/JIT CPU · 8 workers",
            "500k filters × 40 turns",
            "#f4c430",
            runtime_rows["Mech SIMD/JIT CPU, checked (8 workers)"]["samples"],
        ),
        Row(
            "SIMD/JIT CPU · 1 worker",
            "10k filters × 20 turns",
            "#f4c430",
            simd["rows"]["checked"]["throughput_millions"],
        ),
        Row(
            "Cranelift SIMD AOT CPU",
            "10k filters × 200 turns",
            "#f4c430",
            dylib["rows"]["Mech Cranelift SIMD AOT"]
            ["throughput_million_ekf_turns_per_second"]["samples"],
        ),
        Row(
            "Cranelift JIT CPU",
            "10k filters × 20 turns",
            "#f4c430",
            aot["rows"]["Mech Cranelift JIT CPU, same processes"]
            ["samples_million_ekf_turns_per_second"],
        ),
        Row(
            "Cranelift AOT CPU",
            "10k filters × 20 turns",
            "#f4c430",
            aot["rows"]["Mech Cranelift AOT CPU"]
            ["samples_million_ekf_turns_per_second"],
        ),
        Row(
            "Scalar artifact evaluator",
            "10k filters × 20 turns",
            "#f4c430",
            [
                sample["scalar_checked_million_ekf_turns_per_second"]
                for sample in scalar["samples"]
            ],
        ),
    ]

    width, height = 1600, 988
    left, right, top = 430, 165, 160
    axis_y = 798
    chart_width = width - left - right
    minimum, maximum = 0.5, 650.0
    log_min = math.log10(minimum)
    log_span = math.log10(maximum) - log_min

    def x(value: float) -> float:
        return left + chart_width * (math.log10(value) - log_min) / log_span

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="mech-title mech-desc">',
        '<title id="mech-title">One Mech EKF across eight execution backends</title>',
        '<desc id="mech-desc">Eight checked execution backends for the same high-level Mech EKF on one Apple M1. Every retained process sample is shown with its median and observed minimum-to-maximum range. A logarithmic scale keeps the scalar and GPU backends visible together.</desc>',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#9aa8ba}.grid{stroke:#2a374c;stroke-width:1}.minor-grid{stroke:#1a2538;stroke-width:1}.row-guide{stroke:#182235;stroke-width:1}.axis{fill:#9aa8ba;font-size:14px}.label{font-size:18px;font-weight:600}.detail{fill:#9aa8ba;font-size:13px}.value{font-size:14px;font-variant-numeric:tabular-nums}.whisker{stroke:#e8edf5;stroke-width:2.5}.footnote{fill:#aab5c5;font-size:13px}</style>',
        '<text x="42" y="48" font-size="28" font-weight="700">One Mech EKF across eight execution backends</text>',
        '<text x="42" y="80" class="muted" font-size="16">Checked publication after every turn · same Apple M1 · raw retained process samples · workload shown per row</text>',
        '<text x="42" y="112" class="muted" font-size="13">Circles are samples; diamonds are medians; whiskers are observed min–max ranges, not confidence intervals. Logarithmic throughput scale.</text>',
    ]

    ticks = [0.5, 1, 2, 5, 10, 20, 50, 100, 200, 500]
    for tick in ticks:
        tick_x = x(tick)
        major = tick in {1, 10, 100}
        css = "grid" if major else "minor-grid"
        lines.append(
            f'<line x1="{tick_x:.1f}" y1="{top - 25}" x2="{tick_x:.1f}" y2="{axis_y}" class="{css}"/>'
        )
        lines.append(
            f'<text x="{tick_x:.1f}" y="{axis_y + 27}" text-anchor="middle" class="axis">{tick:g}</text>'
        )

    for index, row in enumerate(rows):
        y = top + index * 78
        median = statistics.median(row.samples)
        low, high = min(row.samples), max(row.samples)
        lines.append(
            f'<line x1="{left}" y1="{y + 37}" x2="{width - right}" y2="{y + 37}" class="row-guide"/>'
        )
        lines.append(
            f'<text x="{left - 26}" y="{y - 3}" text-anchor="end" class="label">{esc(row.label)}</text>'
        )
        lines.append(
            f'<text x="{left - 26}" y="{y + 19}" text-anchor="end" class="detail">{esc(row.detail)}</text>'
        )
        lines.append(
            f'<line x1="{x(low):.1f}" y1="{y}" x2="{x(high):.1f}" y2="{y}" class="whisker"/>'
        )
        for endpoint in (low, high):
            lines.append(
                f'<line x1="{x(endpoint):.1f}" y1="{y - 11}" x2="{x(endpoint):.1f}" y2="{y + 11}" class="whisker"/>'
            )
        for sample, offset in zip(sorted(row.samples), sample_offsets(len(row.samples))):
            lines.append(
                f'<circle cx="{x(sample):.1f}" cy="{y + offset}" r="6" fill="{row.color}" stroke="#080c14" stroke-width="2"><title>{esc(row.label)} sample: {sample:.3f} million turns/s</title></circle>'
            )
        median_x = x(median)
        lines.append(
            f'<path d="M {median_x:.1f} {y - 10} L {median_x + 10:.1f} {y} L {median_x:.1f} {y + 10} L {median_x - 10:.1f} {y} Z" fill="#ffffff" stroke="#080c14" stroke-width="2"><title>{esc(row.label)} median: {median:.3f} million turns/s</title></path>'
        )
        lines.append(
            f'<text x="{x(high) + 18:.1f}" y="{y + 5}" class="value">{median:.3f} · n={len(row.samples)}</text>'
        )

    lines.extend(
        [
            f'<line x1="{left}" y1="{axis_y}" x2="{width - right}" y2="{axis_y}" class="grid"/>',
            f'<text x="{left + chart_width / 2:.1f}" y="{axis_y + 58}" text-anchor="middle" class="muted" font-size="15">million EKF turns per second (log scale)</text>',
            '<text x="42" y="909" class="footnote">Rows combine retained same-machine campaigns. Workload size changes where needed; all values are normalized throughput.</text>',
            '<text x="42" y="935" class="footnote">Read this figure as backend reach, not a fine ranking. Small cross-row gaps are not performance claims.</text>',
            '</svg>',
        ]
    )
    write_svg("post-mech-backend-stack.svg", lines)


def main() -> None:
    render_cross_language()
    render_mech_backends()


if __name__ == "__main__":
    main()
