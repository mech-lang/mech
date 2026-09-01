# Parallel EKF throughput table

This table is generated from the same retained evidence and row set as the SVG charts. Each contract is ranked independently from slowest to fastest; checked and unchecked values are never mixed in one rank.

Workloads: CPU/language 10,000 filters x 20 turns; Mech backend 100,000 filters x 5 CPU turns; matched runtime/native controls 500,000 filters x 40 turns. Setup, compilation, allocation, warmup, and final readback are outside the timed region.

## Checked (slowest to fastest)

| Rank | Runtime/lane | Family | Million EKF turns/s |
| ---: | --- | --- | ---: |
| 1 | Python pure scalar | Python | 0.246 |
| 2 | Lua PUC baseline | Lua | 0.564 |
| 3 | Lua PUC advanced | Lua | 0.584 |
| 4 | Mech scalar, checked | Mech | 0.919 |
| 5 | LuaJIT fixed-shape flat, checked | LuaJIT | 1.263 |
| 6 | Halide | Halide | 2.707 |
| 7 | Julia generic, checked | Julia | 3.073 |
| 8 | Halide JIT SIMD 8 workers, checked | Halide | 3.270 |
| 9 | Mech SIMD | Mech | 4.161 |
| 10 | NumPy vectorized fixed-shape, checked | NumPy | 10.673 |
| 11 | Mech Cranelift JIT | Mech | 16.606 |
| 12 | Julia fixed-shape, checked | Julia | 18.670 |
| 13 | Futhark multicore, 1 worker (10k x 20) | Futhark | 19.614 |
| 14 | Rust optimized fixed-shape, checked | Rust | 20.011 |
| 15 | Rust packed SIMD, checked | Rust | 25.650 |
| 16 | Julia fixed-shape SIMD, checked | Julia | 28.094 |
| 17 | Futhark ISPC scalarized SIMD, 1 worker (10k x 20) | Futhark | 30.576 |
| 18 | Julia SIMD.jl intrinsics, checked | Julia | 30.835 |
| 19 | Mech Cranelift SIMD-JIT | Mech | 41.496 |
| 20 | Futhark multicore, 8 workers (10k x 20) | Futhark | 48.391 |
| 21 | Mech GPU, checked repeated | Mech | 56.279 |
| 22 | Mech Cranelift SIMD-JIT parallel | Mech | 57.950 |
| 23 | Mech GPU, checked one-turn | Mech | 62.798 |
| 24 | NumPy/Numba parallel JIT, 8 workers | NumPy | 77.225 |
| 25 | NumPy/Numba, fused worker-local block checked (8 workers) | NumPy | 80.323 |
| 26 | Taichi LLVM CPU, 8 workers | Taichi | 86.047 |
| 27 | Mech SIMD/JIT CPU, 8 workers | Mech | 104.783 |
| 28 | Julia SIMD.jl, 8 workers | Julia | 106.341 |
| 29 | Halide GPU, native Metal (strict fault-observing) | Halide | 111.474 |
| 30 | Julia SIMD.jl, fused worker-local block checked (8 workers) | Julia | 128.544 |
| 31 | Mech Cranelift SIMD-JIT, checkpointed fused block checked (8 workers) | Mech | 145.573 |
| 32 | Rust packed SIMD, fused worker-local block strict checked (8 workers) | Rust | 146.509 |
| 33 | Mech GPU, WGPU per-turn | Mech | 152.972 |
| 34 | Taichi optimized native Metal, checked | Taichi | 168.798 |
| 35 | Taichi GPU, native Metal | Taichi | 176.710 |
| 36 | Julia GPU, native Metal (strict retained-state) | Julia | 178.135 |
| 37 | Mech GPU, native Metal | Mech | 187.999 |

## Unchecked (slowest to fastest)

