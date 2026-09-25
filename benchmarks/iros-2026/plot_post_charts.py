#!/usr/bin/env python3
"""Render the publication-facing IROS EKF charts from retained samples."""

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
LANGUAGE_COLORS = {
    "Mech": "#f4c430",
    "Rust": "#dea584",
    "Mojo": "#ff7a1a",
    "Julia": "#9558b2",
    "Futhark": "#5f021f",
    "NumPy/Numba": "#4d77cf",
    "Taichi": "#e36b6b",
    "Halide": "#BD5E9E",
}


@dataclass(frozen=True)
class Row:
    label: str
    detail: str
    color: str
    samples: list[float]


@dataclass(frozen=True)
class ModePairRow:
    label: str
    detail: str
    checked: list[float]
    unchecked: list[float]


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


def series_name(label: str) -> str:
    return label.split(" · ", 1)[0]


def series_color(label: str) -> str:
    return LANGUAGE_COLORS[series_name(label)]


def evidence_name(label: str) -> str:
    if label.startswith("Rust · hand-written MSL"):
        return "Rust + MSL"
    return series_name(label)


def slug(value: str) -> str:
    return "".join(character.lower() if character.isalnum() else "-" for character in value)


def spread_label(samples: list[float]) -> str:
    median = statistics.median(samples)
    return f"{median:.1f} +{max(samples) - median:.1f}/−{median - min(samples):.1f}"


def portable_rows() -> tuple[list[ModePairRow], list[ModePairRow]]:
    """Return the systems that select both CPU and Metal from one source file."""
    runtime = load("apple-m1-mech-taichi-runtime-2026-08-31.json")
    runtime_rows = {row["label"]: row for row in runtime["rows"]}
    metal = load("apple-m1-mech-metal-2026-09-04.json")
    mech_metal = metal["rows"]["Mech direct Metal generated from generic scalar IR"]
    taichi_cpu = json.loads(
        (HERE / "results/apple-m1-taichi-cpu-matched-2026-09-24.json").read_text(
            encoding="utf-8"
        )
    )
    taichi_metal = json.loads(
        (HERE / "results/apple-m1-taichi-metal-matched-2026-09-24.json").read_text(
            encoding="utf-8"
        )
    )
    halide_cpu = json.loads(
        (HERE / "results/apple-m1-halide-cpu-matched-2026-09-24.json").read_text(
            encoding="utf-8"
        )
    )
    halide_metal = json.loads(
        (HERE / "results/apple-m1-halide-metal-matched-2026-09-24.json").read_text(
            encoding="utf-8"
        )
    )
    cpu = [
        ModePairRow(
            "Mech · Cranelift SIMD/JIT",
            "same .mec source",
            runtime_rows["Mech SIMD/JIT CPU, checked (8 workers)"]["samples"],
            runtime_rows["Mech SIMD/JIT CPU, unchecked (8 workers)"]["samples"],
        ),
        ModePairRow(
            "Taichi · LLVM CPU",
            "same .py source",
            taichi_cpu["rows"]["checked"]["samples_million_ekf_turns_per_second"],
            taichi_cpu["rows"]["unchecked"]["samples_million_ekf_turns_per_second"],
        ),
        ModePairRow(
            "Halide · native CPU",
            "same .cpp source",
            halide_cpu["rows"]["checked"]["samples_million_ekf_turns_per_second"],
            halide_cpu["rows"]["unchecked"]["samples_million_ekf_turns_per_second"],
        ),
    ]
    gpu = [
        ModePairRow(
            "Mech · generated MSL",
            "same .mec source",
            mech_metal["checked"]["samples_million_ekf_turns_per_second"],
            mech_metal["unchecked"]["samples_million_ekf_turns_per_second"],
        ),
        ModePairRow(
            "Taichi · native Metal",
            "same .py source",
            taichi_metal["rows"]["checked"]["samples_million_ekf_turns_per_second"],
            taichi_metal["rows"]["unchecked"]["samples_million_ekf_turns_per_second"],
        ),
        ModePairRow(
            "Halide · Metal schedule",
            "same .cpp source",
            halide_metal["rows"]["checked"]["samples_million_ekf_turns_per_second"],
            halide_metal["rows"]["unchecked"]["samples_million_ekf_turns_per_second"],
        ),
    ]
    return cpu, gpu


