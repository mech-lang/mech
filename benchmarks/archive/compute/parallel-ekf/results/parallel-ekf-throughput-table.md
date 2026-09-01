# Parallel EKF throughput table

This table is generated from the same retained evidence and row set as the SVG charts. Each contract is ranked independently from slowest to fastest; checked and unchecked values are never mixed in one rank.

Workloads: CPU/language 10,000 filters x 20 turns; Mech backend 100,000 filters x 5 CPU turns; matched runtime/native controls 500,000 filters x 40 turns. Setup, compilation, allocation, warmup, and final readback are outside the timed region.

## Checked (slowest to fastest)

| Rank | Runtime/lane | Family | Million EKF turns/s |
| ---: | --- | --- | ---: |
| 1 | Python pure scalar | Python | 0.246 |
| 2 | Lua fixed-shape flat checked | Lua | 0.565 |
| 3 | Mech scalar, checked | Mech | 0.919 |
| 4 | LuaJIT fixed-shape flat, checked | LuaJIT | 1.263 |
| 5 | Halide | Halide | 2.707 |
| 6 | Julia generic, checked | Julia | 3.073 |
| 7 | Halide JIT SIMD 8 workers, checked | Halide | 3.270 |
| 8 | Mech SIMD | Mech | 4.161 |
| 9 | NumPy vectorized fixed-shape, checked | NumPy | 10.673 |
| 10 | Mech Cranelift JIT | Mech | 16.606 |
| 11 | Julia fixed-shape, checked | Julia | 18.670 |
| 12 | Mech Cranelift JIT checked fast | Mech | 18.744 |
| 13 | Futhark multicore, 1 worker (10k x 20) | Futhark | 19.614 |
| 14 | Rust packed SIMD, checked | Rust | 25.650 |
| 15 | Julia fixed-shape SIMD, checked | Julia | 28.094 |
| 16 | Julia SIMD.jl intrinsics, checked | Julia | 30.835 |
| 17 | Mech Cranelift SIMD-JIT | Mech | 34.551 |
| 18 | Mech Cranelift SIMD-JIT checked fast | Mech | 36.654 |
| 19 | Futhark ISPC scalarized SIMD, 1 worker (10k x 20) | Futhark | 37.608 |
| 20 | Futhark multicore, 8 workers (10k x 20) | Futhark | 48.391 |
| 21 | Mech GPU, checked repeated | Mech | 56.279 |
| 22 | Mech Cranelift SIMD-JIT parallel | Mech | 57.950 |
| 23 | Mech GPU, checked one-turn | Mech | 62.798 |
| 24 | NumPy/Numba parallel JIT, 8 workers | NumPy | 77.225 |
| 25 | NumPy/Numba, fused worker-local block checked (8 workers) | NumPy | 80.323 |
| 26 | Taichi LLVM CPU, 8 workers | Taichi | 86.047 |
| 27 | Mech SIMD/JIT CPU, 8 workers | Mech | 104.783 |
| 28 | Julia SIMD.jl, 8 workers | Julia | 106.341 |
| 29 | Futhark ISPC fixed-mode, 8 workers (500k x 40) | Futhark | 108.718 |
| 30 | Halide GPU, native Metal (strict fault-observing) | Halide | 111.474 |
| 31 | Julia SIMD.jl, fused worker-local block checked (8 workers) | Julia | 128.544 |
| 32 | Mech Cranelift SIMD-JIT, checkpointed fused block checked (8 workers) | Mech | 145.573 |
| 33 | Rust packed SIMD, fused worker-local block strict checked (8 workers) | Rust | 146.509 |
| 34 | Mech GPU, WGPU per-turn | Mech | 152.972 |
| 35 | Taichi optimized native Metal, checked | Taichi | 168.798 |
| 36 | Taichi GPU, native Metal | Taichi | 176.710 |
| 37 | Julia GPU, native Metal (strict retained-state) | Julia | 178.135 |
| 38 | Mech GPU, native Metal | Mech | 187.999 |

## Unchecked (slowest to fastest)

