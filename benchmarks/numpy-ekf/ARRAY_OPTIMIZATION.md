# Generic array execution for Mech

This experiment adds component-contiguous storage and array execution to the
fixed-shape CPU interpreter. Each lowered instruction operates over a tile of
independent program instances, using SIMD for contiguous groups of four and
masked partial SIMD groups. This amortizes instruction dispatch and avoids gathering each
component from individual filter records. The backend accepts the generic
fixed-shape instruction representation; it does not contain an EKF-specific
implementation.

The plan also combines short ordered sums of products and reuses repeated
trigonometric results for the same input version. When both sine and cosine
are needed, it computes them together. These transformations apply to the
instruction representation rather than recognizing the EKF equations.

The numerical source, inputs, f32 precision, and preset output tolerance match
the [pre-optimization comparison](FAIR_COMPARISON.md). The NumPy control is the
same component-major `einsum` implementation selected in the retained NumPy
pilot. The original campaigns and sources are retained unchanged.

## Results

The unchanged numerical source now runs 4.64–4.93 times faster through the
array interpreter than through the legacy four-lane instruction interpreter
on this Apple M1. Against the unchanged, independently selected NumPy
`einsum` control, throughput is 3.93 times higher at 256 filters, 9.2% higher
at 10,000 filters, and 16.2% higher at 100,000 filters. These are measurements
of this EKF workload and machine, not a general language ranking.

Throughput in million filter-turns/s, median ± unscaled MAD:

| Execution path | 256 filters | 10,000 filters | 100,000 filters |
| --- | ---: | ---: | ---: |
| NumPy component-major `einsum` | 5.687 ± 0.021 | 21.807 ± 0.078 | 20.003 ± 0.052 |
| Mech scalar instruction interpreter | 1.081 ± 0.001 | 1.080 ± 0.002 | 1.073 ± 0.003 |
| Mech legacy SIMD instruction interpreter | 4.825 ± 0.003 | 4.826 ± 0.003 | 4.813 ± 0.010 |
| Mech scalar JIT | 16.912 ± 0.002 | 16.922 ± 0.011 | 16.910 ± 0.004 |
| Mech SIMD JIT | 38.173 ± 0.059 | 38.964 ± 0.036 | 38.889 ± 0.035 |
| Mech SIMD AOT | 36.884 ± 0.189 | 37.054 ± 0.040 | 37.250 ± 0.090 |
| Mech array instruction interpreter | 22.371 ± 0.122 | 23.807 ± 0.030 | 23.248 ± 0.038 |

The first two batch sizes have ten fresh-process samples per row; the largest
has seven. All 189 samples are retained, including slower processes, and no
outliers were removed. The SIMD JIT and AOT paths are still faster than the
new array interpreter. The improvement does not require JIT compilation or
an EKF-specific native kernel: it amortizes generic instruction dispatch over
contiguous components and executes short contractions and shared
trigonometric operations over SIMD tiles.

The independent [tile pilot](results/apple-m1-2026-10-11-array-sincos-pilot-verified/results.json)
selected 512 filters. This is also the public array executor's default,
verified separately against explicit tile selection. Final measurements use
the explicit limit of 512; the 256-filter workload uses only 256 slots.
The EKF plan uses 51 scratch columns and 104,448 bytes at the full tile,
with two shared sine/cosine pairs and two additional reused trigonometric
results.

All 80 full-output validation records passed the original tolerance,
including partial SIMD groups at 1, 17, and 257 filters and 100-turn
trajectories. The array executor's largest absolute errors were
`3.0517578125e-5` for state and `1.9073486328125e-4` for covariance; its largest
error used 22.8% of the permitted component-wise bound. The bound was not
changed during optimization.

The [complete final record](results/apple-m1-2026-10-11-array-final/results.json)
contains every sample, command, full-output validation result, input/output
hash, source copy, build manifest, lockfile, patch, and binary hash.
The earlier pilot is retained, as is an incomplete intermediate pilot whose
provenance check rejected a concurrent test-source change; neither supplied
final performance samples.

## Correctness tests

Fourteen release integration tests passed with warnings denied against both
the patched paper snapshot and the actual current checkout
`c217f2748fa8296940edf1d9122cb62bc9a9d69c` with the production changes.
The current-checkout test used JIT only, without the paper snapshot's AOT
feature. Its preliminary `cargo check --tests` also passed. The generated
[manifest, lockfile, and source provenance](results/apple-m1-2026-10-11-array-final/current-tests/test-provenance.json)
are retained with the final campaign.

The tests cover rectangular matrix recurrence,
elementwise operations, ordered mixed-sign contractions, partial SIMD groups,
input updates, whole-batch rejection and recovery, multi-turn rollback,
deterministic fault selection, constant-only state, checked and unchecked
execution, and the registered CPU SIMD factory. Trigonometric tests cover
operand overwrites and input changes, as well as all `atan2` quadrants and
signed zero. A separate driver unit test checks nonsymmetric covariance
conversion to the public dump layout.

