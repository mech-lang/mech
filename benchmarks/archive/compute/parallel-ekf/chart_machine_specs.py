"""Shared machine label for the archived EKF SVG charts."""

from __future__ import annotations

import html


MACHINE_SPECS = "Machine: Apple M1 | macOS 15.6.1 | arm64"


def svg_machine_specs(width: int, height: int, right: int = 24, bottom: int = 18) -> str:
    """Return a bottom-right SVG label without relying on chart-local styles."""
    label = html.escape(MACHINE_SPECS, quote=True)
    return (
        f'<text x="{width - right}" y="{height - bottom}" text-anchor="end" '
        f'fill="#91a0b5" font-size="12">{label}</text>'
    )
