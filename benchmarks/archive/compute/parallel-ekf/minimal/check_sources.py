#!/usr/bin/env python3
"""Check copy-only controls and account for declared source specializations."""

from pathlib import Path
import re


HERE = Path(__file__).resolve().parent
PAIRS = (
    (HERE.parents[4] / "hosts/gpu/examples/parallel_ekf_rust_scalar.rs", HERE / "rust_scalar.rs"),
    (HERE.parent / "julia_scalar.jl", HERE / "julia_scalar.jl"),
    (HERE.parent / "julia_simd_intrinsics.jl", HERE / "julia_simd.jl"),
    (HERE.parent / "luajit_scalar.lua", HERE / "luajit_scalar.lua"),
    (HERE.parent / "luajit_fast.lua", HERE / "luajit_fast.lua"),
    (HERE.parent / "taichi_comparable.py", HERE / "taichi_comparable.py"),
    (HERE.parent / "taichi_optimized.py", HERE / "taichi_optimized.py"),
)

SPECIALIZED = (
    HERE / "rust_simd.rs",
    HERE / "rust_scalar_optimized.rs",
    HERE / "lua_advanced.lua",
)


def code(path: Path) -> list[str]:
    text = re.sub(r'""".*?"""\n?', "", path.read_text(encoding="utf-8"), flags=re.S)
    result = []
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith(("#", "//", "--")):
            continue
        # This is a runtime-policy pin, not numerical body code.  It is
        # intentionally present only in the strict Taichi compact control.
        if "\"fast_math\": False" in line:
            continue
        result.append(line)
    # Formatting-only minimization can reflow a statement over several lines;
    # compare the resulting token stream rather than physical line breaks.
    compact = re.sub(r"\s+", "", "\n".join(result))
    compact = re.sub(r",([}\]])", r"\1", compact)
    return [compact]


def main() -> None:
    failures = []
    for reference, compact in PAIRS:
        if code(reference) != code(compact):
            failures.append(compact.name)
    missing_specializations = [str(path) for path in SPECIALIZED if not path.exists()]
    if failures or missing_specializations:
        detail = []
        if failures:
            detail.append("copy-only controls changed: " + ", ".join(failures))
        if missing_specializations:
            detail.append("missing specialized controls: " + ", ".join(missing_specializations))
        raise SystemExit("; ".join(detail))
    print(f"compact source check passed ({len(PAIRS)} copy-only controls; {len(SPECIALIZED)} specialized controls present)")


if __name__ == "__main__":
    main()
