# Parallel EKF throughput table

This table is generated from the same retained evidence and row set as the SVG charts. Each contract is ranked independently from fastest to slowest; checked and unchecked values are never mixed in one rank.

Workloads: CPU/language 10,000 filters x 20 turns; Mech backend 100,000 filters x 5 CPU turns; matched runtime/native controls 500,000 filters x 40 turns. Setup, compilation, allocation, warmup, and final readback are outside the timed region.

## Checked (fastest to slowest)

| Rank | Runtime/lane | Family | Million EKF turns/s |
| ---: | --- | --- | ---: |
| 1 | Mech GPU, native Metal | Mech | 187.999 |
| 2 | Julia GPU, native Metal (strict retained-state) | Julia | 178.135 |
| 3 | Taichi GPU, native Metal | Taichi | 176.710 |
| 4 | Taichi optimized native Metal, checked | Taichi | 168.798 |
| 5 | Mech GPU, WGPU per-turn | Mech | 152.972 |
| 6 | Rust packed SIMD, fused worker-local block strict checked (8 workers) | Rust | 146.509 |
| 7 | Mech Cranelift SIMD-JIT, checkpointed fused block checked (8 workers) | Mech | 145.573 |
| 8 | Julia SIMD.jl, fused worker-local block checked (8 workers) | Julia | 128.544 |
| 9 | Halide GPU, native Metal (strict fault-observing) | Halide | 111.474 |
| 10 | Julia SIMD.jl, 8 workers | Julia | 106.341 |
| 11 | Mech SIMD/JIT CPU, 8 workers | Mech | 104.783 |
| 12 | Taichi LLVM CPU, 8 workers | Taichi | 86.047 |
| 13 | NumPy/Numba, fused worker-local block checked (8 workers) | NumPy | 80.323 |
| 14 | NumPy/Numba parallel JIT, 8 workers | NumPy | 77.225 |
| 15 | Mech GPU, checked one-turn | Mech | 62.798 |
| 16 | Mech Cranelift SIMD-JIT parallel | Mech | 57.950 |
| 17 | Mech GPU, checked repeated | Mech | 56.279 |
| 18 | Futhark multicore, 8 workers (10k x 20) | Futhark | 48.391 |
| 19 | Mech Cranelift SIMD-JIT | Mech | 41.496 |
| 20 | Julia SIMD.jl intrinsics, checked | Julia | 30.835 |
| 21 | Futhark ISPC scalarized SIMD, 1 worker (10k x 20) | Futhark | 30.576 |
| 22 | Julia fixed-shape SIMD, checked | Julia | 28.094 |
| 23 | Rust packed SIMD, checked | Rust | 25.650 |
| 24 | Rust optimized fixed-shape, checked | Rust | 20.011 |
| 25 | Futhark multicore, 1 worker (10k x 20) | Futhark | 19.614 |
| 26 | Julia fixed-shape, checked | Julia | 18.670 |
| 27 | Mech Cranelift JIT | Mech | 16.606 |
| 28 | NumPy vectorized fixed-shape, checked | NumPy | 10.673 |
| 29 | Mech SIMD | Mech | 4.161 |
| 30 | Halide JIT SIMD 8 workers, checked | Halide | 3.270 |
| 31 | Julia generic, checked | Julia | 3.073 |
| 32 | Halide | Halide | 2.707 |
| 33 | LuaJIT fixed-shape flat, checked | LuaJIT | 1.263 |
| 34 | Mech scalar, checked | Mech | 0.919 |
| 35 | Lua PUC advanced | Lua | 0.584 |
| 36 | Lua PUC baseline | Lua | 0.564 |
| 37 | Python pure scalar | Python | 0.246 |

## Unchecked (fastest to slowest)

