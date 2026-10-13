# Mech optimization history

This branch preserves how Mech improved during the Benchmarks Game work: the idea, the general language/compiler/runtime change, its measured effect, and the evidence. The [structured ledger](mech-optimization-history.json) records exact medians and stable study IDs for later summaries. Raw historical reports remain authoritative.

All measurements below are from the Apple M3 studies linked with each entry. Each before/after belongs to its own cohort. Do not multiply gains across cohorts or compare resident execution after loading with fresh-process JIT/AOT time. “Speedup” means before divided by after; a value below 1 is a regression. Containing commits collect work and do not always identify its exact implementation date.

## Examples of ideas moving into Mech

- **Rust to general streaming:** FASTA submission #7 suggested period preformatting and a bounded producer/mapper pipeline. The unchanged compact Mech source measured 3.009813 → 1.823449 seconds (1.65×); expanded source measured 1.236767 → 0.964124 seconds (1.28×). These are historical output-equivalent experiments. The subsequent required-work audit disabled integer-cutoff lowering for strict FASTA; fixed-string caching and the pipeline remained disclosed.
- **Chapel to source organization:** Pair updates made temporary lifetimes and shared products explicit in Mech source: 2.028586 → 1.983660 seconds (1.02×) in the historical five-sample cohort. This was a source rewrite with validated arithmetic differences.
- **LLVM/Rust to reusable numeric machinery:** Matrix work led to typed graph simplification, row tiling, range proofs and induction experiments. Profitability depended on the unroll/worker policy; the fastest measured option left induction off.
- **Zig to an experiment we rejected:** Sharing Nbody products reduced instruction count but slowed execution. We retained the separate seed/scheduling improvement and kept the losing evidence.

The user remembers another Mojo-inspired improvement on another computer. The accessible inspected chats document Mojo comparison ports, but do not identify that transfer. Attribution remains open in the ledger; it can be completed when the matching conversation or compiler receipt is recovered.

## Dated changes and measured effects

### 2026-10-07 — Native f64 lowering, SIMD and explicit floating-point policies

**Change:** Lower fixed-shape ranges, gathers, combinations, reductions and updates to native loops; pack coordinates and preserve reduction order; expose FMA and algebraic powers explicitly.

**Origin and scope:** Scalar AOT backport from iros-workshop-benchmarks; SIMD/code-generation investigation against native submissions. Change kind: compiler and opt-in policy.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| Strict scalar to strict SIMD | 2.420990 | 2.166538 | 1.117× |
| SIMD to explicit FMA | 2.166538 | 2.029314 | 1.068× |
| Power source: strict to algebraic powers plus FMA | 9.264462 | 2.030818 | 4.562× |

**Evidence limits:** Historical cohort: do not join its absolute times to later AC-power cohorts. FMA and algebraic powers change arithmetic policy. Refined reciprocal variants lost; no universal fast-math claim. External scalar backport commit b10593b75ec8b9283ae33e4ac330c00dda9957d7 is a dependency provenance reference, not a Mech commit.

**Receipts:** [nbody/results/apple-m3-native-output-ablation-50m.json](nbody/results/apple-m3-native-output-ablation-50m.json), [nbody/results/apple-m3-v04-native-optimized-50m.json](nbody/results/apple-m3-v04-native-optimized-50m.json).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-07 — Chapel-inspired pair update organization

**Change:** Share displacement times magnitude and interleave both velocity contributions per pair in counted Mech source.

**Origin and scope:** Chapel pair updates and temporary lifetime organization; accessible chat turn 01a11604-c7ef-7bf3-ad83-fbe5b5ba30c5. Change kind: source rewrite.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| Original FMA source to pairwise FMA source | 2.028586 | 1.983660 | 1.023× |

**Evidence limits:** Changes accumulation order; passes energy checks and 28 full-state comparisons with documented tolerances. Depth scheduling emitted identical hot-loop code and showed no established benefit.

**Receipts:** [nbody/results/apple-m3-chapel-tricks-50m.json](nbody/results/apple-m3-chapel-tricks-50m.json), [nbody/results/apple-m3-chapel-tricks-50m.table.md](nbody/results/apple-m3-chapel-tricks-50m.table.md).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-07 — Reusable matrix optimizations and profitability experiments

**Change:** Common-subexpression DAGs, hoisting, range-proved bounds removal, degree-two induction, f64x2 unrolling, row tiling, exclusive buffer reuse and smaller parallel chunks.

**Origin and scope:** Rust/LLVM row tiling, work partitioning, polynomial induction and integer denominator techniques; spectralnorm README documents the transfer. Change kind: compiler, source and worker policy.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| One-worker integer-coefficient diagnostic | 0.808000 | 0.638000 | 1.266× |

