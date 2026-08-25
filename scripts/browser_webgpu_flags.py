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
            # Chromium's own Linux WebGPU SwiftShader pixel-test profile keeps
            # ANGLE, Dawn, and Vulkan on the same software implementation and
            # avoids creating a presentation surface in a headless process.
            # Using only the Vulkan switches can destroy Dawn's external
            # instance while submitted work is still pending.
            flags.extend(
                [
                    "--enable-features=Vulkan",
                    "--use-angle=swiftshader",
                    "--use-vulkan=swiftshader",
                    "--disable-vulkan-surface",
                ]
            )
    return flags