def render_portable_chart(
    filename: str,
    rows: list[ModePairRow],
    title: str,
    subtitle: str,
    maximum: float,
    tick_step: int,
    gpu: bool,
) -> None:
    width, height = 1800, 700
    left, plot_right, top, axis_y = 385, 1390, 205, 512
    chart_width = plot_right - left

    def x(value: float) -> float:
        return left + chart_width * value / maximum

    prefix = "portable-gpu" if gpu else "portable-cpu"
    patterns = ["<defs>"]
    if gpu:
        for row in rows:
            identifier = f"{prefix}-{slug(series_name(row.label))}"
            patterns.append(
                f'<pattern id="{identifier}" width="8" height="8" patternUnits="userSpaceOnUse" patternTransform="rotate(30)"><rect width="8" height="8" fill="{series_color(row.label)}"/><path d="M0 0V8" stroke="#080c14" stroke-opacity="0.52" stroke-width="2"/></pattern>'
            )
    patterns.append("</defs>")
    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="{prefix}-title {prefix}-desc">',
        f'<title id="{prefix}-title">{esc(title)}</title>',
        f'<desc id="{prefix}-desc">Paired horizontal bars compare checked and unchecked median EKF throughput for Mech, Taichi, and Halide, each selecting this backend from the same application source used for its other device. Whiskers show observed minimum to maximum.</desc>',
        *patterns,
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#9aa8ba}.grid{stroke:#2a374c;stroke-width:1}.row-guide{stroke:#182235;stroke-width:1}.axis{fill:#9aa8ba;font-size:14px}.label{font-size:19px;font-weight:600}.detail{fill:#9aa8ba;font-size:13px}.value{font-size:13px;font-variant-numeric:tabular-nums}.whisker{stroke:#eef3fa;stroke-width:2}.footnote{fill:#aab5c5;font-size:13px}</style>',
        f'<text x="42" y="48" font-size="28" font-weight="700">{esc(title)}</text>',
        f'<text x="42" y="80" class="muted" font-size="16">{esc(subtitle)}</text>',
        '<text x="42" y="112" class="muted" font-size="13">Bars are medians; +high/−low and whiskers are the full observed process min–max, not confidence intervals.</text>',
        '<rect x="1090" y="136" width="24" height="14" rx="2" fill="#e8edf5" opacity="0.95"/><text x="1125" y="149" class="muted" font-size="14">checked</text>',
        '<rect x="1210" y="136" width="24" height="14" rx="2" fill="#e8edf5" opacity="0.48"/><text x="1245" y="149" class="muted" font-size="14">unchecked</text>',
        '<text x="1418" y="150" class="muted" font-size="13">median +high/−low M turns/s</text>',
    ]
    for tick in range(0, int(maximum) + 1, tick_step):
        tick_x = x(float(tick))
        lines.append(
            f'<line x1="{tick_x:.1f}" y1="{top - 28}" x2="{tick_x:.1f}" y2="{axis_y}" class="grid"/>'
        )
        lines.append(
            f'<text x="{tick_x:.1f}" y="{axis_y + 27}" text-anchor="middle" class="axis">{tick}</text>'
        )
    for index, row in enumerate(rows):
        y = top + index * 96
        lines.append(
            f'<line x1="{left}" y1="{y + 44}" x2="{plot_right}" y2="{y + 44}" class="row-guide"/>'
        )
        lines.append(
            f'<text x="{left - 24}" y="{y - 4}" text-anchor="end" class="label">{esc(row.label)}</text>'
        )
        lines.append(
            f'<text x="{left - 24}" y="{y + 18}" text-anchor="end" class="detail">{esc(row.detail)}</text>'
        )
        for mode, samples, mode_y, opacity in (
            ("checked", row.checked, y - 13, 0.95),
            ("unchecked", row.unchecked, y + 13, 0.48),
        ):
            median, low, high = statistics.median(samples), min(samples), max(samples)
            fill = series_color(row.label)
            if gpu:
                fill = f"url(#{prefix}-{slug(series_name(row.label))})"
            lines.append(
                f'<rect x="{left}" y="{mode_y - 9}" width="{max(1.0, x(median) - left):.1f}" height="18" rx="3" fill="{fill}" opacity="{opacity}"><title>{esc(evidence_name(row.label))} · {mode} median: {median:.3f} million turns/s</title></rect>'
            )
            lines.append(
                f'<line x1="{x(low):.1f}" y1="{mode_y}" x2="{x(high):.1f}" y2="{mode_y}" class="whisker"/>'
            )
            for endpoint in (low, high):
                lines.append(
                    f'<line x1="{x(endpoint):.1f}" y1="{mode_y - 6}" x2="{x(endpoint):.1f}" y2="{mode_y + 6}" class="whisker"/>'
                )
            lines.append(
                f'<text x="1418" y="{mode_y + 5}" class="value">{mode[0].upper()} {spread_label(samples)} · n={len(samples)}</text>'
            )
    device_note = (
        "Diagonal hatch denotes GPU. " if gpu else ""
    ) + "Highest retained median in both modes is Mech; this is a descriptive result, not a language-speed claim."
    lines.extend(
        [
            f'<line x1="{left}" y1="{axis_y}" x2="{plot_right}" y2="{axis_y}" class="grid"/>',
            f'<text x="{left + chart_width / 2:.1f}" y="{axis_y + 58}" text-anchor="middle" class="muted" font-size="15">million EKF turns per second</text>',
            '<text x="42" y="620" class="footnote">Same application source within each system: Mech .mec, Taichi .py, Halide .cpp; CPU/Metal selected without rewriting EKF equations.</text>',
            f'<text x="42" y="647" class="footnote">{esc(device_note)}</text>',
            '<text x="42" y="674" class="footnote">Toolchains, compiler lowering, schedules, status observation, campaign dates, and system state differ. All samples reported zero faults.</text>',
            "</svg>",
        ]
    )
    write_svg(filename, lines)