**Evidence limits:** 0.808 to 0.638 seconds is a diagnostic source-plus-compiler comparison with counters, not an isolated compiler-pass speedup. Induction helped slightly at unroll two and lost at unroll four; fastest selected variant left induction off.

**Receipts:** [spectralnorm/README.md](spectralnorm/README.md), [spectralnorm/results/apple-m3-optimized-5500.json](spectralnorm/results/apple-m3-optimized-5500.json), [spectralnorm/results/induction-worker-counters.json](spectralnorm/results/induction-worker-counters.json), [spectralnorm/results/induction-paired-counters.json](spectralnorm/results/induction-paired-counters.json), [spectralnorm/results/integer-coefficient-ablation.json](spectralnorm/results/integer-coefficient-ablation.json).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-08 — Byte fusion, SIMD and ownership-safe storage reuse

**Change:** Fuse reverse/translate, prove 256-entry byte lookup for SIMD, reuse last-use buffers only under exclusive ownership, partition disjoint pairs and bound wrapping copies.

**Origin and scope:** General single-consumer fusion, ownership and disjoint-range reasoning. Change kind: compiler and runtime.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| Preserved implementation to combined optimized implementation | 0.353965 | 0.217914 | 1.624× |
| Same current source: fusion off to on | 0.226009 | 0.217914 | 1.037× |
| Same current source: copying to ownership-safe reuse | 0.244865 | 0.217914 | 1.124× |

**Evidence limits:** The preserved first implementation uses per-line input and library formatting; its 0.354 to 0.218 comparison also changes input/storage organization. It is not an isolated same-source compiler-pass gain. Same-current-source ablations separate SIMD, fusion, reuse and partitions; effects are not additive.

**Receipts:** [revcomp/results/paired-optimizations.json](revcomp/results/paired-optimizations.json), [revcomp/README.md](revcomp/README.md).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-08 — Rust-inspired periodic streaming and exact arithmetic lowering

**Change:** Prove bounded arithmetic and rounded comparison thresholds from source; replay a source-proved literal period; use a bounded two-buffer producer/mapper/output pipeline.

**Origin and scope:** Rust submission #7 period preformatting and producer/mapper organization, documented in fasta README. Change kind: compiler and generic runtime.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| Compact exact-arithmetic stage | 3.006139 | 2.278654 | 1.319× |
| Expanded exact-arithmetic stage | 1.235646 | 1.119752 | 1.103× |
| Compact combined final implementation | 3.009813 | 1.823449 | 1.651× |
| Expanded combined final implementation | 1.236767 | 0.964124 | 1.283× |
| Expanded pipeline ablation | 1.044532 | 0.964124 | 1.083× |

**Evidence limits:** These October 8 results predate the October 9 strict required-work correction, which disabled floating-CDF-to-integer-cutoff lowering. They are historical output-equivalent experiments, not current strict qualification. Corrected strict sources retain 200,000,000 fresh LCG updates and floating comparisons. Counted Mech sources remain unchanged within each matched comparison. The final result combines several changes; most expanded arithmetic gain comes from bounded-index simplification. Period and pipeline effects cannot be added to other cohorts. Full 32-bit narrowing lost and was not selected.

**Receipts:** [fasta/results/apple-m3-exact-compare-matched-25000000.json](fasta/results/apple-m3-exact-compare-matched-25000000.json), [fasta/results/apple-m3-streaming-final-25000000.json](fasta/results/apple-m3-streaming-final-25000000.json), [fasta/README.md](fasta/README.md).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-08 — General allocation lifetimes, record layouts and recursive inlining

**Change:** Use planned allocation lifetimes and record layout authority instead of per-node Box plus ownership bookkeeping; tune recursive inline budget.

**Origin and scope:** Ordinary arena/frame lifetime and recursion support; whole benchmark algorithm remains in Mech. Change kind: compiler and runtime.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| Recursive inline budget zero to two | 0.950268 | 0.723920 | 1.313× |

**Evidence limits:** Inline-budget comparison uses the same historical 50-line source. Later 50-to-29-line record/match/comprehension redesign is a separate source change. Abandoned allocation study is retained.

**Receipts:** [binarytrees/results/apple-m3-21.json](binarytrees/results/apple-m3-21.json), [binarytrees/README.md](binarytrees/README.md), [binarytrees/results/apple-m3-individual-box-abandoned.json](binarytrees/results/apple-m3-individual-box-abandoned.json).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-09 — Canonical broadcasts, packed state tails and product reassociation

