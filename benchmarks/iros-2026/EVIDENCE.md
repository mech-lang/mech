# IROS workshop evidence ledger

This ledger maps the four workshop claims to reproducible repository evidence.
It separates results that are ready to publish from claims that still need an
implementation, an equal-workload run, or narrower wording.

The evidence branch is based on `origin/integration/v0.4`. Performance values
are descriptive results from the recorded Apple M1 campaigns, not universal
language rankings. Unless a row says otherwise, throughput summaries use the
median and median absolute deviation (MAD); MAD is a robust descriptive spread,
not a confidence interval.

## 1. Embeddable

### Publishable claim

A Rust application can compile a fixed-shape Mech kernel through
`mech::kernel`, declare live inputs and named state exports, then start a
persistent session and submit checked turns. Scalar JIT compiles once and reuses
its code across sessions. Scalar and four-lane AOT modes emit a reusable native
dynamic library. The [runnable Rust example](../../examples/embedded_ekf/main.rs)
and [interface documentation](../../docs/embedding-kernels.md) demonstrate this
path with the real EKF source; the embedding API's 21 integration tests pass.

The separate minimal benchmark loader calls the emitted library's C ABI with
contiguous buffers and executes checked turns without including the parser or
compiler. The source-compiling `mech::kernel` example includes those compilation
facilities. `Kernel::save_bundle` persists AOT code and versioned interface
metadata. `unsafe Kernel::load_bundle` loads a trusted, compatible saved bundle
without parsing, lowering, compiling, or linking. Tests verify relocation,
state/covariance agreement, rejected-turn rollback, corruption/mismatch errors,
code lifetime, and fresh-process loading without compiler/linker tools. The
current consumer Cargo feature still includes compiler dependencies.

Loading an arbitrary existing dylib without matching kernel metadata is not
part of that interface. A generated-kernel `rlib` emitter is also not implemented.
The new ergonomic bundle wrapper has not been performance-measured; the table
below retains the archived minimal-ABI loader results.

The current evidence does **not** show that arbitrary Rust data structures cross
the dynamic-library boundary unchanged. The measured ABI is a pointer table of
contiguous `f32` buffers plus an extent. Rust-native functions can separately be
installed into Mech through `FunctionCatalogBuilder` and
`MechFunctionFactory`; that is in-process extension, not the AOT dylib ABI.

### Measured AOT diagnostic

The common loader excludes compilation, linking, `dlopen`, allocation, input
construction, packing, warmup, reset, and reporting from the timed region. Each
row is one host thread, 10,000 filters x 200 checked turns, seven fresh
processes, with run order alternated by sample.

| Library | Throughput, median +/- MAD | Artifact size | Peak process RSS, median +/- MAD |
| --- | ---: | ---: | ---: |
| Mech scalar Cranelift AOT | 14.671 +/- 0.008 M turns/s | 33,544 B | 2,818,048 +/- 0 B |
| Optimized Rust `cdylib` | 21.121 +/- 0.005 M turns/s | 50,016 B | 2,818,048 +/- 0 B |
| Mech four-lane Cranelift AOT | 34.863 +/- 0.010 M turns/s | 33,864 B | 2,818,048 +/- 0 B |

The SIMD AOT artifact is 32.29% smaller than the Rust library and is 65.07%
faster in this diagnostic. This is not a matched language-speed result: Mech
uses packed cross-filter SIMD while the Rust dylib is a hand-specialized scalar
loop that LLVM locally vectorizes. The matched SIMD/eight-worker language result
belongs in the heterogeneous section. Median peak RSS is identical and the
observed ranges overlap, so this campaign establishes no memory-use difference.

Complete final states agree within 4.09e-4 after 200 turns and all samples
report zero faults. Scalar AOT and scalar JIT also have overlapping retained
five-process ranges, supporting equivalent steady-state execution. Cold scalar
AOT emit/link/load was 202.029 ms; cached load median was 3.283 ms.

### Exact artifacts

- Raw AOT/Rust samples, sizes, hashes, RSS, toolchains, and validation:
  `results/apple-m1-aot-vs-rust-dylib-2026-09-24.json`
- Scalar AOT/JIT and cold/cached load evidence:
  `results/apple-m1-mech-aot-2026-09-24.json`
