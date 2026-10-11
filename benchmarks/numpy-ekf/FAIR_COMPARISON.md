# Expanded NumPy / Mech comparison

This is a new experiment, not a replacement of the measurements in
`results/apple-m1-2026-10-11/`. Its primary question is steady-state CPU
throughput for the same numerical workload using each system's existing
execution paths. An implementation may batch operations, use SIMD, change
its internal layout, or compile the numerical graph. Matching those internal
strategies is not the fairness criterion; matching the work, arithmetic
precision, available CPU resources, and observable results is.

## Results: Apple M1, 2026-10-11

Million filter-turns per second, **median ± unscaled MAD**; higher is faster.
All rows use float32, one worker/native thread, twenty dependent turns, and
no user integrity predicates. SIMD is permitted on both sides. There are
ten, ten, and seven fresh-process samples per row in the three columns.

| Execution path | 256 filters | 10,000 filters | 100,000 filters |
| --- | ---: | ---: | ---: |
| NumPy stacked `matmul`, filter-major | 1.557 ± 0.007 | 2.246 ± 0.025 | 2.271 ± 0.003 |
| NumPy `einsum`, component-major | 5.685 ± 0.019 | 21.814 ± 0.041 | 19.969 ± 0.073 |
| Mech scalar instruction interpreter | 1.076 ± 0.002 | 1.078 ± 0.002 | 1.075 ± 0.004 |
| Mech SIMD-4 instruction interpreter | 4.824 ± 0.009 | 4.829 ± 0.004 | 4.810 ± 0.010 |
| Mech scalar JIT | 16.914 ± 0.005 | 16.920 ± 0.026 | 16.899 ± 0.006 |
| Mech SIMD-4 JIT | 38.156 ± 0.030 | 38.986 ± 0.029 | 38.880 ± 0.028 |
| Mech SIMD-4 AOT | 37.068 ± 0.067 | 37.103 ± 0.048 | 37.228 ± 0.042 |

At 10,000 filters, the selected NumPy implementation is 4.52× faster than
Mech's SIMD instruction interpreter and 1.29× faster than scalar JIT.
Mech's SIMD JIT and AOT are respectively 1.79× and 1.70× faster than that
NumPy implementation. At 100,000 filters, SIMD JIT and AOT are 1.95× and
1.86× faster. These are workload-specific results, not claims about every
NumPy program or the fastest possible NumPy implementation.

The small batch shows the importance of reporting workload size: NumPy's
selected implementation reaches 5.68 M/s at 256 filters versus 21.81 M/s at
10,000. All samples are retained, including the slower samples. In particular,
the seven 100,000-filter NumPy `einsum` samples range from 16.709 to 20.042 M/s;
three are below 18.2 M/s, so the small MAD does not describe the whole lower
tail. No samples were removed as outliers.

All 49 candidate full-output checks against seven reference runs passed,
followed by 189 timed samples and 21 discarded warm processes. Maximum
absolute differences from the reference were `3.052e-5` for state and
`1.907e-4` for covariance; no element used more than 22.8% of its preset
tolerance. Each lane's checksum was unchanged across repeated timed runs.
The seven NumPy unit tests and the native covariance-layout test passed.

Environment: Apple M1, 8 GiB RAM, macOS 15.6.1; Python 3.12.14, NumPy 2.3.5
with Apple Accelerate; Rust 1.96.0-nightly. The five Mech rows use one binary
built from the same pinned backend revision. No backend implementation was
changed for these measurements.

[Raw measurements, validation, and provenance](results/apple-m1-2026-10-11-fair/results.json)
include all samples, commands, source/input/output/binary hashes, copied
harness sources, and the pinned build manifest and dependency lockfile.

## What this says about the interpreter

There is a measured gap between NumPy's batch-wide array execution and
Mech's instruction interpreters, even when the latter uses four-wide SIMD.
The current SIMD interpreter loops over groups of four filters, gathers
their values, dispatches each lowered instruction, and scatters updated
state. NumPy's component-major implementation instead dispatches an array
operation over contiguous components of the whole batch. The JIT/AOT
paths compile the numerical instruction sequence instead of interpreting
each instruction repeatedly.

This source-level difference identifies a useful optimization experiment:
component-major storage and batch-wide instruction execution in the
interpreter, reducing repeated dispatch and gather/scatter work. It is not
a profile attributing a measured percentage to those costs. Any improvement
must preserve the same full-output checks and be measured in a new campaign;
the results above are the pre-optimization baseline.

The **public reactive f32 runtime** was tested separately, not substituted
for either instruction-interpreter row. Its four-filter, twenty-turn probe
failed numerical validation (maximum absolute difference `0.240364`), so
the collector stopped before taking performance samples. Initial-state
checks passed, but this mismatch still requires diagnosis; no timing from
that failing path is used in the table. Its separate scheduling, snapshot,
and publication work would also need to be stated when reporting it.
[Retained failed-check evidence](results/apple-m1-2026-10-11-resident-diagnostic/results.json)
records the unchanged tolerance and full outputs.

