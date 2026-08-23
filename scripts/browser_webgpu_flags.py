"""Canonical Chromium flags for browser WebGPU acceptance tests."""

from __future__ import annotations


def chrome_webgpu_test_flags(*, software_adapter: bool, linux: bool = False) -> list[str]:
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
        if linux:
            # Chromium's Linux WebGPU decoder requires a Vulkan shared
            # context (or GLES compat). Force the same SwiftShader Vulkan
            # driver as the selected Dawn fallback adapter on GPU-less CI.
            flags.extend(["--enable-features=Vulkan", "--use-vulkan=swiftshader"])
    return flags