- Common Rust loader and ABI: `rust-dylib/dylib-runner.rs`
- Rust control library: `rust-dylib/rust-ekf-dylib.rs`
- Mech scalar AOT implementation: `../../hosts/gpu/src/batched/aot.rs`
- Mech SIMD AOT implementation: `../../hosts/gpu/src/batched/simd_aot.rs`
- Public Rust interface: `../../hosts/gpu/src/embed.rs`, re-exported as `mech::kernel`
- Runnable consumer and kernel: `../../examples/embedded_ekf/main.rs` and `ekf.mec`
- Interface contract and test suite: `../../docs/embedding-kernels.md` and
  `../../tests/kernel_embedding.rs`
- Rust-to-Mech extension surface:
  `../../src/core/src/function/catalog.rs` and
  `../../src/core/src/function/mod.rs`

### Evidence still needed for the post

The previously missing minimal Rust consumer is implemented and tested. Its
example imports `mech::kernel::{Backend, Kernel}`, compiles source with named
inputs/exports, starts a resident session, submits a turn, and reads `state`.
It does not require the caller to construct pointer tables or invent artifact
metadata.

Remaining scope decisions and diagnostics:

1. Keep the source-compiling API example distinct from the standalone ABI
   loader when discussing binary dependencies and size.
2. Record `nm` exports and `otool -L` dependencies for both libraries. This is
   the direct proof for exported symbols and what is or is not self-contained.
3. Decide whether the post wants dynamic loading only or also static/object
   linking. The present implementation emits a PIC object and links a dylib;
   it does not expose a supported static-link packaging command.
4. The new same-workload campaign measures checked and unchecked Mech SIMD AOT.
   A matched Rust dylib checked/unchecked control is still a separate experiment;
   the retained Rust/Mech dylib campaign is checked only, n=7.
5. If memory is kept in the table, retain the present whole-process RSS label.
   A private-library memory claim would require a different metric.

## 2. Numerical

### Publishable claim

The Mech EKF is written as typed matrix equations close to textbook notation.
The same application source remains unchanged when the benchmark selects a
scalar, SIMD, multithreaded, AOT, WGPU, or Metal physical strategy. The compiler
has a typed dataflow graph and explicit state/integrity nodes, which is the
optimization authority used by the measured fixed-shape backends.

### Reproduced source-size evidence

The audit normalizes every programmer-chosen identifier occurrence to one code
point while retaining keywords, primitive types, library names, punctuation,
numbers, attributes, and literal contents. Comments and nonliteral whitespace
are excluded. The counter and all 17 tests pass on this branch.

| Implementation | Normalized source characters |
| --- | ---: |
| Mech textbook / backend-neutral | 1,079 |
| Rust textbook, publication-normalized | 2,355 |
| Mech SIMD-4 / eight workers | 1,079 |
| Rust SIMD-4 / eight workers | 5,243 |

The matched optimized Rust source is almost five times as long under this
normalized-character metric. Of the
2,888-character growth from textbook Rust to optimized Rust, SIMD wrappers and
adapters plus batch dispatch/workers/rollback account for 2,526 characters
(87.47%). This result describes these implementations; it is not a lower bound
for Rust.

The direct corrected-state/Joseph-covariance excerpt costs 38 normalized
characters in Mech and 240 in the selected textbook Rust extract. Rust helper
definitions are counted separately.

### Exact artifacts

- Audit method and provenance:
  `../archive/compute/parallel-ekf/source-size-audit/README.md`
- Four publication values:
  `../archive/compute/parallel-ekf/source-size-audit/counts.csv`
- Category proof:
  `../archive/compute/parallel-ekf/source-size-audit/proof/breakdown.csv`
- Counted Mech source:
  `../archive/compute/parallel-ekf/source-size-audit/selected/ekf.mec`
- Counted Rust sources:
  `../archive/compute/parallel-ekf/source-size-audit/selected/rust_textbook.rs`
  and `selected/rust_simd.rs`

### How to discuss linear algebra

