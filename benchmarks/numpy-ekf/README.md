# Batched NumPy EKF comparison

The new array interpreter and its fresh comparison with the unchanged tuned
NumPy control are documented in [ARRAY_OPTIMIZATION.md](ARRAY_OPTIMIZATION.md).

The expanded experiment is documented in [FAIR_COMPARISON.md](FAIR_COMPARISON.md).
It selects among standard NumPy layouts/contractions and measures Mech's
existing SIMD interpreter, SIMD JIT, and SIMD AOT paths as well. The initial
campaign below is retained as historical evidence, not the current best
implementation comparison.

This experiment tests the effect of replacing the archived NumPy control's
Python loop over independent filters with NumPy stacked matrix operations.
It compares fresh measurements of both NumPy implementations with Mech's
scalar fixed-shape artifact executor and single-worker Cranelift JIT. The
scalar artifact executor is not the ordinary nalgebra-backed interpreter.

## Results: Apple M1, 2026-10-11

Throughput in **million filter-turns/s, median ± MAD**; higher is faster.
All lanes use one worker/thread, float32, twenty turns, and no constraint
enforcement. The two columns have ten and seven samples per lane,
respectively.

| Implementation | 10,000 filters | 100,000 filters |
| --- | ---: | ---: |
| NumPy sequential small matrices | 0.052603 ± 0.000176 | Not measured |
| NumPy batched stacked matrices | 2.280812 ± 0.010790 | 2.273763 ± 0.001128 |
| Mech scalar artifact executor | 1.206441 ± 0.000595 | 1.203480 ± 0.000666 |
| Mech Cranelift JIT | 18.787025 ± 0.026990 | 18.699897 ± 0.018137 |

At 10,000 filters, the batched NumPy implementation is **43.36× faster**
than the sequential NumPy control and **1.89× faster** than Mech's scalar
artifact executor. Mech JIT is **8.24× faster** than batched NumPy. At
100,000 filters, the corresponding NumPy/scalar and JIT/NumPy ratios are
1.89× and 8.22×. Ratios use median throughputs.

This initial experiment established that batching materially changes the
NumPy comparison. It did not establish NumPy's maximum attainable
performance or measure the general-purpose Mech interpreter or Numba.

All 23 full-output validation runs across six scenarios passed. Batched NumPy and sequential
NumPy were bit-identical in every shared validation case; Mech scalar and
JIT were also bit-identical in every validation case. Across the NumPy/Mech
comparisons, maximum absolute state error was `2.0981e-5` and covariance
error was `1.5259e-4`. Every element was within the preset combined
absolute/relative tolerance; the largest error used 18.1% of its bound.
Each lane's checksum was identical across its repeated timed samples.

Environment: Apple M1, 8 GiB RAM, macOS 15.6.1; Python 3.12.14, NumPy 2.3.5
with Apple Accelerate; Rust 1.96.0-nightly. Mech repository revision:
`c217f2748fa8296940edf1d9122cb62bc9a9d69c`, plus the benchmark harness
archived here. No backend changes were made for this comparison.

[Raw samples, validation, and provenance](results/apple-m1-2026-10-11/results.json)
are accompanied by the exact binary inputs and full validation dumps in
the same result directory. Four NumPy unit tests and the driver's
nonsymmetric two-filter covariance-layout unit test also passed.

## Implementations

- `numpy_sequential.py` calls the unchanged `Scratch` and `step` implementation
  in `../archive/compute/parallel-ekf/numpy_scalar.py` once per filter per turn.
- `numpy_batched.py` uses stacked `matmul`, broadcasting, vectorized
  trigonometric functions, preallocated scratch arrays, and `out=` operations.
  Its only execution loop in Python is over turns, not filters. It uses no
  Numba, custom native extension, or hand-written scalar EKF kernel.
- The original `mech-driver` compiled `../../hosts/gpu/fixtures/ekf-kernel.mec` using the
  repository's existing broadcast lowerer and scalar/JIT APIs. It does not
  initialize a GPU or modify either backend.

All lanes use float32 state, the same initial conditions, and the same
predict/correct equations with a Joseph covariance update. Exact shared binary
input arrays remove language-specific input-generation differences. As in
the archived workload, each filter reuses its input values on every turn.

## Measurement protocol

The collector first validates every final state and covariance element,
including both benchmark batch sizes. It checks 1, 4, 20, and 100-turn cases
and non-vector-width filter counts. Comparison uses
`abs(candidate - reference) <= 1e-4 + 1e-5 * abs(reference)`, fixed before
measurement. NumPy computes innovation variance as `H(PHᵀ)` while Mech uses
`(HP)Hᵀ`, so mathematically equivalent float32 arithmetic can differ in
rounding. Validation failure stops the campaign.

The workloads are 10,000 filters × 20 turns (ten samples of all four lanes)
and 100,000 filters × 20 turns (seven samples of the three batched/native
lanes). Each sample runs in a fresh process, warms up for five turns, resets
to the initial state, then times twenty turns. Each campaign also discards
one process per lane. Lane order is shuffled within each round with a fixed
seed. NumPy/BLAS thread settings and Mech worker count are one. Run the
collector only after all compilation has finished, without another benchmark
running concurrently.

Timing includes execution, per-turn state updates, and allocations performed
inside execution. Mech JIT's single `dispatch_turns(20)` also refreshes its
host-readable state snapshot before the timer stops. Imports, source/JIT
compilation, initial allocation/setup, input binding, warmup/reset, checksums,
and file output are excluded. There is no turn fusion: NumPy updates its
arrays in place each turn, while Mech uses ping-pong candidate buffers.
These are not claims of identical atomic-publication machinery.

All lanes are **unchecked**, matching the NumPy comparison under review.
Mech uses `without_integrity_constraints()`. This disables constraint
enforcement and fault handling; in this repository snapshot, the scalar
executor can still evaluate unused predicate intermediates retained in its
IR, whereas JIT optimization can eliminate them. This experiment neither
measures checked rollback nor changes that API to improve its score.

Results use median throughput and unscaled median absolute deviation (MAD).
The raw JSON retains every sample, validation result, input/source/binary
hash, build flags, revision, and environment information. These are new
measurements; do not splice them into the historical campaign or describe
this baseline as the fastest possible NumPy implementation. Numba, Rust
controls, multi-worker execution, and GPU execution are outside this test.

## Reproduction

The current driver uses a constraint-free source and therefore must not be
used to reproduce this initial campaign. The original driver source and
manifest are retained under
[`results/apple-m1-2026-10-11/source/mech-driver/`](results/apple-m1-2026-10-11/source/mech-driver/),
and its exact source/binary hashes and build flags are in the original raw
report. The expanded experiment's reproduction commands are in
[FAIR_COMPARISON.md](FAIR_COMPARISON.md).
