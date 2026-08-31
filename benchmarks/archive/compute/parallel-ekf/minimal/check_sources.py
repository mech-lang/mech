#!/usr/bin/env python3
"""Check that compact non-Mech controls only remove source scaffolding."""

from pathlib import Path
import re


HERE = Path(__file__).resolve().parent
PAIRS = (
    (HERE.parents[4] / "hosts/gpu/examples/parallel_ekf_rust_scalar.rs", HERE / "rust_scalar.rs"),
    (HERE.parents[4] / "hosts/gpu/examples/parallel_ekf_rust_simd.rs", HERE / "rust_simd.rs"),
    (HERE.parent / "julia_scalar.jl", HERE / "julia_scalar.jl"),
    (HERE.parent / "julia_simd_intrinsics.jl", HERE / "julia_simd.jl"),
    (HERE.parent / "luajit_scalar.lua", HERE / "luajit_scalar.lua"),
    (HERE.parent / "luajit_fast.lua", HERE / "luajit_fast.lua"),
    (HERE.parent / "taichi_comparable.py", HERE / "taichi_comparable.py"),
    (HERE.parent / "taichi_optimized.py", HERE / "taichi_optimized.py"),
)


def code(path: Path) -> list[str]:
    text = re.sub(r'""".*?"""\n?', "", path.read_text(encoding="utf-8"), flags=re.S)
    result = []
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith(("#", "//", "--")):
            continue
        result.append(line)
    return result


def main() -> None:
    failures = []
    for reference, compact in PAIRS:
        if code(reference) != code(compact):
            failures.append(compact.name)
    if failures:
        raise SystemExit("compact source changed numerical code: " + ", ".join(failures))
    print(f"compact source check passed ({len(PAIRS)} controls)")


if __name__ == "__main__":
    main()