| Rank | Runtime/lane | Family | Million EKF turns/s |
| ---: | --- | --- | ---: |
| 1 | Mech GPU, unchecked one-submit | Mech | 3729.673 |
| 2 | Mech GPU, unchecked in-place repeated | Mech | 433.892 |
| 3 | Mech GPU, unchecked repeated | Mech | 350.930 |
| 4 | Mech GPU, native Metal | Mech | 275.534 |
| 5 | Taichi optimized native Metal, unchecked | Taichi | 217.297 |
| 6 | Halide GPU, native Metal (strict fault-observing) | Halide | 212.283 |
| 7 | Julia GPU, native Metal (strict retained-state) | Julia | 199.454 |
| 8 | Taichi GPU, native Metal | Taichi | 194.793 |
| 9 | Mech SIMD/JIT CPU, fused unchecked block (8 workers) | Mech | 165.830 |
| 10 | Rust packed SIMD, fused worker-local block (8 workers) | Rust | 163.866 |
| 11 | Mech GPU, WGPU per-turn | Mech | 157.141 |
| 12 | Julia SIMD.jl, fused worker-local block (8 workers) | Julia | 133.605 |
| 13 | Mech SIMD/JIT CPU, 8 workers | Mech | 110.469 |
| 14 | Julia SIMD.jl, 8 workers | Julia | 109.628 |
| 15 | Taichi LLVM CPU, 8 workers | Taichi | 98.140 |
| 16 | NumPy/Numba, fused worker-local block (8 workers) | NumPy | 81.972 |
| 17 | NumPy/Numba parallel JIT, 8 workers | NumPy | 77.130 |
| 18 | Mech GPU, unchecked in-place one-turn | Mech | 54.635 |
| 19 | Mech GPU, unchecked one-turn | Mech | 51.801 |
| 20 | Mech GPU, unchecked ping-pong one-turn | Mech | 51.801 |
| 21 | Mech Cranelift SIMD-JIT unchecked | Mech | 49.787 |
| 22 | Futhark multicore, 8 workers (10k x 20) | Futhark | 47.824 |
| 23 | Futhark ISPC scalarized SIMD, 1 worker (10k x 20) | Futhark | 43.337 |
| 24 | Julia fixed-shape SIMD, unchecked | Julia | 35.100 |
| 25 | Julia SIMD.jl intrinsics, unchecked | Julia | 32.260 |
| 26 | Rust optimized fixed-shape, unchecked | Rust | 24.713 |
| 27 | Julia fixed-shape, unchecked | Julia | 21.937 |
| 28 | Futhark multicore, 1 worker (10k x 20) | Futhark | 19.635 |
| 29 | Rust packed SIMD, unchecked | Rust | 18.627 |
| 30 | Mech Cranelift JIT unchecked | Mech | 18.191 |
| 31 | LuaJIT fixed-shape flat, unchecked | LuaJIT | 15.991 |
| 32 | NumPy vectorized fixed-shape, unchecked | NumPy | 12.032 |
| 33 | Halide JIT SIMD 8 workers, unchecked | Halide | 5.593 |
| 34 | Halide | Halide | 5.058 |
| 35 | Julia generic, unchecked | Julia | 3.123 |
| 36 | LuaJIT scalar outer loop, unchecked | LuaJIT | 1.074 |
| 37 | Mech scalar, unchecked | Mech | 1.029 |
| 38 | Lua PUC advanced | Lua | 0.879 |
| 39 | Lua PUC baseline | Lua | 0.836 |
| 40 | Python pure scalar | Python | 0.356 |
| 41 | NumPy scalar outer loop, unchecked | NumPy | 0.055 |

Checked rows include candidate validation/publication. Unchecked rows explicitly omit those guarantees. The GPU one-submit row is a fused unchecked control and is therefore shown only in the unchecked section.
Futhark GPU has no numeric row on this Apple M1: Futhark 0.27 exposes CUDA/OpenCL backends but no Metal backend; the generated OpenCL kernel is rejected by Apple's driver.
Futhark's eight-worker fixed-mode row is omitted here because no FMA-disabled evidence is retained for that workload; the strict one-worker scalarized control is shown above.
NumPy GPU has no numeric row on this Apple M1: plain NumPy has no GPU backend and CuPy requires CUDA/NVIDIA. The capability result is retained separately.