A [follow-up first-turn audit](results/apple-m1-2026-10-11-resident-first-turn-audit/results.json)
passes after one turn but fails after two turns (36 of 48 components outside
tolerance). Source inspection suggests an f32 snapshot change-notification
problem: dot products declare `ExactScalar` change detection, while the
scalar-token helper returns `None` for snapshot values. This could suppress
downstream recomputation. That is a specific hypothesis for a minimal
regression test, not an established causal fix or a performance conclusion.

## Common contract

- Identical shared float32 velocity, angular-velocity, and bearing arrays;
  identical initial state, covariance, constants, and Joseph EKF recurrence.
- Twenty dependent turns per independent filter. Every turn updates state;
  the host requires the complete final state and covariance, not an
  intermediate snapshot after each turn. No fusion across successive turns.
- One worker; NumPy/BLAS thread limits are one. Native SIMD is allowed on
  both sides. A single thread does not imply scalar instructions.
- No user integrity predicates on either side. The new Mech source omits
  predicate declarations and helper calculations, rather than merely
  disabling enforcement after lowering. This does not disable ordinary
  runtime type/shape checking or change any production backend.
- Compilation, initial allocation, and input binding happen before timing.
  Five warmup turns are followed by an explicit reset. Timing includes
  execution, state updates, and any allocations performed inside execution;
  checksums and binary output are computed afterward.
- Full elementwise output validation precedes timing, using the original
  tolerance `abs(candidate-reference) <= 1e-4 + 1e-5*abs(reference)`.
  A failed check prevents performance collection for that lane/workload.
- Samples run in fresh processes, interleaved in randomized order, with no
  simultaneous compilation or other benchmark. Report median throughput
  and unscaled median absolute deviation, not the best sample.

## NumPy implementation selection

The candidates were the original preallocated stacked `matmul`, `einsum`
with filter-major storage, `einsum` with component-major storage, and
stacked `matmul` with component-major storage. They use the same matrix
equations; none uses Numba, generated native code, or a scalar-expanded
EKF kernel. Component-major storage makes each component across the batch
contiguous without packing or unpacking it every turn.

A separate pilot used three fresh-process samples and one discarded process
per candidate at 10,000 and 100,000 filters. The selection rule was the
geometric mean of median throughput at those two batch sizes. The selected
candidate was **component-major `einsum`**, which is rerun using independent
samples in the final comparison. The original stacked-`matmul` row is also
retained to show the effect of this choice.

[Pilot evidence](results/apple-m1-2026-10-11-numpy-pilot/results.json) retains
all candidates, commands, outputs, versions, hashes, and validation results.
Pilot results are not substituted for final measurements.

## Mech execution paths

The scalar artifact executor, four-lane SIMD artifact executor, scalar JIT,
and the paper's existing four-lane SIMD JIT/AOT paths are distinct rows.
These artifact executors must not be labeled as the general reactive
interpreter or as a nalgebra matrix interpreter. The old source interpreter
is a private compiler-planning implementation, not a public runtime API.
The current checkout's public resident matrix multiplication requires f64,
so it cannot silently stand in for an f32 control. The newer paper snapshot
has public f32 resident numeric bindings; any measurement of that path is
a separate reactive-runtime diagnostic, including its ordinary snapshot,
scheduling, and publication costs.

All five Mech rows use the same paper revision,
`0d85319d60c460cdecb81e03c181d62ecfe1ef39`, built in an isolated workspace.
They compile the same constraint-free numerical source and consume the same
binary inputs. Neither an older backend revision nor an older performance
number is substituted for an interpreter or compiled row.

## Reproduction

Use Python with NumPy installed and the repository's pinned Rust toolchain.
Build the pinned driver before collecting samples:

```sh
python benchmarks/numpy-ekf/paper-driver/prepare.py
```

The preparation script prints a new temporary snapshot directory. Substitute
that directory for `/path/to/paper-snapshot` below:

```sh
RUSTFLAGS='-D warnings -A dead_code' cargo build --release --offline \
  --manifest-path /path/to/paper-snapshot/build/Cargo.toml \
  --bin numpy-ekf-paper-driver

python -m unittest discover -s benchmarks/numpy-ekf -p 'test_numpy*.py'

env OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1 \
  VECLIB_MAXIMUM_THREADS=1 NUMEXPR_NUM_THREADS=1 BLIS_NUM_THREADS=1 \
  python benchmarks/numpy-ekf/run_fair_comparison.py \
  --paper-binary /path/to/paper-snapshot/build/target/release/numpy-ekf-paper-driver \
  --paper-build-provenance /path/to/paper-snapshot/build-provenance.json \
  --pilot benchmarks/numpy-ekf/results/apple-m1-2026-10-11-numpy-pilot/results.json \
  --output benchmarks/numpy-ekf/results/new-independent-run
```

The collector verifies the pilot's selected candidate and source hashes,
retains the harness sources and paper build manifest/lockfile, and refuses
to overwrite an existing report. It tests 256, 10,000, and 100,000 filters
using ten, ten, and seven fresh-process samples per lane. The `dead_code`
allowance is local to these CPU-only builds; no global lint or unsafe
policy is changed. Offline builds require cached dependencies.
For exact dependency reproduction, put the retained
`results/apple-m1-2026-10-11-fair/paper-build/Cargo.lock` in the generated
snapshot's `build/` directory before compiling, and add `--locked` to Cargo.
