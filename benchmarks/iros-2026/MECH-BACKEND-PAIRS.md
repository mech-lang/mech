# Matched Mech backend comparison

This completed campaign measures five Mech backends in checked and unchecked modes. Each backend occupies one chart row with two adjacent bars. The [raw evidence](results/apple-m1-mech-backend-pairs-n10-2026-09-25.json) contains 100 successful measured processes and ten passing preflights, collected on 2026-09-25 from source revision `f4b69052cc6d1d618e1c80acac947795ab2e8472`.

## Results

Apple M1, macOS 15.6.1; 500,000 filters, five warmup turns followed by 40 timed turns in the same session; ten fresh processes per backend/mode. Values are median ± unscaled MAD, in million filter-turns/s.

| Backend | Checked | Unchecked |
| --- | ---: | ---: |
| Evaluator, one worker | 0.695 ± 0.001 | 0.828 ± 0.001 |
| Scalar JIT, one worker | 13.224 ± 0.005 | 15.949 ± 0.033 |
| SIMD AOT, one worker | 34.617 ± 0.010 | 37.059 ± 0.085 |
| SIMD JIT, eight workers | 121.071 ± 0.528 | 141.645 ± 0.832 |
| Metal GPU | 420.896 ± 2.291 | 420.747 ± 2.283 |

All measured processes reported zero faults. The ten numerical preflights passed; the largest absolute component error was `8.392334e-5`. All five checked backends rejected the injected NaN without changing published state. The completed-record audit verified sample counts, timing arithmetic, summaries, source hashes, and per-mode AOT identity. Per-state sums and total checksums repeated exactly within each case and agreed between checked and unchecked modes within each backend. These aggregate checks supplement the component-wise preflight; they do not prove full-state equality for the measured population.

The results describe these backends and this workload, not a ranking of programming languages. The Metal checked/unchecked median difference is smaller than either MAD; this campaign does not resolve a small checking cost on that backend.

## Workload and execution

Every case compiles the same bearing-only, f32 EKF in [`examples/embedded_ekf/ekf.mec`](../../examples/embedded_ekf/ekf.mec). The default batch contains 500,000 independent filters. Rust generates the same f32 input arrays for every case. Seven source bindings are live inputs: `dt`, `linear-velocity`, `angular-velocity`, `bearing`, `measurement-noise`, `finite-limit`, and `covariance-symmetry-tolerance`. Velocity and bearing vary across filters; the other four inputs are broadcast scalars. Inputs remain resident and unchanged during the timed interval.

| Chart row | Numerical execution | Parallelism |
| --- | --- | --- |
| Evaluator | Scalar interpretation of lowered fixed-shape instructions | One CPU worker |
| Scalar JIT | Cranelift-generated scalar machine code | One CPU worker |
| SIMD AOT | Cranelift-generated four-lane native shared library | One CPU worker |
| SIMD JIT 8w | Cranelift-generated four-lane machine code | Eight persistent CPU workers |
| Metal GPU | Generated component-major shader, compiled through Naga to Metal | GPU, 64-thread groups |

The evaluator is the fixed-shape numerical instruction evaluator, not the general source-language interpreter. Its Rust dispatch loop is optimized, while numerical instructions are interpreted at runtime. The compiled backends execute specialized native code or a generated GPU shader. These are intentionally different execution strategies for the same kernel.

Checked execution retains three source predicates: finite candidate state, positive covariance diagonal, and covariance symmetry. Unchecked execution explicitly removes only those three named predicates and instructions used exclusively by them. State-update dependencies remain. Every case retains double-buffered state and a publication boundary after each turn. Checked failure preserves the previously published state for the entire batch. GPU command completion is awaited in both modes. SIMD JIT synchronizes its workers each turn; it does not use fused-block publication.

Both modes use the same numerical optimizations within a backend. In particular, Metal uses explicit FMA and eliminates factors of `1` and `-1`; it retains zero products so that non-finite values cannot be hidden by zero-factor elimination. Backend-specific floating-point evaluation can produce small differences, which the preflight checks below bound.

## Timing and sampling

