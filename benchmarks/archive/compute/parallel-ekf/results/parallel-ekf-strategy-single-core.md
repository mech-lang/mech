# Parallel EKF: Single-core

Single-worker SIMD/JIT controls where available; scalar controls remain where they are not. Workload: **10,000 filters x 20 turns where available**. Checked and unchecked are separate columns; source edits are measured against each language's baseline source.

**Scope note:** SIMD/JIT controls are identified in the representative-source column; scalar controls remain where a SIMD/JIT implementation is not available.

| Language | Representative source | Code L/C | Edit vs baseline L/C | Checked M/s | Unchecked M/s | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Mech | same `.mec`; Cranelift SIMD/JIT backend | 42 / 1,513 | 0 / 0 | 41.496 | 49.787 | measured |
| Julia | explicit SIMD.jl lanes | 196 / 6,935 | 204 / 8,960 | 30.835 | 32.260 | measured |
| Futhark | strict scalarized ISPC program, one worker | 105 / 3,566 | 112 / 4,266 | 30.576 | 43.337 | measured |
| Rust | packed four-lane SIMD | 566 / 13,848 | 708 / 24,453 | 25.650 | 18.627 | measured |
| NumPy | batched fixed-shape arrays | 108 / 2,974 | 94 / 3,123 | 11.129 | 12.558 | measured |
| Halide | fixed-shape pipeline, one host worker | 324 / 8,928 | 0 / 0 | 2.707 | 5.058 | measured |
| LuaJIT | flat scalarized FFI state | 153 / 7,031 | 242 / 9,393 | 1.263 | 15.991 | measured |
| Lua | flat scalarized Lua state (PUC Lua advanced) | 196 / 5,884 | 181 / 7,203 | 0.584 | 0.879 | measured |

`partial` means the source exists but one checked/unchecked measurement is not recorded yet; it is not treated as zero.

## Missing backends and untested controls

- **Python**: backend/strategy unavailable.
- **Taichi**: backend/strategy unavailable.