**Change:** Canonicalize single-input shuffles; optionally pack odd state tails; opt into constant-product reassociation. A separate source puts mass multiplication earlier.

**Origin and scope:** LLVM-style broadcast/lane reasoning and native code inspection. Change kind: compiler plus separate source rewrite.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| Unchanged compact source | 1.168187 | 1.121837 | 1.041× |
| Mass-first source plus backend changes | 1.129855 | 1.053052 | 1.073× |

**Evidence limits:** Compact comparison keeps its source. The mass-first fast comparison changes source as well as backend policies; attribute the combined effect accordingly.

**Receipts:** [nbody/experiments/20261009-local-simd/final-confirmation-50m.json](nbody/experiments/20261009-local-simd/final-confirmation-50m.json), [nbody/experiments/20261009-local-simd/post-validation.json](nbody/experiments/20261009-local-simd/post-validation.json), [nbody/experiments/20261009-local-simd/mandelbrot-regression.json](nbody/experiments/20261009-local-simd/mandelbrot-regression.json).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-09 — Source compaction and ordinary function application

**Change:** Reduce source scaffolding; preserve evaluation-once application semantics and native kernels.

**Origin and scope:** Express the same work through canonical Mech forms; general transpose permutation and numeric suffix lowering. Change kind: language syntax and source rewrite.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| Spectral source expression rewrite (regression) | 0.084689 | 0.088730 | 0.954× |

**Evidence limits:** Nbody compaction 32/37 to 20/25 lines preserved object bytes. Application migration preserved 14 non-spectral compute objects and passed 174 comparisons across 81 input cases. Small timing changes do not establish a speedup. The nine-line spectral source adds intermediate reciprocal rounding and multiplication. It was slower in nine of ten paired rounds; the faithful twelve-line form was restored. Application syntax itself is not the demonstrated cause.

**Receipts:** [Nbody source compaction](nbody/experiments/20261009-source-compact/source-compaction.json), [strict source correction](work-audit/20261009/audit.json), [function-application-audit/20261009T171437Z/RESULTS.md](function-application-audit/20261009T171437Z/RESULTS.md), [function-application-audit/20261009T171437Z/application-v3/verification.json](function-application-audit/20261009T171437Z/application-v3/verification.json), [function-application-audit/20261009T171437Z/application-v3/spectral-confirmation-10.json](function-application-audit/20261009T171437Z/application-v3/spectral-confirmation-10.json).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-10 — Shared admission, JIT reuse and typed resident interpretation

**Change:** Share parsed documents between frontends, reparse only changed text, reuse admitted control IR, release parse storage; interpret independent coefficients in typed tiles of at most 32 independent elements/coefficient calls with whole-invocation unsigned proofs and checked fallback; inline count dispatch.

**Origin and scope:** General dependency/dispatch investigation; reusable coefficient batches rather than a cached matrix formula. Change kind: compiler and interpreter runtime.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| Resident spectral norm | 5.184886 | 0.457014 | 11.345× |
| JIT Pi digits | 0.719950 | 0.447466 | 1.609× |
| JIT FASTA | 1.126172 | 0.765849 | 1.470× |
| JIT binary trees | 0.733148 | 0.525069 | 1.396× |
| JIT Mandelbrot | 1.066208 | 0.783310 | 1.361× |
| JIT fannkuch | 2.476235 | 2.020614 | 1.225× |
| JIT Nbody | 1.628762 | 1.380457 | 1.180× |
| Resident fannkuch (regression) | 59.393436 | 61.607569 | 0.964× |

**Evidence limits:** Three randomized rounds: 135 full-workload samples, 54 semantic checks and fifteen identical AOT objects. Fastest measured source is chosen independently per execution path; this is a combined change. Resident timing begins after loading/activation; JIT uses whole fresh-process time including compilation. Do not compare these boundaries as a single pass. Resident fannkuch regressed 3.7%. Before and after are separate completed cohorts with fresh AOT controls, not one randomized old/new paired batch; the AOT control geometric-mean ratio was 0.999029530.

**Receipts:** [execution-modes/20261010-final/README.md](execution-modes/20261010-final/README.md), [execution-modes/20261010-final/summary.json](execution-modes/20261010-final/summary.json), [execution-modes/20261010-final/timings.json](execution-modes/20261010-final/timings.json), [execution-modes/20261010-final/snapshots.json](execution-modes/20261010-final/snapshots.json).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-11 — Consumer grouping and register-pressure scheduling

**Change:** Schedule generic native graph regions and simplify the square accumulation seed.

