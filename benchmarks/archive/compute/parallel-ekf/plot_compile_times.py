#!/usr/bin/env python3
"""Render the recorded EKF compiler-time evidence as a grouped bar chart."""

from __future__ import annotations

import argparse
import html
import json
import statistics
from pathlib import Path

from chart_machine_specs import svg_machine_specs
from source_diff_report import COLORS


HERE = Path(__file__).resolve().parent


def render(data: dict) -> str:
    rows = data.get("rows", {})
    languages = [language for language in COLORS if language in rows]
    languages.extend(language for language in rows if language not in languages)
    languages = [language for language in languages if rows[language].get("baseline") or rows[language].get("advanced")]
    width = 1500
    left = 250
    right = 190
    top = 130
    row_height = 52
    bottom = 70
    chart_width = width - left - right
    height = top + row_height * len(languages) + bottom

    def median(entry: dict | None) -> float | None:
        if not isinstance(entry, dict) or not entry.get("available"):
            return None
        value = entry.get("median_milliseconds", entry.get("milliseconds"))
        if isinstance(value, list):
            value = statistics.median(value) if value else None
        return None if value is None else float(value)

    maximum = max(
        [value for language in languages for value in (median(rows[language].get("baseline")), median(rows[language].get("advanced"))) if value is not None]
        or [1.0]
    )
    scale_max = maximum * 1.18
    colors = {language: COLORS.get(language, "#91a0b5") for language in languages}

    def esc(value: object) -> str:
        return html.escape(str(value), quote=True)

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#91a0b5}.grid{stroke:#263246;stroke-width:1}.label{font-size:15px}.value{font-size:13px;font-variant-numeric:tabular-nums}.phase{font-size:11px;fill:#91a0b5}</style>',
        '<text x="42" y="42" font-size="28" font-weight="700">Parallel EKF compile and first-run cost</text>',
        '<text x="42" y="70" class="muted" font-size="15">Apple M1 medians in milliseconds; each language shows baseline / advanced source controls</text>',
        f'<text x="{left}" y="108" font-size="15" font-weight="600">wall time (ms)</text>',
        '<rect x="1080" y="91" width="14" height="14" fill="#91a0b5"/><text x="1102" y="103" class="muted" font-size="13">baseline</text>',
        '<rect x="1180" y="91" width="14" height="14" fill="#91a0b5" opacity="0.45"/><text x="1202" y="103" class="muted" font-size="13">advanced</text>',
    ]
    for tick in range(0, 7):
        value = scale_max * tick / 6
        x = left + chart_width * tick / 6
        lines.append(f'<line x1="{x:.1f}" y1="{top - 15}" x2="{x:.1f}" y2="{height - bottom}" class="grid"/>')
        lines.append(f'<text x="{x:.1f}" y="{height - bottom + 22}" text-anchor="middle" class="muted" font-size="12">{value:.0f}</text>')
    for index, language in enumerate(languages):
        y = top + index * row_height
        baseline = median(rows[language].get("baseline"))
        advanced = median(rows[language].get("advanced"))
        color = colors[language]
        lines.append(f'<text x="{left - 16}" y="{y + 25}" text-anchor="end" class="label">{esc(language)}</text>')
        for offset, value, opacity in ((5, baseline, 0.95), (27, advanced, 0.45)):
            if value is None:
                lines.append(f'<text x="{left + 8}" y="{y + offset + 13}" class="phase">not measured</text>')
                continue
            bar_width = chart_width * value / scale_max
            lines.append(f'<rect x="{left}" y="{y + offset}" width="{max(1.0, bar_width):.1f}" height="15" rx="2" fill="{color}" opacity="{opacity}"/>')
            label_x = min(left + bar_width + 8, width - right + 4)
            lines.append(f'<text x="{label_x:.1f}" y="{y + offset + 12}" class="value">{value:.3f}</text>')
        base_entry = rows[language].get("baseline", {})
        advanced_entry = rows[language].get("advanced", {})
        base_phase = base_entry.get("phase", "") if isinstance(base_entry, dict) else ""
        advanced_phase = advanced_entry.get("phase", "") if isinstance(advanced_entry, dict) else ""
        phase = f"{base_phase} / {advanced_phase}" if base_phase and advanced_phase and base_phase != advanced_phase else (base_phase or advanced_phase)
        lines.append(f'<text x="{width - right + 15}" y="{y + 19}" class="phase">{esc(phase)}</text>')
    lines.append('<text x="42" y="{}" class="muted" font-size="12">AOT and bytecode rows time artifact creation; Julia/Taichi include cold startup and specialization; Mech uses source/artifact preparation.</text>'.format(height - 96))
    lines.append(svg_machine_specs(width, height, right=right, bottom=18))
    lines.append('</svg>')
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(render(json.loads(args.input.read_text(encoding="utf-8"))), encoding="utf-8")


if __name__ == "__main__":
    main()
