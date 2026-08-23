"""Canonical Chromium flags for browser WebGPU acceptance tests."""

from __future__ import annotations


def chrome_webgpu_test_flags(*, software_adapter: bool) -> list[str]:
    """Return WebGPU flags, optionally forcing Chromium's test adapter."""

    flags = ["--enable-unsafe-webgpu"]
    if software_adapter:
        flags.extend(
            [
                "--enable-unsafe-swiftshader",
                "--use-webgpu-adapter=swiftshader",
                "--use-gpu-in-tests",
            ]
        )
    return flags
