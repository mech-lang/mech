# Parallel EKF backend and scalar-language comparison

This benchmark produces two distinct comparisons from the high-level EKF in
`../../fixtures/ekf-kernel.mec`.

## Mech physical backends

The scalar CPU, four-lane SIMD CPU, and GPU lanes execute the same compiler
artifact and persistent per-filter state. The SIMD implementation changes
only the physical value type of the scalarized instruction stream to
`wide::f32x4`; it uses NEON on Apple Silicon and SSE where available. The
primary GPU lane submits and synchronizes one Mech turn at a time. The batched
GPU lane is reported separately because it records 120 dependent turns in one
submission and therefore amortizes synchronization.

Apple M1 median of three processes after one discarded process warmup,
100,000 filters, 2026-08-13:

| Mech backend | Million EKF-turns/s | Scalar speedup |
| --- | ---: | ---: |
| Mech scalar | 1.222 | 1.00x |
| Mech SIMD (`4xf32`) | 4.416 | 3.61x |
| Mech GPU, one submission/turn | 61.060 | 49.97x |
| Mech GPU, 120 turns/submission | 329.650 | 269.76x |

Parsing, artifact compilation, input construction, allocation, GPU setup,
warmup, final readback, and correctness checks are outside the timed regions.

## Scalar outer-loop languages

Every lane owns 10,000 persistent filters and executes one filter at a time
for five warmup turns, a state reset, and 20 measured turns. Inputs, equations,
`f32` state, Joseph covariance update, and checksums agree. "Scalar" here means
the outer filter loop is sequential. It does not claim that a language's
scalar math or small matrix library avoids every SIMD instruction internally.

Apple M1 median of three processes after one discarded process warmup, except
five for bimodal LuaJIT samples:

| Scalar outer-loop lane | Million EKF-turns/s | Relative to Mech scalar |
| --- | ---: | ---: |
| Mech scalar artifact evaluator | 1.221 | 1.00x |
| Rust fixed-shape, no-inline filter | 12.947 | 10.60x |
| NumPy sequential small matrices | 0.055 | 0.04x |
| Julia sequential small matrices | 2.835 | 2.32x |
| LuaJIT sequential FFI `f32` state | 1.921 | 1.57x |

Versions were Rust `1.96.0-nightly`, Python `3.14.6`, NumPy `2.5.2`, Julia
`1.12.6`, and LuaJIT `2.1.1785763465`. NumPy, Julia BLAS, and related native
thread counts were pinned to one.

Build the native Mech benchmark, then run the complete comparison:

```text
cargo build -p mech-gpu --release --features native --example parallel_ekf_benchmark
python3 hosts/gpu/benchmarks/parallel-ekf/run.py --python /path/to/python-with-numpy
```

The runner compiles the checked-in Rust control directly with
`rustc -C opt-level=3 -C target-cpu=native`, validates every scalar checksum,
and prints both Markdown tables.
