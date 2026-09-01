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
| 6 | Mech SIMD | Mech | 4.161 |
| 7 | NumPy vectorized fixed-shape, checked | NumPy | 10.673 |
| 8 | Mech Cranelift JIT | Mech | 16.606 |
| 9 | Julia fixed-shape, checked | Julia | 18.670 |
| 10 | Mech Cranelift JIT checked fast | Mech | 18.744 |
| 11 | Rust packed SIMD, checked | Rust | 25.650 |
| 12 | Julia fixed-shape SIMD, checked | Julia | 28.094 |
| 13 | Julia SIMD.jl intrinsics, checked | Julia | 30.835 |
| 14 | Mech Cranelift SIMD-JIT | Mech | 34.551 |
| 15 | Mech Cranelift SIMD-JIT checked fast | Mech | 36.654 |
| 16 | Futhark, checked | Futhark | 48.391 |
| 17 | Mech GPU, checked repeated | Mech | 56.279 |
| 18 | Mech Cranelift SIMD-JIT parallel | Mech | 57.950 |
| 19 | Mech GPU, checked one-turn | Mech | 62.798 |
| 20 | Taichi LLVM CPU, 8 workers | Taichi | 86.047 |
| 21 | Mech SIMD/JIT CPU, 8 workers | Mech | 104.783 |
| 22 | Julia SIMD.jl, 8 workers | Julia | 106.341 |
| 23 | Mech GPU, WGPU per-turn | Mech | 152.972 |
| 24 | Taichi optimized native Metal, checked | Taichi | 168.798 |
| 25 | Taichi GPU, native Metal | Taichi | 176.710 |
| 26 | Mech GPU, native Metal | Mech | 246.151 |

## Unchecked (slowest to fastest)

| Rank | Runtime/lane | Family | Million EKF turns/s |
| ---: | --- | --- | ---: |
| 1 | NumPy scalar outer loop, unchecked | NumPy | 0.055 |
| 2 | Lua fixed-shape flat unchecked | Lua | 0.835 |
| 3 | Mech scalar, unchecked | Mech | 1.029 |
| 4 | LuaJIT scalar outer loop, unchecked | LuaJIT | 1.074 |
| 5 | Julia generic, unchecked | Julia | 3.123 |
| 6 | Halide, unchecked | Halide | 5.058 |
| 7 | NumPy vectorized fixed-shape, unchecked | NumPy | 12.032 |
| 8 | LuaJIT fixed-shape flat, unchecked | LuaJIT | 15.991 |
| 9 | Rust optimized fixed-shape, unchecked | Rust | 16.907 |
| 10 | Mech Cranelift JIT unchecked | Mech | 18.191 |
| 11 | Rust packed SIMD, unchecked | Rust | 18.627 |
| 12 | Mech Cranelift JIT unchecked fast | Mech | 20.154 |
| 13 | Julia fixed-shape, unchecked | Julia | 21.937 |
| 14 | Julia SIMD.jl intrinsics, unchecked | Julia | 32.260 |
| 15 | Julia fixed-shape SIMD, unchecked | Julia | 35.100 |
| 16 | Mech Cranelift SIMD-JIT unchecked | Mech | 38.226 |
| 17 | Mech Cranelift SIMD-JIT unchecked fast | Mech | 40.860 |
| 18 | Futhark, unchecked | Futhark | 47.824 |
| 19 | Mech GPU, unchecked one-turn | Mech | 51.801 |
| 20 | Mech GPU, unchecked ping-pong one-turn | Mech | 51.801 |
| 21 | Mech GPU, unchecked in-place one-turn | Mech | 54.635 |
| 22 | Mech Cranelift SIMD-JIT parallel unchecked fast | Mech | 61.823 |
| 23 | Taichi LLVM CPU, 8 workers | Taichi | 98.140 |
| 24 | Julia SIMD.jl, 8 workers | Julia | 109.628 |
| 25 | Mech SIMD/JIT CPU, 8 workers | Mech | 110.469 |
| 26 | Mech GPU, WGPU per-turn | Mech | 157.141 |
| 27 | Taichi GPU, native Metal | Taichi | 194.793 |
| 28 | Taichi optimized native Metal, unchecked | Taichi | 217.297 |
| 29 | Mech GPU, native Metal | Mech | 241.028 |
| 30 | Mech GPU, unchecked repeated | Mech | 350.930 |
| 31 | Mech GPU, unchecked in-place repeated | Mech | 433.892 |
| 32 | Mech GPU, unchecked one-submit | Mech | 3729.673 |

Checked rows include candidate validation/publication. Unchecked rows explicitly omit those guarantees. The GPU one-submit row is a fused unchecked control and is therefore shown only in the unchecked section.
Futhark GPU has no numeric row on this Apple M1: Futhark 0.27 exposes CUDA/OpenCL backends but no Metal backend; the generated OpenCL kernel is rejected by Apple's driver.
