"""Shared machine specification box for the archived EKF SVG charts."""

from __future__ import annotations

import html


MACHINE_LINES = (
    "Test machine",
    "Mac mini Macmini9,1 | Apple M1 | 8-core CPU (4P + 4E)",
    "8 GB LPDDR4 | 8-core integrated Apple M1 GPU | Metal 3",
    "macOS 15.6.1 | arm64",
)


def svg_machine_specs(width: int, height: int, right: int = 24, bottom: int = 18) -> str:
    """Return a bottom-right machine specification box."""
    box_width = 390
    box_height = 72
    x = width - right - box_width
    y = height - bottom - box_height
    title, *details = MACHINE_LINES
    lines = [
        f'<g aria-label="{html.escape(title, quote=True)}">',
        f'<rect x="{x}" y="{y}" width="{box_width}" height="{box_height}" rx="4" fill="#111827" stroke="#334155"/>',
        f'<text x="{x + 12}" y="{y + 16}" fill="#e8edf5" font-size="11" font-weight="700">{html.escape(title, quote=True)}</text>',
    ]
    for index, detail in enumerate(details, start=1):
        lines.append(
            f'<text x="{x + 12}" y="{y + 16 + index * 15}" fill="#91a0b5" font-size="10">{html.escape(detail, quote=True)}</text>'
        )
    lines.append("</g>")
    return (
        "\n".join(lines)
    )
