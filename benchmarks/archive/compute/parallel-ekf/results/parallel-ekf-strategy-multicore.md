# Parallel EKF: Eight-worker multicore

Matched eight-worker CPU fused block; checked mode validates each candidate and publishes at the block boundary. Workload: **500,000 filters x 40 turns where available**. Checked and unchecked are separate columns; source edits are measured against each language's baseline source.

| Language | Representative source | Code L/C | Edit vs baseline L/C | Checked M/s | Unchecked M/s | Result |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| Mech | same `.mec`; checkpointed fused eight-worker SIMD/JIT block | 42 / 1,513 | 0 / 0 | 145.573 | 165.830 | measured |
| Rust | packed SIMD with eight worker-local blocks | 566 / 13,848 | 708 / 24,453 | 146.509 | 163.866 | measured |
| NumPy | Numba `prange` eight-worker loop | 217 / 7,871 | 244 / 11,087 | 80.323 | 81.972 | measured |
| Julia | Threads.@threads static publication | 238 / 8,371 | 252 / 11,429 | 106.341 | 109.628 | measured |
| Taichi | scalar SoA fields with eight CPU workers | 277 / 11,406 | 224 / 11,640 | 86.047 | 98.140 | measured |
| Halide | parallel/vectorized pipeline with eight workers | 324 / 8,928 | 0 / 0 | 3.270 | 5.593 | measured |
| Futhark | ISPC fixed-mode with eight workers | 56 / 3,098 | 0 / 0 | 108.718 | 152.330 | measured |

`partial` means the source exists but one checked/unchecked measurement is not recorded yet; it is not treated as zero.

## Missing backends and untested controls

- **Python**: backend/strategy unavailable.
- **LuaJIT**: backend/strategy unavailable.
- **Lua**: backend/strategy unavailable.