**Origin and scope:** LLVM consumer grouping and C++/Chapel temporary lifetime/register-pressure inspection. Change kind: compiler policy.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| Compact region-eight scheduling | 1.119698 | 1.089513 | 1.028× |
| Fast-source control | 1.046597 | 1.047539 | 0.999× |

**Evidence limits:** Unchanged 20-line compact source; all thirty state bits preserved in the recorded comparisons. Fast source fresh control showed no gain.

**Receipts:** [nbody/experiments/20261011-competitor-techniques/confirmation-50m.json](nbody/experiments/20261011-competitor-techniques/confirmation-50m.json), [nbody/experiments/20261011-competitor-techniques/README.md](nbody/experiments/20261011-competitor-techniques/README.md), [nbody/experiments/20261011-competitor-techniques/final-object-verification.json](nbody/experiments/20261011-competitor-techniques/final-object-verification.json).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-11 — Rejected shared products; retained seed and schedule policy

**Change:** Investigate arithmetic sharing and compare it independently with square-seed and region-eight scheduling.

**Origin and scope:** Zig-style shared displacement-times-magnitude products. Change kind: compiler opt-in experiment and selected policy.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| Shared-product experiment (regression) | 1.052555 | 1.084063 | 0.971× |
| Seed plus region-eight confirmation | 1.052555 | 1.038264 | 1.014× |
| Independent seed plus region-eight replication | 1.054019 | 1.039486 | 1.014× |

**Evidence limits:** Sharing reduced static instructions from 282 to 244 but made the recorded run slower. The winning change was seed simplification plus region-eight scheduling, not the shared-product pass. Source remains unchanged.

**Receipts:** [nbody/experiments/20261011-shared-products/confirmation-50m.json](nbody/experiments/20261011-shared-products/confirmation-50m.json), [nbody/experiments/20261011-shared-products/replication-50m.json](nbody/experiments/20261011-shared-products/replication-50m.json), [nbody/experiments/20261011-shared-products/README.md](nbody/experiments/20261011-shared-products/README.md), [nbody/experiments/20261011-shared-products/native-audit.json](nbody/experiments/20261011-shared-products/native-audit.json).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-11 — Existing region-size policy beats scheduling prototypes

**Change:** Select the already-existing region-24 policy and restore all five experimental compiler edits.

**Origin and scope:** Dependency/scheduling exploration following native submission audits. Change kind: policy selection; no production compiler change.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| Region eight to existing region twenty-four | 1.041305 | 1.039142 | 1.002× |
| Independent replication | 1.041377 | 1.039063 | 1.002× |

**Evidence limits:** Two fifteen-round cohorts show about 0.2% lower elapsed time. Production compiler and source were unchanged; this is policy tuning, not a new compiler pass.

**Receipts:** [nbody/experiments/20261011-scheduling/confirmation-50m.json](nbody/experiments/20261011-scheduling/confirmation-50m.json), [nbody/experiments/20261011-scheduling/replication-50m.json](nbody/experiments/20261011-scheduling/replication-50m.json), [nbody/experiments/20261011-scheduling/README.md](nbody/experiments/20261011-scheduling/README.md), [nbody/experiments/20261011-scheduling/restored-source-verification.json](nbody/experiments/20261011-scheduling/restored-source-verification.json).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-07/09 — General count, bigint and regex operations

**Change:** Optimize immutable dependency scheduling and packed extraction; use general counting containers and partitions, reusable GMP cells/direct quotient operations and generic byte-regex operations.

**Origin and scope:** Reuse general Rust/native libraries as intended by Mech; packed-key and counting techniques compared with Rust/Julia/Chapel. Change kind: library/runtime and separate source variants.

**Evidence limits:** No isolated before/after speedup is asserted here without a matched receipt. The withdrawn eight-line k-nucleotide host shortcut was replaced by counted 17/24-line Mech bodies. Mech uses Rust `regex::bytes` (regex 1.13.1) through general library operations; that timing does not measure a Mech-generated regex engine.

**Receipts:** [knucleotide/README.md](knucleotide/README.md), [pidigits/README.md](pidigits/README.md), [regexredux/README.md](regexredux/README.md).

**Containing commits:** `bf9d7f2b6`.

### 2026-10-11 — Ordinary array state forms and arrow mutation correctness

**Change:** Reduce Mandelbrot source state scaffolding from 20 to 11 and 29 to 16 lines; preserve mutation and return the updated state from arrow bodies.

**Origin and scope:** Uiua array/state comparisons prompted exploration of ordinary Mech array forms. Change kind: source expressiveness and correctness.