Signed-zero tests exposed that the SIMD library's negation formed `0 - x`,
losing the sign of zero, and that `atan2` also needed to preserve the sign of
its first operand. The array executor now flips the sign bit for negation
and restores the `atan2` result's sign from that operand. Exact sign-bit
tests pass; this repair did not relax numerical tolerances.

## Measurement protocol

Every Mech row is built from revision
`0d85319d60c460cdecb81e03c181d62ecfe1ef39`, with the production array executor
copied verbatim, a module export patch, and three substitutions connecting
the public CPU SIMD factory to the array executor. The preparation script
records the source and patch hashes. Legacy scalar/SIMD interpreters, scalar
JIT, SIMD JIT, SIMD AOT, and the new array interpreter share this one binary.

All measurements use one worker, f32 arithmetic, five warmup turns followed
by a reset, twenty dependent turns, and complete final state and covariance.
The timed endpoint includes conversion required by the public state API.
Compilation, initial allocation, input binding, warmup/reset, checksums, and
file output are outside the timed interval. User integrity predicates are
omitted from the common numerical fixture; the array backend's integrity and
rollback behavior is tested separately.

Before performance sampling, every output component must meet
`abs(candidate - reference) <= 1e-4 + 1e-5 * abs(reference)`. The reference uses
the independent NumPy matrix implementation. Validation includes partial SIMD groups
and long trajectories. A failed check stops collection.

A separate pilot tests tile sizes 32, 64, 128, 256, 512, 1024, and 4096.
Selection uses the geometric mean of median throughput at 10,000 and 100,000
filters. The final campaign reruns the selected tile with independent samples
at 256, 10,000, and 100,000 filters. Processes run sequentially in randomized
order, with native thread limits set to one. All samples and discarded warm
processes are retained; results report median and unscaled MAD.

The configured tile is a capacity limit: batches smaller than the tile use
fewer SIMD groups. Driver records include the effective capacity, scratch
storage, and shared trigonometric operations. These diagnostics are collected
before the timer.

## Reproduction

Prepare a new isolated snapshot, optionally using the previous retained lock
as a dependency seed:

```sh
python benchmarks/numpy-ekf/array-driver/prepare.py \
  --lock benchmarks/numpy-ekf/results/apple-m1-2026-10-11-fair/paper-build/Cargo.lock

RUSTFLAGS='-D warnings -A dead_code' cargo build --release --offline \
  --manifest-path /path/to/array-snapshot/build/Cargo.toml \
  --bin numpy-ekf-array-driver

RUSTFLAGS='-D warnings -A dead_code' cargo test --release --offline \
  --manifest-path /path/to/array-snapshot/build/Cargo.toml \
  --test batched_array_cpu
```

Substitute the printed snapshot path, then collect a pilot with two fresh
samples per tile at each batch size:

```sh
python benchmarks/numpy-ekf/run_array_comparison.py \
  --mode pilot --samples 2 --large-samples 2 \
  --mech-binary /path/to/array-snapshot/build/target/release/numpy-ekf-array-driver \
  --build-provenance /path/to/array-snapshot/build-provenance.json \
  --pilot benchmarks/numpy-ekf/results/apple-m1-2026-10-11-numpy-pilot/results.json \
  --output benchmarks/numpy-ekf/results/new-array-pilot
```

The collector sets NumPy and BLAS thread limits to one. Finish compilation
before measuring. The final collector verifies the selected tile, unchanged
NumPy sources, copied production source, patch, manifest, and binary:

```sh
python benchmarks/numpy-ekf/run_array_comparison.py \
  --mode final \
  --mech-binary /path/to/array-snapshot/build/target/release/numpy-ekf-array-driver \
  --build-provenance /path/to/array-snapshot/build-provenance.json \
  --pilot benchmarks/numpy-ekf/results/apple-m1-2026-10-11-numpy-pilot/results.json \
  --array-pilot benchmarks/numpy-ekf/results/new-array-pilot/results.json \
  --output benchmarks/numpy-ekf/results/new-array-final
```

Each run requires a new output directory. Raw reports include full validation
records, commands, all timing samples, machine/toolchain metadata, and
source/input/output/binary hashes, along with copied sources and build files.

To verify the current checkout rather than the pinned measurement snapshot,
generate its minimal test manifest and substitute the printed path:

```sh
python benchmarks/numpy-ekf/array-driver/prepare_current_tests.py

RUSTFLAGS='-D warnings -A dead_code' cargo check --tests --offline \
  --manifest-path /path/to/current-tests/Cargo.toml

RUSTFLAGS='-D warnings -A dead_code' cargo test --release --offline \
  --manifest-path /path/to/current-tests/Cargo.toml \
  --test batched_array_cpu
```
