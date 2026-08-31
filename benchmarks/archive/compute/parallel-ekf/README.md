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
| NumPy control | `benchmarks/archive/compute/parallel-ekf/numpy_scalar.py` |
| Julia generic control | `benchmarks/archive/compute/parallel-ekf/julia_scalar.jl` |
| Julia fixed-shape control | `benchmarks/archive/compute/parallel-ekf/julia_flat.jl` |
| Julia packed-lane control | `benchmarks/archive/compute/parallel-ekf/julia_simd.jl` |
| LuaJIT control | `benchmarks/archive/compute/parallel-ekf/luajit_scalar.lua` |
| Controlled runner | `benchmarks/archive/compute/parallel-ekf/run.py` |
| Correctness tests | `hosts/gpu/tests/parallel_ekf.rs` |

## Mech physical backends

The scalar CPU, four-lane SIMD CPU, Cranelift JIT, and GPU lanes execute the
same compiler artifact and persistent per-filter state. The SIMD
implementation changes only the physical value type of the scalarized
instruction stream to `wide::f32x4`; it uses NEON on Apple Silicon and SSE
where available. The JIT converts that instruction stream into one native SSA
function containing the complete outer filter loop. The primary GPU lane
submits and synchronizes one Mech turn at a time.

The Mech source itself declares `finite-candidate!`,
`positive-covariance!`, and `symmetric-covariance!` using generic numeric,
comparison, Boolean, and matrix-index operations. There is no EKF validation
primitive or separate Rust publication policy. Constraint names survive
artifact and bytecode encoding, affect artifact identity, and are reported by
structured faults. A failed candidate is rejected before the published buffer
changes. The session records only a fault count and latest named fault, so
fault evidence cannot grow an unbounded log. GPU turns read a compact device
fault status before advancing the published ping-pong buffer; checked
multi-turn calls therefore execute as repeated checked turns.

The table below is the preserved **unchecked** Apple M1 baseline from commit
`6b27e4cdbcdd53ddb0c646169be0bb597bd2a39e`: five-process median after one
discarded warmup, 100,000 filters, 2026-08-14. It predates the integrity policy
and must not be presented as checked throughput.

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

## Initial checked evaluated artifact

Commit `7605c5c9081a22d7bcba0b0c288570a7c3a41f41` compiles the three
source-authored constraints into every backend. Five release-mode processes
on Apple M1 Metal, with 100,000 filters, three scalar reference turns, 20
single GPU samples, and 120 repeated checked GPU turns, produced these
medians:

| Checked Mech backend | Time/turn | Million EKF-turns/s | Unchecked reference | Relative change |
| --- | ---: | ---: | ---: | ---: |
| Scalar artifact evaluator | 122.212 ms | 0.818 | 1.216 | -32.7% |
| SIMD (`4xf32`) | 31.702 ms | 3.154 | 4.414 | -28.5% |
| Cranelift JIT | 8.105 ms | 12.339 | 17.306 | -28.7% |
| GPU, one checked submission/turn | 1.942 ms | 51.497 | 53.557 | -3.8% |
| GPU, repeated checked turns | 1.767 ms | 56.580 | not comparable | not comparable |

Source parsing, artifact construction, and scalarization took a median
`107.022 ms`; JIT preparation took `3.573 ms`. Maximum CPU/GPU absolute error
was `6.866e-5`, and JIT output matched scalar output bit-for-bit. The Apple
Metal correctness suite passed all nine tests, including injected finite,
positive-diagonal, and symmetry failures and proof that an invalid GPU
candidate leaves the previous state published.

The old `343.969 M/s` GPU number is deliberately excluded from the overhead
calculation: it publishes only after a 120-turn command batch, while the
checked repeated lane validates before every publication. Comparing those
numbers would attribute a guarantee-boundary change to constraint arithmetic.
All five checked process samples are preserved in
[`results/apple-m1-checked-integrity-2026-08-14.json`](results/apple-m1-checked-integrity-2026-08-14.json).

## Optimized checked artifact

Commit `efc85d48e562fe4ccc1af535e04f9bf4617e05a6` keeps the same source
constraints, named faults, candidate rejection, bounded fault state, and
previous-estimate retention. It changes their generic execution strategy:

- constraint-only Boolean graphs compile as predicates rather than `f32`
  result registers;
- dead numeric instructions are removed after tracing state outputs and
  predicate inputs;
- `abs(x) <= f32::MAX` lowers to exact `f32` finiteness testing and
  `abs(left - right) <= tolerance` lowers to one predicate;
- SIMD comparisons remain native masks until the final fault decision; and
- JIT code returns its first packed fault instead of writing and rescanning a
  result for every filter.

Five isolated release processes on the same Apple M1 Metal adapter, with
100,000 filters, five scalar reference turns, 40 single GPU samples, and 120
repeated checked GPU turns, produced:

| Checked Mech backend | Time/turn | Million EKF-turns/s | Initial checked | Change | Unchecked reference | Remaining checked cost |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar artifact evaluator | 106.286 ms | 0.941 | 0.818 | +15.0% | 1.216 | -22.6% |
| SIMD (`4xf32`) | 26.233 ms | 3.812 | 3.154 | +20.9% | 4.414 | -13.6% |
| Cranelift JIT | 6.849 ms | 14.600 | 12.339 | +18.3% | 17.306 | -15.6% |
| GPU, one checked submission/turn | 1.630 ms | 61.348 | 51.497 | +19.1% | 53.557 | noisy |
| GPU, repeated checked turns | 1.820 ms | 54.959 | 56.580 | -2.9% | not comparable | not comparable |

