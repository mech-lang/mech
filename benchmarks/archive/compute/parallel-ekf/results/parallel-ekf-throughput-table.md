# Parallel EKF throughput table

This table is generated from the same retained evidence and row set as the SVG charts. Each contract is ranked independently from slowest to fastest; checked and unchecked values are never mixed in one rank.

Workloads: CPU/language 10,000 filters x 20 turns; Mech backend 100,000 filters x 5 CPU turns; matched runtime/native controls 500,000 filters x 40 turns. Setup, compilation, allocation, warmup, and final readback are outside the timed region.

## Checked (slowest to fastest)

| Rank | Runtime/lane | Family | Million EKF turns/s |
| ---: | --- | --- | ---: |
| 1 | Lua fixed-shape flat checked | Lua | 0.565 |
| 2 | Mech scalar, checked | Mech | 0.919 |
| 3 | LuaJIT fixed-shape flat, checked | LuaJIT | 1.263 |
| 4 | Halide, checked | Halide | 2.707 |
| 5 | Julia generic, checked | Julia | 3.073 |
| 6 | Halide JIT SIMD 8 workers, checked | Halide | 3.270 |
| 7 | Mech SIMD | Mech | 4.161 |
| 8 | NumPy vectorized fixed-shape, checked | NumPy | 10.673 |
| 9 | Mech Cranelift JIT | Mech | 16.606 |
| 10 | Julia fixed-shape, checked | Julia | 18.670 |
| 11 | Mech Cranelift JIT checked fast | Mech | 18.744 |
| 12 | Rust packed SIMD, checked | Rust | 25.650 |
| 13 | Julia fixed-shape SIMD, checked | Julia | 28.094 |
| 14 | Julia SIMD.jl intrinsics, checked | Julia | 30.835 |
| 15 | Mech Cranelift SIMD-JIT | Mech | 34.551 |
| 16 | Mech Cranelift SIMD-JIT checked fast | Mech | 36.654 |
| 17 | Futhark, checked | Futhark | 48.391 |
| 18 | Futhark ISPC SIMD 8 workers, checked | Futhark | 49.164 |
| 19 | Mech GPU, checked repeated | Mech | 56.279 |
| 20 | Mech Cranelift SIMD-JIT parallel | Mech | 57.950 |
| 21 | Mech GPU, checked one-turn | Mech | 62.798 |
| 22 | NumPy/Numba parallel JIT, 8 workers | NumPy | 77.225 |
| 23 | Taichi LLVM CPU, 8 workers | Taichi | 86.047 |
| 24 | Mech SIMD/JIT CPU, 8 workers | Mech | 104.783 |
| 25 | Julia SIMD.jl, 8 workers | Julia | 106.341 |
| 26 | Mech GPU, WGPU per-turn | Mech | 152.972 |
| 27 | Taichi optimized native Metal, checked | Taichi | 168.798 |
| 28 | Taichi GPU, native Metal | Taichi | 176.710 |
| 29 | Julia GPU, native Metal | Julia | 197.078 |
| 30 | Mech GPU, native Metal | Mech | 246.151 |

## Unchecked (slowest to fastest)

| Rank | Runtime/lane | Family | Million EKF turns/s |
| ---: | --- | --- | ---: |
| 1 | NumPy scalar outer loop, unchecked | NumPy | 0.055 |
| 2 | Lua fixed-shape flat unchecked | Lua | 0.835 |
| 3 | Mech scalar, unchecked | Mech | 1.029 |
| 4 | LuaJIT scalar outer loop, unchecked | LuaJIT | 1.074 |
| 5 | Julia generic, unchecked | Julia | 3.123 |
| 6 | Halide, unchecked | Halide | 5.058 |
| 7 | Halide JIT SIMD 8 workers, unchecked | Halide | 5.593 |
| 8 | NumPy vectorized fixed-shape, unchecked | NumPy | 12.032 |
| 9 | LuaJIT fixed-shape flat, unchecked | LuaJIT | 15.991 |
| 10 | Rust optimized fixed-shape, unchecked | Rust | 16.907 |
| 11 | Mech Cranelift JIT unchecked | Mech | 18.191 |
| 12 | Rust packed SIMD, unchecked | Rust | 18.627 |
| 13 | Mech Cranelift JIT unchecked fast | Mech | 20.154 |
| 14 | Julia fixed-shape, unchecked | Julia | 21.937 |
| 15 | Julia SIMD.jl intrinsics, unchecked | Julia | 32.260 |
| 16 | Julia fixed-shape SIMD, unchecked | Julia | 35.100 |
| 17 | Mech Cranelift SIMD-JIT unchecked | Mech | 38.226 |
| 18 | Mech Cranelift SIMD-JIT unchecked fast | Mech | 40.860 |
| 19 | Futhark, unchecked | Futhark | 47.824 |
| 20 | Mech GPU, unchecked one-turn | Mech | 51.801 |
| 21 | Mech GPU, unchecked ping-pong one-turn | Mech | 51.801 |
| 22 | Futhark ISPC SIMD 8 workers, unchecked | Futhark | 53.648 |
| 23 | Mech GPU, unchecked in-place one-turn | Mech | 54.635 |
| 24 | Mech Cranelift SIMD-JIT parallel unchecked fast | Mech | 61.823 |
| 25 | NumPy/Numba parallel JIT, 8 workers | NumPy | 77.130 |
| 26 | Taichi LLVM CPU, 8 workers | Taichi | 98.140 |
| 27 | Julia SIMD.jl, 8 workers | Julia | 109.628 |
| 28 | Mech SIMD/JIT CPU, 8 workers | Mech | 110.469 |
| 29 | Mech GPU, WGPU per-turn | Mech | 157.141 |
| 30 | Taichi GPU, native Metal | Taichi | 194.793 |
| 31 | Julia GPU, native Metal | Julia | 216.462 |
| 32 | Taichi optimized native Metal, unchecked | Taichi | 217.297 |
| 33 | Mech GPU, native Metal | Mech | 241.028 |
| 34 | Mech GPU, unchecked repeated | Mech | 350.930 |
| 35 | Mech GPU, unchecked in-place repeated | Mech | 433.892 |
| 36 | Mech GPU, unchecked one-submit | Mech | 3729.673 |

Checked rows include candidate validation/publication. Unchecked rows explicitly omit those guarantees. The GPU one-submit row is a fused unchecked control and is therefore shown only in the unchecked section.
Futhark GPU has no numeric row on this Apple M1: Futhark 0.27 exposes CUDA/OpenCL backends but no Metal backend; the generated OpenCL kernel is rejected by Apple's driver.
NumPy GPU has no numeric row on this Apple M1: plain NumPy has no GPU backend and CuPy requires CUDA/NVIDIA. The capability result is retained separately.
