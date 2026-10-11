# N-body: Zig and native compiler gaps, 2026-10-11

Mech improves from 1.054019 s to 1.039486 s, a 1.4% reduction, in the final
seven-run replication. A separate seven-run confirmation gives the same gain
(1.052555 s to 1.038264 s). Both use the unchanged 25-LOC pure Mech source.
The selected flags add `--simplify-square-seeds --schedule-regions 8` to its
existing FMA, algebraic-power and state-tail-packing options. Compact retains
its previous `--schedule-regions 8` choice and 20 LOC.

| Final matched program | Seconds | Time × Mech fast | LOC | LOC × Mech fast |
|---|---:|---:|---:|---:|
| C++ #6 | 0.899343 | 0.8652 | 202 | 8.08 |
| Chapel #3 | 0.970248 | 0.9334 | 79 | 3.16 |
| Zig #2 | 1.006515 | 0.9683 | 117 | 4.68 |
| Mech selected fast | 1.039486 | 1.0000 | 25 | 1.00 |
| Mech previous fast control | 1.054019 | 1.0140 | 25 | 1.00 |
| Rust #3 | 1.087958 | 1.0466 | 196 | 7.84 |
| Mech compact | 1.090478 | 1.0491 | 20 | 0.80 |

These are local Apple M3 process timings, including startup, initialization,
full computation, both energies and output. Compilation is excluded for all
AOT entries. AC power and Low Power Mode disabled are required. Full workloads
run sequentially, after validation/warmup, with shuffled rounds. Confirmation
and replication use different shuffle seeds; neither discarded any samples.

## What the other compilers do better

Zig #2 shares `displacement * magnitude` before weighting the two velocity
updates by their different masses. It uses ordinary square root/division, all
ten interactions, all three coordinates and all five position updates in
each of 50 million steps. Its four-lane vectors contain a zero padding lane;
this is SIMD layout, not an omitted physical coordinate.

C++ #6 and Chapel #3 retain Mech's displacement-times-mass-first order in
native code. They overlap those products with the expensive magnitude
dependency, and choose different scalar/vector grouping for reductions and
updates. Therefore shared products are not a universal explanation for their
speed advantage. The compiler must balance reused arithmetic against the
critical path and values occupying registers.

| Native step | Instructions | Stack loads | Stack stores | Fused instructions |
|---|---:|---:|---:|---:|
| C++ #6 | 172 | 11 | 2 | 52 |
| Chapel #3 | 204 | 25 | 9 | 44 |
| Zig #2 | 204 | 8 | 2 | 50 |
| Mech previous fast | 282 | 59 | 21 | 65 |
| Mech selected fast | 276 | 51 | 23 | 60 |

These are static instructions in each unrolled native step; a paired load/store
may transfer multiple values. Counts describe code generation, not a direct
cycle-cost attribution. C++/Chapel use four vector and two scalar sqrt/divide
instructions each; Zig uses ten scalar, Mech five vector. Each executes the ten
required distance calculations. Zig's more batched SIMD #3 is slower than #2.

## General experiments, including losses

`share_scaled_products()` is a new opt-in graph transformation, invoked before
FMA contraction with `--share-scaled-products`. It recognizes differently
constant-weighted products by SSA operands, proves at least two scaled producers
can disappear, and creates one shared variable product. It rebuilds registers
topologically, preserves SIMD consumer pairs and remaps all state/output roots.
There is no benchmark-name, body-count, runtime-input or iteration-count test.
This changes floating-point association/range and requires explicit relaxed
arithmetic; default compilation remains unchanged.

That pass cuts the fast loop to 244 instructions and 45 stack loads under
eight-instruction scheduling regions, but its confirmed time is 1.084063 s,
versus the unchanged control's 1.052555 s. Sharing adds a dependency after
magnitude becomes available. It is available experimentally but is not selected
for the benchmark. Pure Mech parentheses express the same formula without
calling a host implementation; they also fail to yield a confirmed gain.

A projected constant-register-pressure scheduler was tested on both Mech forms.
It did not improve them, so its implementation and flag were removed. Its
snapshot, pinned generation drivers and raw samples remain in this directory.
The earlier scheduler is restored exactly.

Square reduction seed simplification with eight-instruction regions does help.
It removes only a proved positive-zero square seed, preserves state bits and
retains subsequent summand order. The two seven-run cohorts' paired geometric
ratios are 0.9870 and 0.9859; within-cohort bootstrap intervals are respectively
[0.9812, 0.9917] and [0.9839, 0.9882]. These quantify this cohort's sampling,
not cross-machine performance or immunity to host-frequency effects.

## Validation and evidence

All 46 native tests pass: 32 fused-state tests and 14 map tests. New tests verify
shared-product arithmetic removal/idempotence, negative coefficients, unrelated
recurrences, scalar/SIMD agreement and perturbed initial states. Existing tests
cover complete state, signed zero, repeated selectors, termination and complete
Mandelbrot bitmaps. Selected scheduling/seed options preserve all 30 state bits
at 0, 1, 1,000 and 10,000 turns. Against the separate Rust state reference, the
largest scaled component error is 2.871e-12, below the 1e-10 audit limit.

`default-object-verification.json` proves the previous fast and compact native
objects are byte-identical with the new compiler when the new flag is absent.
`selected-object-verification.json` reproduces both final selected objects.
`summary.json` pins reports, compiler sources, source/binary provenance and audits.
`matched-mech.csv` includes standard deviations, raw seconds and both × Mech
metrics. All three pilots, 63 confirmation samples and 49 replication samples
are retained, including losing candidates. `native-audit.json` records the
pinned code-generation comparison and a C++ dependency trace.

Reproduce candidates using a new tag/output manifest with `prepare.py`. Re-run
the final cohort with `replicate.py --programs replication-programs.json
--samples 7 --output /tmp/nbody-fresh-replication.json`. Run builds/tests before
timings, and preserve old executables and report paths.