def render_portable_combo(cpu: list[ModePairRow], gpu: list[ModePairRow]) -> None:
    """Render a poster-oriented two-panel summary with the claim limits in-frame."""
    width, height = 1800, 1120
    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="portable-combo-title portable-combo-desc">',
        '<title id="portable-combo-title">One EKF source per system, two execution backends</title>',
        '<desc id="portable-combo-desc">A two-panel poster chart compares checked and unchecked EKF throughput on CPU and Metal for Mech, Taichi, and Halide. Each system selects both backends from the same application source. Mech has the highest measured median in both panels; notes state that the result is descriptive and not a general language ranking.</desc>',
        '<defs>',
    ]
    for row in gpu:
        identifier = f"combo-gpu-{slug(series_name(row.label))}"
        lines.append(
            f'<pattern id="{identifier}" width="8" height="8" patternUnits="userSpaceOnUse" patternTransform="rotate(30)"><rect width="8" height="8" fill="{series_color(row.label)}"/><path d="M0 0V8" stroke="#080c14" stroke-opacity="0.52" stroke-width="2"/></pattern>'
        )
    lines.extend(
        [
            "</defs>",
            '<rect width="100%" height="100%" fill="#080c14"/>',
            '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#9aa8ba}.panel{fill:#0d1421;stroke:#2a374c;stroke-width:1}.grid{stroke:#2a374c;stroke-width:1}.label{font-size:16px;font-weight:600}.detail{fill:#9aa8ba;font-size:12px}.value{font-size:12px;font-variant-numeric:tabular-nums}.axis{fill:#9aa8ba;font-size:12px}.whisker{stroke:#eef3fa;stroke-width:2}.note{fill:#aab5c5;font-size:14px}</style>',
            '<text x="42" y="54" font-size="30" font-weight="700">One EKF source per system, two execution backends</text>',
            '<text x="42" y="87" class="muted" font-size="16">Mech, Taichi, and Halide each select CPU or Metal without rewriting the EKF equations</text>',
            '<rect x="42" y="111" width="24" height="14" rx="2" fill="#e8edf5" opacity="0.95"/><text x="77" y="124" class="muted" font-size="14">checked</text>',
            '<rect x="162" y="111" width="24" height="14" rx="2" fill="#e8edf5" opacity="0.48"/><text x="197" y="124" class="muted" font-size="14">unchecked</text>',
            '<text x="312" y="124" class="muted" font-size="13">bars: median · whiskers/+−: observed process min–max</text>',
        ]
    )

    panels = [
        (cpu, 42, "CPU · eight workers", 130.0, 20, False),
        (gpu, 920, "GPU · Apple Metal", 450.0, 50, True),
    ]
    for rows, panel_x, panel_title, maximum, tick_step, is_gpu in panels:
        panel_y, panel_w, panel_h = 153, 838, 536
        plot_left, plot_right, top, axis_y = panel_x + 188, panel_x + 660, 274, 560

        def px(value: float) -> float:
            return plot_left + (plot_right - plot_left) * value / maximum

        lines.append(
            f'<rect x="{panel_x}" y="{panel_y}" width="{panel_w}" height="{panel_h}" rx="14" class="panel"/>'
        )
        lines.append(
            f'<text x="{panel_x + 24}" y="{panel_y + 42}" font-size="22" font-weight="700">{esc(panel_title)}</text>'
        )
        lines.append(
            f'<text x="{panel_x + 24}" y="{panel_y + 68}" class="muted" font-size="13">500,000 filters × 40 turns · f32 · synchronized after every turn</text>'
        )
        for tick in range(0, int(maximum) + 1, tick_step):
            tick_x = px(float(tick))
            lines.append(
                f'<line x1="{tick_x:.1f}" y1="{top - 35}" x2="{tick_x:.1f}" y2="{axis_y}" class="grid"/>'
            )
            lines.append(
                f'<text x="{tick_x:.1f}" y="{axis_y + 23}" text-anchor="middle" class="axis">{tick}</text>'
            )
        for index, row in enumerate(rows):
            y = top + index * 88
            lines.append(
                f'<text x="{plot_left - 16}" y="{y - 3}" text-anchor="end" class="label">{esc(series_name(row.label))}</text>'
            )
            lines.append(
                f'<text x="{plot_left - 16}" y="{y + 16}" text-anchor="end" class="detail">{esc(row.label.split(" · ", 1)[1])}</text>'
            )
            for mode, samples, mode_y, opacity in (
                ("checked", row.checked, y - 13, 0.95),
                ("unchecked", row.unchecked, y + 13, 0.48),
            ):
                median, low, high = statistics.median(samples), min(samples), max(samples)
                fill = series_color(row.label)
                if is_gpu:
                    fill = f"url(#combo-gpu-{slug(series_name(row.label))})"
                lines.append(
                    f'<rect x="{plot_left}" y="{mode_y - 9}" width="{max(1.0, px(median) - plot_left):.1f}" height="18" rx="3" fill="{fill}" opacity="{opacity}"><title>{esc(series_name(row.label))} {esc(panel_title)} {mode}: {median:.3f} million turns/s</title></rect>'
                )
                lines.append(
                    f'<line x1="{px(low):.1f}" y1="{mode_y}" x2="{px(high):.1f}" y2="{mode_y}" class="whisker"/>'
                )
                for endpoint in (low, high):
                    lines.append(
                        f'<line x1="{px(endpoint):.1f}" y1="{mode_y - 5}" x2="{px(endpoint):.1f}" y2="{mode_y + 5}" class="whisker"/>'
                    )
                lines.append(
                    f'<text x="{panel_x + 674}" y="{mode_y + 4}" class="value">{mode[0].upper()} {spread_label(samples)} · n={len(samples)}</text>'
                )
        lines.append(
            f'<text x="{panel_x + panel_w / 2:.1f}" y="{panel_y + panel_h - 25}" text-anchor="middle" class="muted" font-size="13">million EKF turns per second</text>'
        )
    lines.extend(
        [
            '<text x="42" y="750" font-size="22" font-weight="700">What these measurements support</text>',
            '<text x="42" y="784" class="note">• Mech has the highest retained median in both panels, checked and unchecked, for this EKF on this Apple M1.</text>',
            '<text x="42" y="812" class="note">• That is a descriptive measurement—not evidence that Mech is intrinsically faster than Taichi or Halide.</text>',
            '<text x="42" y="840" class="note">• Same-source means one application file per system: Mech .mec to Cranelift SIMD/JIT or generated MSL;</text>',
            '<text x="62" y="864" class="note">Taichi .py to LLVM CPU or Metal; Halide .cpp to native CPU schedule or Metal schedule.</text>',
            '<text x="42" y="908" font-size="18" font-weight="700">Poster notes</text>',
            '<text x="42" y="936" class="muted" font-size="13">Workload: 500,000 filters × 40 turns, f32, resident ping-pong state, one synchronized publication per turn; CPU uses eight workers.</text>',
            '<text x="42" y="962" class="muted" font-size="13">Statistic: median; +high/−low is full observed process min–max. n=3 Mech CPU, n=5 Mech GPU, n=7 Taichi/Halide. No CI or p-value.</text>',
            '<text x="42" y="988" class="muted" font-size="13">Toolchains: Mech v0.4 integration / Cranelift / direct Metal; Taichi 1.7.4 / LLVM 15 / Python 3.12.14; Halide 21.0.0_1 / Apple clang 17.</text>',
            '<text x="42" y="1014" class="muted" font-size="13">Differences: compiler lowering, schedules, compact versus per-lane fault observation, campaign dates, and system state. All samples reported zero faults.</text>',
            '<text x="42" y="1060" class="muted" font-size="13">Diagonal hatch denotes GPU. Raw samples, source hashes, exact commands, and provenance are retained in this repository.</text>',
            "</svg>",
        ]
    )
    write_svg("post-portable-combo.svg", lines)


