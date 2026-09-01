# Parallel EKF: Compiled baseline

Direct native, JIT, or ahead-of-time compiled controls, with no interpreter in the timed loop. Workload: **10,000 filters x 20 turns where available**. Checked and unchecked are separate columns; source edits are measured against each language's baseline source.

**Scope note:** This view uses each language's retained native/JIT/AOT scalar control. Mech uses the paired scalar Cranelift JIT checked/unchecked measurements from the backend evidence; the single-core and multicore views remain separate SIMD/JIT controls.

| Language | Representative source | Code L/C | Edit vs baseline L/C | Checked M/s | Unchecked M/s | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Mech | scalar Cranelift JIT | 42 / 1,513 | 0 / 0 | 16.713 | 18.191 | measured |
| Rust | fixed-shape scalar arrays | 243 / 6,059 | 0 / 0 | 19.799 | 20.544 | measured |
| Julia | generic scalar JIT arrays | 126 / 4,322 | 0 / 0 | 3.073 | 3.123 | measured |
| LuaJIT | generic FFI JIT loop | 205 / 4,810 | 0 / 0 | 1.068 | 1.740 | measured |
| Taichi | Vector/Matrix resident fields with compiled kernel | 260 / 8,891 | 0 / 0 | 19.405 | 22.452 | measured |
| Halide | fixed-shape compiled pipeline | 324 / 8,928 | 0 / 0 | 2.707 | 5.058 | measured |
| Futhark | compiled data-parallel array program | 56 / 3,098 | 0 / 0 | 19.614 | 19.635 | measured |

`partial` means the source exists but one checked/unchecked measurement is not recorded yet; it is not treated as zero.