Source parsing, artifact construction, and scalarization took a median
`64.134 ms`; JIT preparation took `3.494 ms`. Maximum CPU/GPU absolute error
remained `6.866e-5`, and JIT output remained bit-for-bit equal to scalar.
All 27 package tests passed, including the three injected integrity failures
and GPU publication retention. The generated WGSL shrank from 13,908 to
11,733 bytes as dead constraint arithmetic disappeared.

GPU one-turn samples ranged from `47.859` to `62.495 M/s`; per-turn host
synchronization still dominates and makes this too noisy to attribute the
median change to device predicate lowering. Raw samples, command parameters,
implementation commit, and validation commands are preserved in
[`results/apple-m1-checked-integrity-optimized-2026-08-14.json`](results/apple-m1-checked-integrity-optimized-2026-08-14.json).

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
all discarded warmups and measured stdout. This file is retained as pre-policy
evidence rather than silently relabeled. The raw samples also show why these
figures remain provisional: synchronized GPU samples ranged from `48.613` to
`65.510 M/s`, while the JIT backend samples stayed between `17.225` and
`17.343 M/s` at the 100,000-filter setting.

Build the native Mech benchmark, then run the complete comparison:

```text
cargo build -p mech-gpu --release --features native,jit --example parallel_ekf_benchmark
python3 benchmarks/archive/compute/parallel-ekf/run.py --python /path/to/python-with-numpy
```

Add `--evidence-output /path/to/results.json` to record the exact Git commit,
platform, tool versions, thread environment, commands, discarded warmups,
every measured process stdout, parsed checksums, and summary medians. Published
results should include this generated JSON rather than only the tables above.

The runner compiles the checked-in Rust control directly with
`rustc -C opt-level=3 -C target-cpu=native`, validates every scalar and JIT
checksum, and prints both Markdown tables.

## Julia inlining probe

The Julia control in this checkout uses `Base.@inline` on `step!`.  This is a
deliberate optimization hint for the scalar outer loop, not a change to the
EKF equations or storage.  On the Apple M1, nine isolated 20-turn processes
with 10,000 filters produced `3.091M` lane-turns/s median.  The same source
with the annotation removed produced `2.875M`; the original `Base.@noinline`
source produced `2.852M`.  A longer 100-turn corroboration produced `3.069M`,
`2.820M`, and `2.800M`, respectively.  Checksums were identical across all
variants and one-process startup wall time remained within `1.57--1.59s`.  The
detailed commands and raw medians are in
[`results/julia-inline-apple-m1-2026-08-30.md`](results/julia-inline-apple-m1-2026-08-30.md).

The Julia comparison has four explicitly named modes. The generic source uses
ordinary heap-backed `Matrix` values and `mul!`, which is the closest
translation of the high-level Mech matrix expressions. The fixed-shape source
uses flat `Float32` buffers and compile-time 3x3/3x2 products, matching the
storage and operation shape of the optimized Rust control. Both sources accept
`unchecked` or `checked` as a third argument. Checked mode evaluates the same
finite-state, finite-covariance, positive-diagonal, and covariance-symmetry
predicates as the Mech artifact before publishing a candidate; a failed
candidate leaves the prior state unchanged and increments the fault count.
The runner executes all four Julia rows, while the Rust row remains a raw
unchecked control by design.

In a five-process Apple M1 probe with 10,000 filters and 20 measured turns,
the current medians were:

| Julia implementation | Validation | Million lane-turns/s |
| --- | --- | ---: |
| Generic Matrix/`mul!` | unchecked | 3.09 |
| Generic Matrix/`mul!` | checked | 3.08 |
| Fixed-shape flat tuples | unchecked | 21.9 |
| Fixed-shape flat tuples | checked | 19.0 |

All four modes produced the same checksum within the existing `f32`
tolerance. The fixed-shape checked result is the relevant comparison to a
checked Mech numeric backend; the unchecked result isolates arithmetic and
storage cost only.

## Julia packed-lane comparison

`julia_simd.jl` gives Julia the same four-filter physical execution shape as
Mech's SIMD-JIT lane. It stores each state and covariance component as a
`StaticArrays.SVector{4,Float32}`, advances four independent filters per outer
iteration, uses Julia's `sincos` pair, and keeps the same checked-mode
finite/positive-diagonal/symmetry predicates. A fully valid group takes a
branch-only publication path; per-lane `ifelse` rollback is materialized only
when a lane fails. This is a fair packed-lane Julia comparison, while the
generic and fixed-shape sequential rows remain available to show the cost of
the language/runtime shape itself.

On this Apple M1 checkout with 100,000 filters and 100 measured turns (after
the script's five-turn warmup), the direct runs were:

| Julia implementation | Validation | Million lane-turns/s |
| --- | --- | ---: |
| Fixed-shape flat tuples | unchecked | 22.69 |
| Fixed-shape flat tuples | checked | 19.57 |
| Fixed-shape packed `SVector{4}` | unchecked | 37.72 |
| Fixed-shape packed `SVector{4}` | checked | 29.83 |

The packed source produced the same `26,697,851.679688` checksum as the flat
source (within `f32` accumulation rounding). The checked packed result is
therefore close to Mech's checked SIMD-JIT result on this run, rather than an
unchecked Julia-only advantage. `StaticArrays` is already an installed Julia
dependency; no optional SIMD package is required.