Mention the matrix surface, typed shapes, and Rust implementation ecosystem,
but do not say the accelerated EKF result is simply a nalgebra benchmark.
Ordinary Mech matrix storage and operations use the Rust `nalgebra` dependency;
the measured fixed-shape compute path lowers the typed graph into its own scalar,
SIMD, Cranelift, WGSL, or MSL execution rather than calling nalgebra inside the
hot kernel. Future alternative matrix backends are a design opportunity, not a
measured result in this package.

### Evidence still needed for the post

1. Choose one short, real Mech equation excerpt and its matched Rust excerpt;
   preserve links to the full counted files rather than inventing poster code.
2. Export a compact typed-dataflow snapshot for that excerpt: state inputs,
   predictor, corrector, three integrity predicates, and atomic publication.
3. State the source-count boundary directly below the chart. Raw line counts
   (87 Mech, 196 textbook Rust, 401 SIMD Rust) may be shown only as secondary
   context because comments and formatting differ.

## 3. Reactive

### Publishable claim

A Mech turn is a bounded atomic reaction. An input change identifies dirty
reactive cells, deterministic dependency scheduling executes only the affected
graph, state/register writes are staged, integrity constraints decide whether
the candidate is accepted, and publication plus retained recording happens at
the commit boundary. A rejected candidate leaves the previous published state
live. External after-commit effects occur only after successful publication.

### Diagram evidence

The detailed post diagram should show this sequence:

```text
host input / timer / resource completion
                  |
                  v
        identify changed root cells
                  |
                  v
       deterministic dirty subgraph
                  |
                  v
     execute combinational nodes once
                  |
                  v
      stage registers and resident state
                  |
                  v
       evaluate integrity constraints
             /              \
          reject            accept
            |                  |
   discard candidate      atomic publish
   retain old state       + turn record
                               |
                               v
                    deliver after-commit effects
```

The archived resident-EKF qualification measured the same 4,096-turn episode
in ten samples. The complete source-artifact turn reported a 315.053 ns median
and 315.586 ns p95, zero steady-state allocations, 20 dirty nodes, one
publication store, and one retained record per turn. The source kernel alone
was 223.001 ns median; the bytecode kernel was 224.684 ns. Complete source and
bytecode turns measured 315.053 and 314.756 ns respectively, a 1.00094 ratio,
and both reproduced the reference trajectory hash. The complete turn remained
approximately history-independent at 0, 1,000, and 100,000 retained records.

These archived values explain the mechanism and its bounded overhead. They are
not the cross-language throughput result and should not share an axis with the
parallel f32 EKF charts.

### Exact artifacts

- Protocol and EKF contract: `../archive/runtime-gate-b/README.md`
- Raw n=10 turn evidence: `../archive/runtime-gate-b/b2-resident-turn.json`
- Reactive transaction implementation: `../../src/core/src/reactive_transaction.rs`
- Runtime commit/abort/effects:
  `../../src/runtime/src/runtime/transaction/`
- Memory staging requirements:
  `../../src/engine/src/memory_planner/turn.rs`

### Evidence still needed for the post

1. Render the detailed reaction diagram from the sequence above, with a small
   worked example showing which nodes stay clean after one input changes.
2. Include one rejected-turn trace proving old state retention and one accepted
   trace proving publication/record/effect order. The repository has tests for
   these behaviors; a compact machine-readable trace would make the figure
   independently inspectable.
3. Keep the reactive headline semantic. The 315 ns figure is supporting
   evidence that the semantics are implemented efficiently, not the definition
   of reactivity.

## 4. Heterogeneous

### Cross-language CPU figure: ready

The archived CPU comparison fixes 500,000 filters x 40 turns, f32 state,
four-wide SIMD where the system exposes it, eight workers, ten fresh processes
per row, deterministically shuffled run order, and no removed samples. The
selected Mech, Rust, Mojo, Julia, Futhark, and Numba rows fuse the 40-turn block;
the supplemental Taichi and Halide rows synchronize publication each turn.
The matched Mech/Rust pair uses block-atomic checked rollback.

Use these rows in the main cross-language figure: Mech, Rust, Mojo, Julia,
Futhark, Taichi, and NumPy/Numba. Halide is retained in the evidence package but
may be omitted from the simplified poster figure if visual density demands it.
The chart must label medians and MAD and must not rank gaps smaller than their
spread.

