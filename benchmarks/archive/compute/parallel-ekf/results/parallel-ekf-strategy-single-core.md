# Parallel EKF: Single-core

One process and one host worker; explicit SIMD/JIT controls are used where the retained evidence provides them. Workload: **10,000 filters x 20 turns where available**. Checked and unchecked are separate columns; source edits are measured against each language's baseline source.

**Scope note:** The closest Futhark comparison is its one-worker row (19.614 checked / 19.635 unchecked). The 48.391 / 47.824 Futhark result uses eight workers and belongs to the multicore view.

| Language | Representative source | Code L/C | Edit vs baseline L/C | Checked M/s | Unchecked M/s | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Mech | same `.mec`; Cranelift SIMD/JIT backend | 42 / 1,513 | 0 / 0 | 34.521 | 38.226 | measured |
| Rust | packed four-lane SIMD | 566 / 13,848 | 708 / 24,453 | 25.650 | 18.627 | measured |
| NumPy | batched fixed-shape arrays | 108 / 2,974 | 94 / 3,123 | 11.129 | 12.558 | measured |
| Julia | explicit SIMD.jl lanes | 196 / 6,935 | 204 / 8,960 | 30.835 | 32.260 | measured |
| LuaJIT | flat scalarized FFI state | 153 / 7,031 | 242 / 9,393 | 1.263 | 15.991 | measured |
| Lua | flat scalarized Lua state | 153 / 7,031 | 0 / 0 | 0.565 | 0.835 | measured |
| Halide | fixed-shape pipeline, one host worker | 324 / 8,928 | 0 / 0 | 2.707 | 5.058 | measured |
| Futhark | same data-parallel program, one worker | 56 / 3,098 | 0 / 0 | 19.614 | 19.635 | measured |

`partial` means the source exists but one checked/unchecked measurement is not recorded yet; it is not treated as zero.

## Missing backends and untested controls

- **Python**: backend/strategy unavailable.
- **Taichi**: backend/strategy unavailable.