def render_cross_language() -> None:
    fused = load("apple-m1-fused-reference-controls-2026-08-31.json")
    mech_unchecked = load("apple-m1-mech-persistent-simd-2026-08-31.json")
    mojo = load("apple-m1-mojo-advanced-2026-09-04.json")
    futhark = load("apple-m1-futhark-ispc-fixed-2026-08-31.json")
    rows = [
        ModePairRow(
            "Mech",
            "SIMD/JIT · f32x4",
            fused["rows"]["mech_fused_checked"]["throughput_millions"],
            mech_unchecked["rows"]["fused_unchecked_block"]["throughput_millions"],
        ),
        ModePairRow(
            "Rust",
            "packed SIMD · f32x4",
            fused["rows"]["rust_fused_checked"]["throughput_millions"],
            fused["rows"]["rust_fused"]["throughput_millions"],
        ),
        ModePairRow(
            "Mojo",
            "explicit SIMD-4",
            mojo["rows"]["Mojo fused SIMD-4, 8 workers"]["checked"]
            ["samples_million_ekf_turns_per_second"],
            mojo["rows"]["Mojo fused SIMD-4, 8 workers"]["unchecked"]
            ["samples_million_ekf_turns_per_second"],
        ),
        ModePairRow(
            "Julia",
            "SIMD.jl",
            fused["rows"]["julia_fused_checked"]["throughput_millions"],
            fused["rows"]["julia_fused"]["throughput_millions"],
        ),
        ModePairRow(
            "Futhark",
            "ISPC AOT",
            futhark["rows"]["checked"]["throughput_millions"],
            futhark["rows"]["unchecked"]["throughput_millions"],
        ),
        ModePairRow(
            "NumPy/Numba",
            "compiled parallel kernel",
            fused["rows"]["numba_fused_checked"]["throughput_millions"],
            fused["rows"]["numba_fused"]["throughput_millions"],
        ),
    ]

    width, height = 1800, 900
    left, plot_right, top = 355, 1450, 190
    axis_y = 765
    chart_width = plot_right - left
    maximum = 185.0

    def x(value: float) -> float:
        return left + chart_width * value / maximum

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="cross-title cross-desc">',
        '<title id="cross-title">Comparable checked and unchecked CPU EKF throughput across six implementations</title>',
        '<desc id="cross-desc">Paired horizontal bars show median checked and unchecked throughput for Mech, Rust, Mojo, Julia, Futhark, and NumPy with Numba on one Apple M1. Thin whiskers and value labels report the observed minimum-to-maximum spread.</desc>',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#9aa8ba}.grid{stroke:#2a374c;stroke-width:1}.row-guide{stroke:#182235;stroke-width:1}.axis{fill:#9aa8ba;font-size:14px}.label{font-size:19px;font-weight:600}.detail{fill:#9aa8ba;font-size:13px}.value{font-size:13px;font-variant-numeric:tabular-nums}.whisker{stroke:#eef3fa;stroke-width:2}.footnote{fill:#aab5c5;font-size:13px}</style>',
        '<text x="42" y="48" font-size="28" font-weight="700">Comparable CPU EKF throughput across six implementations</text>',
        '<text x="42" y="80" class="muted" font-size="16">Apple M1 · 500,000 filters × 40 turns · f32 · eight workers · fused CPU execution</text>',
        '<text x="42" y="112" class="muted" font-size="13">Bars are medians; thin whiskers and +/− labels are observed min–max spread, not confidence intervals.</text>',
        '<rect x="1125" y="136" width="24" height="14" rx="2" fill="#e8edf5" opacity="0.95"/><text x="1160" y="149" class="muted" font-size="14">checked</text>',
        '<rect x="1245" y="136" width="24" height="14" rx="2" fill="#e8edf5" opacity="0.48"/><text x="1280" y="149" class="muted" font-size="14">unchecked</text>',
        '<text x="1435" y="150" class="muted" font-size="13">median +high/−low M turns/s</text>',
    ]

    for tick in range(0, 181, 20):
        tick_x = x(float(tick))
        lines.append(
            f'<line x1="{tick_x:.1f}" y1="{top - 25}" x2="{tick_x:.1f}" y2="{axis_y}" class="grid"/>'
        )
        lines.append(
            f'<text x="{tick_x:.1f}" y="{axis_y + 27}" text-anchor="middle" class="axis">{tick}</text>'
        )

    for index, row in enumerate(rows):
        y = top + index * 92
        lines.append(
            f'<line x1="{left}" y1="{y + 43}" x2="{plot_right}" y2="{y + 43}" class="row-guide"/>'
        )
        lines.append(
            f'<text x="{left - 24}" y="{y - 4}" text-anchor="end" class="label">{esc(row.label)}</text>'
        )
        lines.append(
            f'<text x="{left - 24}" y="{y + 18}" text-anchor="end" class="detail">{esc(row.detail)}</text>'
        )
        for mode, samples, mode_y, opacity in (
            ("checked", row.checked, y - 13, 0.95),
            ("unchecked", row.unchecked, y + 13, 0.48),
        ):
            median = statistics.median(samples)
            low, high = min(samples), max(samples)
            color = series_color(row.label)
            lines.append(
                f'<rect x="{left}" y="{mode_y - 9}" width="{max(1.0, x(median) - left):.1f}" height="18" rx="3" fill="{color}" opacity="{opacity}"><title>{esc(series_name(row.label))} · {mode} median: {median:.3f} million turns/s</title></rect>'
            )
            lines.append(
                f'<line x1="{x(low):.1f}" y1="{mode_y}" x2="{x(high):.1f}" y2="{mode_y}" class="whisker"/>'
            )
            for endpoint in (low, high):
                lines.append(
                    f'<line x1="{x(endpoint):.1f}" y1="{mode_y - 6}" x2="{x(endpoint):.1f}" y2="{mode_y + 6}" class="whisker"/>'
            )
            lines.append(
                f'<text x="1435" y="{mode_y + 5}" class="value">{mode[0].upper()} {spread_label(samples)} · n={len(samples)}</text>'
            )

    lines.extend(
        [
            f'<line x1="{left}" y1="{axis_y}" x2="{plot_right}" y2="{axis_y}" class="grid"/>',
            f'<text x="{left + chart_width / 2:.1f}" y="{axis_y + 58}" text-anchor="middle" class="muted" font-size="15">million EKF turns per second</text>',
            '<text x="42" y="850" class="footnote">All rows match workload, CPU, worker count, and fused execution. Mech/Rust additionally match block-atomic rollback; other fault interfaces differ.</text>',
            '<text x="42" y="875" class="footnote">NumPy/Numba is compiled by Numba, not interpreted Python. All measured samples reported zero faults.</text>',
            '</svg>',
        ]
    )
    write_svg("post-cross-language-comparison.svg", lines)


