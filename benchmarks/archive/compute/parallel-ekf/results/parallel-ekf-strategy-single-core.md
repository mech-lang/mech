# Parallel EKF: Single-core

One process and one host worker; explicit SIMD/JIT controls are used where the retained evidence provides them. Workload: **10,000 filters x 20 turns where available**. Checked and unchecked are separate columns; source edits are measured against each language's baseline source.

| Language | Representative source | Code L/C | Edit vs baseline L/C | Checked M/s | Unchecked M/s | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Mech | same `.mec`; Cranelift SIMD/JIT backend | 42 / 1,513 | 0 / 0 | 36.654 | 40.860 | measured |
| Rust | packed four-lane SIMD | 566 / 13,848 | 701 / 23,858 | 25.650 | 18.627 | measured |
| NumPy | batched fixed-shape arrays | 108 / 2,974 | 94 / 3,123 | 11.129 | 12.558 | measured |
| Python | not applicable: no optimized source | N/A | N/A | N/A | N/A | N/A: no implementation |
| Julia | explicit SIMD.jl lanes | 196 / 6,935 | 204 / 8,960 | 30.835 | 32.260 | measured |
| LuaJIT | flat scalarized FFI state | 153 / 7,031 | 224 / 9,071 | 1.263 | 15.991 | measured |
| Lua | flat scalarized Lua state | 153 / 7,031 | 0 / 0 | 0.565 | 0.835 | measured |
| Taichi | not applicable: no single-core row | N/A | N/A | N/A | N/A | N/A: no implementation |
| Halide | fixed-shape pipeline, one host worker | 324 / 8,928 | 0 / 0 | 2.707 | 5.058 | measured |
| Futhark | same data-parallel program, one worker | 56 / 3,098 | 0 / 0 | 19.614 | 19.635 | measured |

`N/A` means the language/backend does not provide this strategy in the retained comparison. `partial` means the source exists but one checked/unchecked measurement is not recorded yet; it is not treated as zero.
