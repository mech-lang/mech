#!/usr/bin/env python3
"""CuPy probe for the NumPy-compatible GPU lane.

This file intentionally does not fall back to NumPy or Metal libraries.  A
fallback would silently change the backend and make the GPU comparison false.
On an NVIDIA host with CUDA and CuPy installed, this is the place to add the
same per-turn resident CuPy control; on Apple Metal it reports capability
absence and exits successfully so the matrix records an explicit N/A.
"""

from __future__ import annotations

import importlib.util
import platform
import sys


def main() -> int:
    if importlib.util.find_spec("cupy") is None:
        print("lane: NumPy GPU (CuPy)")
        print("available: false")
        print("reason: CuPy is not installed; plain NumPy has no GPU backend")
        return 0
    try:
        import cupy as cp  # type: ignore[import-not-found]

        count = int(cp.cuda.runtime.getDeviceCount())
    except Exception as error:  # pragma: no cover - depends on host CUDA
        print("lane: NumPy GPU (CuPy)")
        print("available: false")
        print(f"reason: CuPy could not access CUDA: {error}")
        return 0
    print("lane: NumPy GPU (CuPy)")
    print("available: true")
    print("cuda_devices: ", count)
    print("platform: ", platform.platform())
    print("status: capability probe only; run on an NVIDIA/CUDA host for a numeric lane")
    return 0


if __name__ == "__main__":
    sys.exit(main())