def render_metal_comparison() -> None:
    mech = load("apple-m1-mech-metal-2026-09-04.json")
    mojo = json.loads(
        (HERE / "results/apple-m1-mojo-metal-matched-2026-09-24.json").read_text(
            encoding="utf-8"
        )
    )
    julia = json.loads(
        (HERE / "results/apple-m1-julia-metal-matched-2026-09-24.json").read_text(
            encoding="utf-8"
        )
    )
    taichi = json.loads(
        (HERE / "results/apple-m1-taichi-metal-matched-2026-09-24.json").read_text(
            encoding="utf-8"
        )
    )
    halide = json.loads(
        (HERE / "results/apple-m1-halide-metal-matched-2026-09-24.json").read_text(
            encoding="utf-8"
        )
    )
    rust = json.loads(
        (HERE / "results/apple-m1-rust-metal-2026-09-24.json").read_text(
            encoding="utf-8"
        )
    )
    mech_row = mech["rows"]["Mech direct Metal generated from generic scalar IR"]
    rows = [
        ModePairRow(
            "Mech · generated MSL",
            "",
            mech_row["checked"]["samples_million_ekf_turns_per_second"],
            mech_row["unchecked"]["samples_million_ekf_turns_per_second"],
        ),
        ModePairRow(
            "Rust · hand-written MSL",
            "",
            rust["rows"]["checked"]["samples_million_ekf_turns_per_second"],
            rust["rows"]["unchecked"]["samples_million_ekf_turns_per_second"],
        ),
        ModePairRow(
            "Mojo · native Metal",
            "",
            mojo["rows"]["checked"]["samples_million_ekf_turns_per_second"],
            mojo["rows"]["unchecked"]["samples_million_ekf_turns_per_second"],
        ),
        ModePairRow(
            "Julia · Metal.jl",
            "",
            julia["rows"]["checked"]["samples_million_ekf_turns_per_second"],
            julia["rows"]["unchecked"]["samples_million_ekf_turns_per_second"],
        ),
        ModePairRow(
            "Taichi · native Metal",
            "",
            taichi["rows"]["checked"]["samples_million_ekf_turns_per_second"],
            taichi["rows"]["unchecked"]["samples_million_ekf_turns_per_second"],
        ),
        ModePairRow(
            "Halide · Metal schedule",
            "",
            halide["rows"]["checked"]["samples_million_ekf_turns_per_second"],
            halide["rows"]["unchecked"]["samples_million_ekf_turns_per_second"],
        ),
    ]

    width, height = 1800, 900
    left, plot_right, top = 355, 1450, 190
    axis_y = 765
    chart_width = plot_right - left
    maximum = 460.0

    def x(value: float) -> float:
        return left + chart_width * value / maximum

    patterns = ["<defs>"]
    for row in rows:
        name = series_name(row.label)
        identifier = f"metal-{slug(name)}"
        patterns.append(
            f'<pattern id="{identifier}" width="8" height="8" patternUnits="userSpaceOnUse" patternTransform="rotate(30)"><rect width="8" height="8" fill="{series_color(row.label)}"/><path d="M0 0V8" stroke="#080c14" stroke-opacity="0.52" stroke-width="2"/></pattern>'
        )
    patterns.append("</defs>")

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="metal-title metal-desc">',
        '<title id="metal-title">Apple M1 Metal EKF throughput across six implementation ecosystems</title>',
        '<desc id="metal-desc">Paired horizontal hatched bars show median checked and unchecked Metal throughput for Mech, a Rust host with hand-written MSL, Mojo, Julia, Taichi, and Halide. Thin whiskers and value labels report observed minimum-to-maximum spread.</desc>',
        *patterns,
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#9aa8ba}.grid{stroke:#2a374c;stroke-width:1}.row-guide{stroke:#182235;stroke-width:1}.axis{fill:#9aa8ba;font-size:14px}.label{font-size:19px;font-weight:600}.detail{fill:#9aa8ba;font-size:13px}.value{font-size:13px;font-variant-numeric:tabular-nums}.whisker{stroke:#eef3fa;stroke-width:2}.footnote{fill:#aab5c5;font-size:13px}</style>',
        '<text x="42" y="48" font-size="28" font-weight="700">Apple M1 Metal EKF throughput</text>',
        '<text x="42" y="80" class="muted" font-size="16">500,000 filters × 40 turns · f32 · resident GPU state · synchronized publication after every turn</text>',
        '<text x="42" y="112" class="muted" font-size="13">Bars are medians; thin whiskers and +/− labels are observed min–max spread, not confidence intervals. Diagonal hatch denotes GPU.</text>',
        '<rect x="1125" y="136" width="24" height="14" rx="2" fill="url(#metal-mech)" opacity="0.95"/><text x="1160" y="149" class="muted" font-size="14">checked</text>',
        '<rect x="1245" y="136" width="24" height="14" rx="2" fill="url(#metal-mech)" opacity="0.48"/><text x="1280" y="149" class="muted" font-size="14">unchecked</text>',
        '<text x="1435" y="150" class="muted" font-size="13">median +high/−low M turns/s</text>',
    ]

    for tick in range(0, 451, 50):
        tick_x = x(float(tick))
        lines.append(
            f'<line x1="{tick_x:.1f}" y1="{top - 25}" x2="{tick_x:.1f}" y2="{axis_y}" class="grid"/>'
        )
        lines.append(
            f'<text x="{tick_x:.1f}" y="{axis_y + 27}" text-anchor="middle" class="axis">{tick}</text>'
        )

    for index, row in enumerate(rows):
        y = top + index * 92
        lines.append(
            f'<line x1="{left}" y1="{y + 43}" x2="{plot_right}" y2="{y + 43}" class="row-guide"/>'
        )
        lines.append(
            f'<text x="{left - 24}" y="{y - 4}" text-anchor="end" class="label">{esc(row.label)}</text>'
        )
        for mode, samples, mode_y, opacity in (
            ("checked", row.checked, y - 13, 0.95),
            ("unchecked", row.unchecked, y + 13, 0.48),
        ):
            median = statistics.median(samples)
            low, high = min(samples), max(samples)
            fill = f"url(#metal-{slug(series_name(row.label))})"
            lines.append(
                f'<rect x="{left}" y="{mode_y - 9}" width="{max(1.0, x(median) - left):.1f}" height="18" rx="3" fill="{fill}" opacity="{opacity}"><title>{esc(evidence_name(row.label))} · {mode} median: {median:.3f} million turns/s</title></rect>'
            )
            lines.append(
                f'<line x1="{x(low):.1f}" y1="{mode_y}" x2="{x(high):.1f}" y2="{mode_y}" class="whisker"/>'
            )
            for endpoint in (low, high):
                lines.append(
                    f'<line x1="{x(endpoint):.1f}" y1="{mode_y - 6}" x2="{x(endpoint):.1f}" y2="{mode_y + 6}" class="whisker"/>'
            )
            lines.append(
                f'<text x="1435" y="{mode_y + 5}" class="value">{mode[0].upper()} {spread_label(samples)} · n={len(samples)}</text>'
            )

    lines.extend(
        [
            f'<line x1="{left}" y1="{axis_y}" x2="{plot_right}" y2="{axis_y}" class="grid"/>',
            f'<text x="{left + chart_width / 2:.1f}" y="{axis_y + 58}" text-anchor="middle" class="muted" font-size="15">million EKF turns per second</text>',
            '<text x="42" y="850" class="footnote">All six paths use resident packed SoA with ping-pong publication and one synchronized turn boundary.</text>',
            '<text x="42" y="875" class="footnote">Rust + MSL is a Rust host dispatching hand-written MSL, not Rust source compiled to Metal. Read small gaps as non-ranking.</text>',
            '</svg>',
        ]
    )
    write_svg("post-metal-comparison.svg", lines)


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
        '<desc id="mech-desc">Eight checked execution backends for the same high-level Mech EKF on one Apple M1. Horizontal bars show medians and thin whiskers show observed minimum-to-maximum ranges. A logarithmic scale keeps the scalar and GPU backends visible together.</desc>',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#9aa8ba}.grid{stroke:#2a374c;stroke-width:1}.minor-grid{stroke:#1a2538;stroke-width:1}.row-guide{stroke:#182235;stroke-width:1}.axis{fill:#9aa8ba;font-size:14px}.label{font-size:18px;font-weight:600}.detail{fill:#9aa8ba;font-size:13px}.value{font-size:14px;font-variant-numeric:tabular-nums}.whisker{stroke:#e8edf5;stroke-width:2.5}.footnote{fill:#aab5c5;font-size:13px}</style>',
        '<text x="42" y="48" font-size="28" font-weight="700">One Mech EKF across eight execution backends</text>',
        '<text x="42" y="80" class="muted" font-size="16">Checked publication after every turn · same Apple M1 · raw retained process samples · workload shown per row</text>',
        '<text x="42" y="112" class="muted" font-size="13">Bars are medians; whiskers and +/− labels are observed min–max ranges, not confidence intervals. Logarithmic throughput scale.</text>',
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
        median_x = x(median)
        lines.append(
            f'<rect x="{x(minimum):.1f}" y="{y - 10}" width="{max(1.0, median_x - x(minimum)):.1f}" height="20" rx="3" fill="{row.color}" opacity="0.9"><title>{esc(row.label)} median: {median:.3f} million turns/s</title></rect>'
        )
        lines.append(
            f'<line x1="{x(low):.1f}" y1="{y}" x2="{x(high):.1f}" y2="{y}" class="whisker"/>'
        )
        for endpoint in (low, high):
            lines.append(
                f'<line x1="{x(endpoint):.1f}" y1="{y - 11}" x2="{x(endpoint):.1f}" y2="{y + 11}" class="whisker"/>'
            )
        lines.append(
            f'<text x="{x(high) + 18:.1f}" y="{y + 5}" class="value">{spread_label(row.samples)} · n={len(row.samples)}</text>'
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
    render_metal_comparison()
    render_mech_backends()
    cpu, gpu = portable_rows()
    render_portable_chart(
        "post-portable-cpu-comparison.svg",
        cpu,
        "Same-source CPU backends: Mech, Taichi, and Halide",
        "Apple M1 · 500,000 filters × 40 turns · f32 · eight workers · synchronized publication after every turn",
        130.0,
        20,
        False,
    )
    render_portable_chart(
        "post-portable-metal-comparison.svg",
        gpu,
        "Same-source Metal backends: Mech, Taichi, and Halide",
        "Apple M1 · 500,000 filters × 40 turns · f32 · resident GPU state · synchronized publication after every turn",
        450.0,
        50,
        True,
    )
    render_portable_combo(cpu, gpu)


if __name__ == "__main__":
    main()