| Rank | Runtime/lane | Family | Million EKF turns/s |
| ---: | --- | --- | ---: |
| 1 | NumPy scalar outer loop, unchecked | NumPy | 0.055 |
| 2 | Python pure scalar | Python | 0.356 |
| 3 | Lua PUC baseline | Lua | 0.836 |
| 4 | Lua PUC advanced | Lua | 0.879 |
| 5 | Mech scalar, unchecked | Mech | 1.029 |
| 6 | LuaJIT scalar outer loop, unchecked | LuaJIT | 1.074 |
| 7 | Julia generic, unchecked | Julia | 3.123 |
| 8 | Halide | Halide | 5.058 |
| 9 | Halide JIT SIMD 8 workers, unchecked | Halide | 5.593 |
| 10 | NumPy vectorized fixed-shape, unchecked | NumPy | 12.032 |
| 11 | LuaJIT fixed-shape flat, unchecked | LuaJIT | 15.991 |
| 12 | Mech Cranelift JIT unchecked | Mech | 18.191 |
| 13 | Rust packed SIMD, unchecked | Rust | 18.627 |
| 14 | Futhark multicore, 1 worker (10k x 20) | Futhark | 19.635 |
| 15 | Julia fixed-shape, unchecked | Julia | 21.937 |
| 16 | Rust optimized fixed-shape, unchecked | Rust | 24.713 |
| 17 | Julia SIMD.jl intrinsics, unchecked | Julia | 32.260 |
| 18 | Julia fixed-shape SIMD, unchecked | Julia | 35.100 |
| 19 | Futhark ISPC scalarized SIMD, 1 worker (10k x 20) | Futhark | 43.337 |
| 20 | Futhark multicore, 8 workers (10k x 20) | Futhark | 47.824 |
| 21 | Mech Cranelift SIMD-JIT unchecked | Mech | 49.787 |
| 22 | Mech GPU, unchecked one-turn | Mech | 51.801 |
| 23 | Mech GPU, unchecked ping-pong one-turn | Mech | 51.801 |
| 24 | Mech GPU, unchecked in-place one-turn | Mech | 54.635 |
| 25 | NumPy/Numba parallel JIT, 8 workers | NumPy | 77.130 |
| 26 | NumPy/Numba, fused worker-local block (8 workers) | NumPy | 81.972 |
| 27 | Taichi LLVM CPU, 8 workers | Taichi | 98.140 |
| 28 | Julia SIMD.jl, 8 workers | Julia | 109.628 |
| 29 | Mech SIMD/JIT CPU, 8 workers | Mech | 110.469 |
| 30 | Julia SIMD.jl, fused worker-local block (8 workers) | Julia | 133.605 |
| 31 | Mech GPU, WGPU per-turn | Mech | 157.141 |
| 32 | Rust packed SIMD, fused worker-local block (8 workers) | Rust | 163.866 |
| 33 | Mech SIMD/JIT CPU, fused unchecked block (8 workers) | Mech | 165.830 |
| 34 | Taichi GPU, native Metal | Taichi | 194.793 |
| 35 | Julia GPU, native Metal (strict retained-state) | Julia | 199.454 |
| 36 | Halide GPU, native Metal (strict fault-observing) | Halide | 212.283 |
| 37 | Taichi optimized native Metal, unchecked | Taichi | 217.297 |
| 38 | Mech GPU, native Metal | Mech | 275.534 |
| 39 | Mech GPU, unchecked repeated | Mech | 350.930 |
| 40 | Mech GPU, unchecked in-place repeated | Mech | 433.892 |
| 41 | Mech GPU, unchecked one-submit | Mech | 3729.673 |

Checked rows include candidate validation/publication. Unchecked rows explicitly omit those guarantees. The GPU one-submit row is a fused unchecked control and is therefore shown only in the unchecked section.
Futhark GPU has no numeric row on this Apple M1: Futhark 0.27 exposes CUDA/OpenCL backends but no Metal backend; the generated OpenCL kernel is rejected by Apple's driver.
Futhark's eight-worker fixed-mode row is omitted here because no FMA-disabled evidence is retained for that workload; the strict one-worker scalarized control is shown above.
NumPy GPU has no numeric row on this Apple M1: plain NumPy has no GPU backend and CuPy requires CUDA/NVIDIA. The capability result is retained separately.