| Rank | Runtime/lane | Family | Million EKF turns/s |
| ---: | --- | --- | ---: |
| 1 | NumPy scalar outer loop, unchecked | NumPy | 0.055 |
| 2 | Python pure scalar | Python | 0.356 |
| 3 | Lua fixed-shape flat unchecked | Lua | 0.835 |
| 4 | Mech scalar, unchecked | Mech | 1.029 |
| 5 | LuaJIT scalar outer loop, unchecked | LuaJIT | 1.074 |
| 6 | Julia generic, unchecked | Julia | 3.123 |
| 7 | Halide | Halide | 5.058 |
| 8 | Halide JIT SIMD 8 workers, unchecked | Halide | 5.593 |
| 9 | NumPy vectorized fixed-shape, unchecked | NumPy | 12.032 |
| 10 | LuaJIT fixed-shape flat, unchecked | LuaJIT | 15.991 |
| 11 | Rust optimized fixed-shape, unchecked | Rust | 16.907 |
| 12 | Mech Cranelift JIT unchecked | Mech | 18.191 |
| 13 | Rust packed SIMD, unchecked | Rust | 18.627 |
| 14 | Futhark multicore, 1 worker (10k x 20) | Futhark | 19.635 |
| 15 | Mech Cranelift JIT unchecked fast | Mech | 20.154 |
| 16 | Julia fixed-shape, unchecked | Julia | 21.937 |
| 17 | Julia SIMD.jl intrinsics, unchecked | Julia | 32.260 |
| 18 | Julia fixed-shape SIMD, unchecked | Julia | 35.100 |
| 19 | Mech Cranelift SIMD-JIT unchecked | Mech | 38.226 |
| 20 | Mech Cranelift SIMD-JIT unchecked fast | Mech | 40.860 |
| 21 | Futhark multicore, 8 workers (10k x 20) | Futhark | 47.824 |
| 22 | Mech GPU, unchecked one-turn | Mech | 51.801 |
| 23 | Mech GPU, unchecked ping-pong one-turn | Mech | 51.801 |
| 24 | Futhark ISPC scalarized SIMD, 1 worker (10k x 20) | Futhark | 52.673 |
| 25 | Mech GPU, unchecked in-place one-turn | Mech | 54.635 |
| 26 | Mech Cranelift SIMD-JIT parallel unchecked fast | Mech | 61.823 |
| 27 | NumPy/Numba parallel JIT, 8 workers | NumPy | 77.130 |
| 28 | NumPy/Numba, fused worker-local block (8 workers) | NumPy | 81.972 |
| 29 | NumPy/Numba, fused worker-local block fast math (8 workers) | NumPy | 86.973 |
| 30 | Taichi LLVM CPU, 8 workers | Taichi | 98.140 |
| 31 | Julia SIMD.jl, 8 workers | Julia | 109.628 |
| 32 | Mech SIMD/JIT CPU, 8 workers | Mech | 110.469 |
| 33 | Julia SIMD.jl, fused worker-local block (8 workers) | Julia | 133.605 |
| 34 | Mech SIMD/JIT CPU, persistent pool per-turn (8 workers) | Mech | 150.395 |
| 35 | Futhark ISPC fixed-mode, 8 workers (500k x 40) | Futhark | 152.330 |
| 36 | Mech GPU, WGPU per-turn | Mech | 157.141 |
| 37 | Rust packed SIMD, fused worker-local block (8 workers) | Rust | 163.866 |
| 38 | Mech SIMD/JIT CPU, fused unchecked block (8 workers) | Mech | 165.830 |
| 39 | Taichi GPU, native Metal | Taichi | 194.793 |
| 40 | Julia GPU, native Metal (strict retained-state) | Julia | 199.454 |
| 41 | Halide GPU, native Metal (strict fault-observing) | Halide | 212.283 |
| 42 | Taichi optimized native Metal, unchecked | Taichi | 217.297 |
| 43 | Mech GPU, native Metal | Mech | 275.534 |
| 44 | Mech GPU, unchecked repeated | Mech | 350.930 |
| 45 | Mech GPU, unchecked in-place repeated | Mech | 433.892 |
| 46 | Mech GPU, unchecked one-submit | Mech | 3729.673 |

Checked rows include candidate validation/publication. Unchecked rows explicitly omit those guarantees. The GPU one-submit row is a fused unchecked control and is therefore shown only in the unchecked section.
Futhark GPU has no numeric row on this Apple M1: Futhark 0.27 exposes CUDA/OpenCL backends but no Metal backend; the generated OpenCL kernel is rejected by Apple's driver.
NumPy GPU has no numeric row on this Apple M1: plain NumPy has no GPU backend and CuPy requires CUDA/NVIDIA. The capability result is retained separately.
