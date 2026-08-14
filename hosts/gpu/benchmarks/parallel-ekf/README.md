# Parallel EKF backend and scalar-language comparison

This benchmark produces two distinct comparisons from the high-level EKF in
`../../fixtures/ekf-kernel.mec`.

## Exact checkout and source map

The complete spike is on branch `codex/mech-program-gpu` and is reviewed in
[PR #757](https://github.com/mech-lang/mech/pull/757). It is not present in a
plain `integration/value-executor-v0.4` checkout.

```text
git fetch origin codex/mech-program-gpu
git switch --track origin/codex/mech-program-gpu
```

| Role | Checked-in source |
| --- | --- |
| High-level Mech EKF | `hosts/gpu/fixtures/ekf-kernel.mec` |
| Mech artifact benchmark harness | `hosts/gpu/examples/parallel_ekf_benchmark.rs` |
| Generic scalar, SIMD, and WGPU lowering/execution | `hosts/gpu/src/batched.rs` |
| Cranelift lowering/execution | `hosts/gpu/src/batched/jit.rs` |
| Optimized Rust control | `hosts/gpu/examples/parallel_ekf_rust_scalar.rs` |
| NumPy control | `hosts/gpu/benchmarks/parallel-ekf/numpy_scalar.py` |
| Julia control | `hosts/gpu/benchmarks/parallel-ekf/julia_scalar.jl` |
| LuaJIT control | `hosts/gpu/benchmarks/parallel-ekf/luajit_scalar.lua` |
| Controlled runner | `hosts/gpu/benchmarks/parallel-ekf/run.py` |
| Correctness tests | `hosts/gpu/tests/parallel_ekf.rs` |

## Mech physical backends

The scalar CPU, four-lane SIMD CPU, Cranelift JIT, and GPU lanes execute the
same compiler artifact and persistent per-filter state. The SIMD
implementation changes only the physical value type of the scalarized
instruction stream to `wide::f32x4`; it uses NEON on Apple Silicon and SSE
where available. The JIT converts that instruction stream into one native SSA
function containing the complete outer filter loop. The primary GPU lane
submits and synchronizes one Mech turn at a time. The batched GPU lane is
reported separately because it records 120 dependent turns in one submission
and therefore amortizes synchronization.

Apple M1 median of five processes after one discarded process warmup,
100,000 filters, 2026-08-14:

| Mech backend | Million EKF-turns/s | Scalar speedup |
| --- | ---: | ---: |
| Mech scalar artifact evaluator | 1.216 | 1.00x |
| Mech SIMD (`4xf32`) | 4.414 | 3.63x |
| Mech Cranelift JIT | 17.306 | 14.23x |
| Mech GPU, one submission/turn | 53.557 | 44.04x |
| Mech GPU, 120 turns/submission | 343.969 | 282.87x |

Parsing, artifact compilation, scalarization, JIT compilation, input
construction, allocation, GPU setup, warmup, final readback, and correctness
checks are outside the timed regions. Cranelift `0.131.3` is pinned because it
supports the repository's Rust `1.92` minimum. JIT preparation took `3.340 ms`
in the first recorded hardware run. Its state matched the scalar evaluator
bit-for-bit after four validation turns.

## Scalar outer-loop languages

Every lane owns 10,000 persistent filters and executes one filter at a time
for five warmup turns, a state reset, and 20 measured turns. Inputs, equations,
`f32` state, Joseph covariance update, and checksums agree. "Scalar" here means
the outer filter loop is sequential. It does not claim that a language's
scalar math or small matrix library avoids every SIMD instruction internally.

Apple M1 median of five processes after one discarded process warmup, except
seven LuaJIT samples:

| Scalar outer-loop lane | Million EKF-turns/s | Relative to Mech scalar |
| --- | ---: | ---: |
| Mech scalar artifact evaluator | 1.213 | 1.00x |
| Mech Cranelift JIT | 17.390 | 14.34x |
| Rust optimized fixed-shape | 20.299 | 16.73x |
| NumPy sequential small matrices | 0.055 | 0.05x |
| Julia sequential small matrices | 2.786 | 2.30x |
| LuaJIT sequential FFI `f32` state | 1.089 | 0.90x |

The Rust control permits inlining of the EKF step and its fixed-shape matrix
helpers. The previous `#[inline(never)]` control measured `12.947 M/s`, but it
was not a fair native-code ceiling once the JIT owned and fused the outer
filter loop. Under identical 10,000-filter, 20-turn settings, the JIT reaches
`85.7%` of the optimized Rust throughput.

Versions were Rust `1.96.0-nightly`, Python `3.14.6`, NumPy `2.5.2`, Julia
`1.12.6`, and LuaJIT `2.1.1785763465`. NumPy, Julia BLAS, and related native
thread counts were pinned to one.

The publication evidence is checked in as
[`results/apple-m1-2026-08-14.json`](results/apple-m1-2026-08-14.json). It was
generated from commit `6b27e4cdbcdd53ddb0c646169be0bb597bd2a39e` and contains
all discarded warmups and measured stdout. The raw samples also show why these
figures remain provisional: synchronized GPU samples ranged from `48.613` to
`65.510 M/s`, while the JIT backend samples stayed between `17.225` and
`17.343 M/s` at the 100,000-filter setting.

Build the native Mech benchmark, then run the complete comparison:

```text
cargo build -p mech-gpu --release --features native,jit --example parallel_ekf_benchmark
python3 hosts/gpu/benchmarks/parallel-ekf/run.py --python /path/to/python-with-numpy
```

Add `--evidence-output /path/to/results.json` to record the exact Git commit,
platform, tool versions, thread environment, commands, discarded warmups,
every measured process stdout, parsed checksums, and summary medians. Published
results should include this generated JSON rather than only the tables above.

The runner compiles the checked-in Rust control directly with
`rustc -C opt-level=3 -C target-cpu=native`, validates every scalar and JIT
checksum, and prints both Markdown tables.