Exact evidence:

- `results/apple-m1-cpu-equal-n10-2026-09-24.json`
- `charts/post-cross-language-comparison.svg`
- `STATISTICS.md`

### Same-source/backend figure: integrated and measured

Revision `f4b69052cc6d1d618e1c80acac947795ab2e8472` contains the scalar evaluator,
scalar JIT, four-lane SIMD AOT, eight-worker SIMD JIT, and direct Metal paths.
The completed 2026-09-25 campaign compiles the same f32 EKF with seven live
inputs for every row. Each process runs five warmup turns and 40 timed turns in
the same resident session, using 500,000 filters and per-turn publication.
Ten shuffled rounds provide ten fresh processes for each backend/mode pair.

| Backend | Checked median ± MAD, M filter-turns/s | Unchecked median ± MAD, M filter-turns/s |
| --- | ---: | ---: |
| Evaluator, one worker | 0.695 ± 0.001 | 0.828 ± 0.001 |
| Scalar JIT, one worker | 13.224 ± 0.005 | 15.949 ± 0.033 |
| SIMD AOT, one worker | 34.617 ± 0.010 | 37.059 ± 0.085 |
| SIMD JIT, eight workers | 121.071 ± 0.528 | 141.645 ± 0.832 |
| Metal GPU | 420.896 ± 2.291 | 420.747 ± 2.283 |

All 100 measured processes reported zero faults. All ten 4,092-filter,
45-turn full-state preflights passed, as did all five checked NaN-rejection
tests. The completed-record audit passed and confirmed exact within-case
checksum and per-state-sum repeatability. Raw samples are retained without
outlier removal. See [methods and reproduction](MECH-BACKEND-PAIRS.md),
[raw evidence](results/apple-m1-mech-backend-pairs-n10-2026-09-25.json), and the
[read-only auditor](audit_mech_backend_pairs.py).

These pairs replace the mixed-workload Mech overview. Keep the archived
cross-language figure separate: its selected CPU rows use fused blocks, and
some older Mech kernels specialized four of the seven current live bindings
as constants. The new evaluator, scalar JIT, and SIMD JIT values cannot be
substituted into that historical comparison or treated as isolated regressions.

There is not yet an eight-worker SIMD AOT implementation. Do not label the AOT
row “8 core.” AOT and multithreading are independent properties.

### Configuration wording

The post can accurately draw:

```text
one .mec compute region -> typed Mech dataflow graph -> backend registry -> CPU/GPU session
```

Be precise about `.mcfg`: stable v0.4 product configuration currently exposes
`auto`, `cpu`, `gpu`, `cpu-scalar`, and `wgpu`. SIMD, JIT, AOT, and direct Metal
are benchmark/feature-enabled backends selected through the registry or direct
benchmark API; they are not all stable `.mcfg` product choices on this branch.

### Evidence still needed for the post

Integration and the five-backend, two-mode n=10 campaign are complete. Remaining
presentation/product scope:

1. Decide whether to productize SIMD/JIT/AOT/direct-Metal backend IDs in `.mcfg`.
   Until then, present backend selection as compiler/registry capability rather
   than claiming every measured backend is a stable configuration toggle.
2. Keep the Metal ecosystem comparison in the evidence archive, not the main
   four-story poster. It is useful generated-versus-hand-written backend
   evidence but is not needed for the requested cross-language CPU story.

## Reproduction status

Verified on 2026-09-25:

- `python3 benchmarks/iros-2026/verify.py`
- `python3 measure.py` in the source-size audit directory
- `python3 -m unittest -v test_measure` (17 tests)
- `cargo test --offline --profile kernel-bench --no-default-features --features kernel-benchmarks --test kernel_embedding -j 2`
  (12 tests)
- `python3 benchmarks/iros-2026/audit_mech_backend_pairs.py --input benchmarks/iros-2026/results/apple-m1-mech-backend-pairs-n10-2026-09-25.json`
  (100 measured samples, ten passing preflights, five passing NaN rollback tests)

The first command verifies the result hashes, raw sample contracts, source-size
values, AOT sizes/RSS, and chart statistics. No source-size audit assertion or
IROS evidence check failed.