**Evidence limits:** Several source/execution variants were compared. A byte-identical native object for the equivalent row form does not establish a compiler speedup. Arrow mutation repair is a correctness fix; do not advertise an unmeasured runtime gain.

**Receipts:** [mandelbrot/README.md](mandelbrot/README.md).

**Containing commits:** `005d983df`, `c3396db09`.

### 2026-10-11 — Batched array interpreter integration and a retained regression

**Change:** Integrate the batched interpreter while preserving the existing f64/control/collection dispatch.

**Origin and scope:** General f32 array execution support. Change kind: capability and interpreter integration.

| Matched comparison | Before (s) | After (s) | Speedup |
| --- | ---: | ---: | ---: |
| Default resident fannkuch confirmation | 56.117233 | 59.300908 | 0.946× |

**Evidence limits:** Ninety full runs preserve capability evidence. Same-package static control excludes driver-package identity as the explanation; cause of the regression is not isolated.

**Receipts:** [interpreter-integration/20261011/README.md](interpreter-integration/20261011/README.md), [interpreter-integration/20261011/fannkuch-confirmation.json](interpreter-integration/20261011/fannkuch-confirmation.json), [interpreter-integration/20261011/package-control.json](interpreter-integration/20261011/package-control.json).

**Containing commits:** `45c6837ab`, `dac4c942b`.

### 2026-10-11/12 — Language gap repairs and pending algebraic floating point

**Change:** Add canonical form support and audit an explicit-f64 algebraic prototype on bounded semantic cases.

**Origin and scope:** General tuple/arrow/frame/bigint support; explicit floating-point policy investigation. Change kind: correctness/capability and bounded experiment.

**Evidence limits:** Language-gap study preserved fifteen prior native objects; multiple changes prevent isolated timing attribution. Algebraic prototype has zero qualified full timing runs: strict policy changed zero compared bits, broader policies changed 67 rows per copy, including special values. No claimed codegen or full-workload speedup; production resident policy unchanged.

**Receipts:** [language-gaps/20261011/results/summary.json](language-gaps/20261011/results/summary.json), [interpreter-algebraic/20261012/revisions/explicit-f64-v5/preparation/report.json](interpreter-algebraic/20261012/revisions/explicit-f64-v5/preparation/report.json), [interpreter-algebraic/20261012/revisions/explicit-f64-v5/review-evidence-v1/report.json](interpreter-algebraic/20261012/revisions/explicit-f64-v5/review-evidence-v1/report.json).

**Containing commits:** `c3396db09`, `dac4c942b`.

### 2026-10-09 — Required-work correction and transitive scalar helpers

The compiler follows pure transitive scalar helpers inside matrix comprehensions and rejects impurity/recursion. The strict spectral source restores all four required functions. Strict FASTA disables floating-CDF-to-integer-cutoff lowering, executes every fresh LCG update and retains floating comparisons; fixed ALU string caching and pipeline behavior remain disclosed.

All 51 native-sequence tests passed, and rebuilding 13 unchanged programs preserved their native compute objects. Five-sample full medians for the corrected endpoints were spectral 0.088514667 s, compact FASTA 1.227297416 s and expanded FASTA 0.573187541 s. These are corrected strict endpoints; they do not isolate a pass speedup. [Required-work audit](work-audit/20261009/README.md), [build identities](work-audit/20261009/builds.json) and [raw timings](work-audit/20261009/timings.json) preserve the distinction. Containing commit: `bf9d7f2b6`.

### 2026-10-07 — Mandelbrot masks and recurrence organization

Chapel's eight-pixel organization and five recurrence steps per escape check, plus Rust #4's Boolean-mask/integer arithmetic investigation, informed the Mech source and native mask work. Coordinate FMA changed one N=200 pixel and was rejected; recurrence-only contraction preserved the recorded output checks. Source/grouping/worker forms have separate measurements, so this entry makes no isolated-pass timing claim. [Mandelbrot evidence](mandelbrot/README.md) remains authoritative. Containing commit: `bf9d7f2b6`.

## Maintaining this history

After each Mech optimization experiment, append an entry to the ledger and this document with the source/compiler/library/policy scope, inspiration, unchanged-work validation, exact before/after cohort, timing boundary and receipt paths. Record rejected ideas and regressions as well as selected changes. If there is no matched timing, say so. Preserve the old inputs, sources and reports. Commit and push each completed checkpoint so a later summary can be recovered from the branch rather than conversational memory.

Benchmark continuation remains in [the recovery checkpoint](continuation/root-recovery-20261012.json). Pending language suites and Julia AOT qualification are separate from the measured Mech history above.