Each fresh process prepares one backend and mode, performs five untimed warmup turns, then immediately times 40 additional turns in the same session. There is no reset, reallocation, or recompilation between warmup and measurement. The reported work is 20 million filter-turns; the final state is after 45 turns. Compilation, AOT emission/loading, allocation, input packing, worker startup, warmup, final state readback, and checksums are outside the timed interval.

The collector executes all ten backend/mode combinations once per round, sequentially, in a deterministic shuffled order. Ten rounds give ten fresh-process samples per case. Report median throughput plus the **unscaled median absolute deviation**, `median(abs(sample - median))`, in million filter-turns/s. MAD describes observed variability, not a confidence interval. No samples are removed as outliers. Preflight processes are excluded from these summaries; failed samples remain in the evidence and stop the campaign.

## Validation and provenance

Before measurement, every backend/mode is checked against scalar checked execution for 45 turns on up to 4,092 filters. This count exercises an incomplete Metal thread group and unequal eight-worker partitions. Every state and covariance component is compared using `abs(error) <= 2e-4 + 1e-5 * abs(reference)`, with finite values required. Each checked backend also receives a NaN bearing in the last lane. The test requires an integrity error attributed to that lane and turn, one recorded fault, and no change to any published state.

The [embedding API](../../docs/embedding-kernels.md) and [runnable Rust example](../../examples/embedded_ekf/main.rs) use `mech::kernel` to compile source, start a persistent session, submit checked turns, and read exported state by name. Its 12-test suite has passed separately. That validates the public interface; it is not throughput evidence for this campaign. Re-run it with:

```sh
cargo test --no-default-features --features kernel-aot --test kernel_embedding
```

The JSON preserves raw process output, elapsed times, checksums, validation details, the randomized schedule, and failures. Provenance includes machine/OS information, Rust and LLVM version output, Cargo and C compiler versions, source/manifests and their hashes, Git revision plus working changes, executable SHA-256, and each loaded AOT library's hash and size. Use those recorded identities when citing the measurements, including changes beyond the recorded Git HEAD if the worktree was dirty.

## Reproduction

Run on macOS with Metal support and a native linker toolchain, from the repository root:

```sh
CARGO_BUILD_JOBS=2 cargo build --profile kernel-bench --no-default-features \
  --features kernel-benchmarks --example mech_backend_pairs

MECH_AOT_CACHE_DIR=target/mech-aot-backend-pairs \
python3 benchmarks/iros-2026/measure_mech_backend_pairs.py \
  --binary target/kernel-bench/examples/mech_backend_pairs \
  --output benchmarks/iros-2026/results/mech-backend-pairs-n10.json \
  --samples 10 --instances 500000 --turns 40 --seed 20260925
```

The collector refuses an existing output path. Choose a new filename for a new campaign. It builds nothing during measurement and constrains incidental numerical-library thread pools to one thread; the SIMD JIT worker count is explicitly eight. Avoid concurrent builds or other heavy work during collection.

Audit the retained completed record without rerunning benchmarks:

```sh
python3 benchmarks/iros-2026/audit_mech_backend_pairs.py \
  --input benchmarks/iros-2026/results/apple-m1-mech-backend-pairs-n10-2026-09-25.json
```

The custom `kernel-bench` profile inherits `dev`. It uses optimization level 3 for `mech`, `mech-gpu`, `mech-compute`, and `wide`; other packages remain at level 0. Debug information and incremental compilation are disabled, and symbols are stripped. Dev-profile debug assertions and overflow checks remain enabled. This optimizes the timed numerical executors without optimizing the large source compiler/catalog. Cranelift and Metal compile their generated code separately, outside the timed interval. Do not describe this profile as a uniform release build.

## Relation to the poster's other evidence

The left cross-language CPU chart is an archived, fused-execution comparison for its selected Mech/Rust/Mojo/Julia/Futhark/Numba rows. This new right-hand Mech chart measures per-turn publication, a common batch size, and seven live source inputs on the integrated implementation. Some archived CPU kernels specialized four of those bindings as constants. The new evaluator, scalar JIT, and SIMD JIT rates are therefore not direct updates of those older rates. Keep the campaigns' statistics and source identities separate; equal units do not make their throughput values interchangeable. The completed backend pairs replace the old mixed-workload Mech overview. The checked-only, n=7 Rust/Mech dylib experiment remains a separate size, memory, and code-generation diagnostic.
