# Shared action-output inventory during native preparation

Reusing the complete sorted action-output inventory for one immutable plan
improves level-12 Deflate source-to-executable compilation **16.032216 to
15.689968 seconds (2.13%)**, with both balanced blocks faster and every
executable byte identical. Native preparation falls **20.48%**, and compiler
instructions fall **2.03%**. The change is retained. Compilation below ten
seconds remains open.

Qualified source `f077bec331089ff1dfc2eb28d876ccaa17dec92b` has exactly the
measured three-file diff from `08019220f0235db8424d1644dbd729ce2f2b6e7b`:
SHA-256 `d5d27665d67a1ef63856d7672c32170fc7187e712642751bf9204f1cf165dc4d`.
The source commit precedes this evidence publication. Its entire diff includes
the two production files and focused-test changes, with source snapshots and
the commit comparison retained.

## Removed work

Each numeric phase previously traverses every plan block and constructs a
fresh sorted map of outputs from scalar Copy/Primitive, structural and library
actions. `NativeProgramFacts` now lazily builds that same complete map once
for its immutable plan borrow and retains its sorted entries in a boxed slice.
Each phase borrows the slice. Original ordering, duplicate-key replacement
semantics and sealed action values remain.

The first inventory demand remains after the original header-liveness query
and scope collection. Each phase still traverses the complete output list and
runs its original live-value, scope, scalar/frame/aggregate, definition and
read checks in order. Existing whole-inventory validation and first refusal
remain at their original definition/read queries. The shared inventory grants
no validation or execution authority; shape, factory, current-visit, ownership
and finite admission/accounting paths retain their existing checks.

The preceding current-compiler stack diagnostics are published in
[packet 37](native-current-parallel-compile-profile.md). Deeper caller paths
place sampled ordered-map insertion in preparation's `fresh_values`, which
motivates this trial. An earlier source-lowering scalar-length attribution was
incorrect. The measured work and instruction reductions establish this change's
benefit; waiting and inclusive stack counts are not CPU shares.

## Complete-build qualification

Both subjects use the retained pinned compiler allocator, standard ThinLTO
compiler profile, current Deflate source/runtime, the same 61 program flags and
two workers. The immutable control compiler SHA-256 begins `c5b0`; the candidate
begins `9576`. PGO and native-CPU flags are absent.

Eight primary complete builds run serially in ABBA/BAAB order without overlapping
heavy build, test, sampling, hashing or archival. Each includes fresh source
assembly, parsing, checked frontend, native preparation, optimization and code
generation, Object/metadata writes, Object handoff and native link completion.
The fixed runtime/entry adapter and native inputs are cached and hash-pinned.
Its historical first-use construction of 11.882560208 seconds is excluded.

| Boundary | Control | Shared inventory | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 16.032216 s | 15.689968 s | -2.13% |
| Source-to-Object process within those builds | 15.902663 s | 15.555078 s | -2.19% |
| Final native link | 0.128585 s | 0.133885 s | +4.12% |

The two complete-build blocks improve 1.38% and 2.87%. Every observation remains,
including the first pair where the candidate is about 0.0046 seconds slower.
Object handoff increases 3.86%, about 0.037 ms. Link/handoff movement is
observational; their code and fixed inputs are unchanged.

A separate eight-run source-to-Object cohort uses the same settings and order:

| Work | Control | Shared inventory | Change |
| --- | ---: | ---: | ---: |
| Internal compiler phases | 15.852715 s | 15.546004 s | -1.93% |
| Complete source-to-Object process | 15.919044 s | 15.612757 s | -1.92% |
| Native preparation | 2.014567 s | 1.601935 s | -20.48% |
| Instructions retired | 227.520 billion | 222.908 billion | -2.03% |
| Maximum compiler RSS | 4,489,805,824 B | 4,489,994,240 B | +0.0042% |
| Peak memory footprint | 4,467,039,934 B | 4,467,310,270 B | +0.0061% |

Both Object blocks improve total compiler time, preparation and instructions.
Parsing increases 0.62%, catalog 1.75%, canonical lowering 0.62%, source CFG
construction 1.05%, cleanup including compaction 1.35%, compaction 3.60%,
numeric generation 0.04%, artifact construction 0.42% and encoding 0.21%.
All stage clocks and adverse observations appear in the
[complete result](../../benchmarks/mech-deflate/native-profile/action-output-inventory-apple-m3-20261115.json).
These cohorts are separate from earlier source changes and the zero/two-worker
comparison; their timings and percentages are not pooled or added.

## Correctness and evidence

Every source, bytecode, Object and complete metadata byte matches in all
qualification observations. Fourteen paired numeric/scalar fixtures pass.
The two preliminary complete-executable gates, eight primary complete builds
and separate default-serial gate reproduce the entire 2,060,440-byte executable:
SHA-256 `d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
The default-serial observation is an unpaired byte gate, excluded from timing
means. No new emitted-program runtime timing campaign is claimed.

All seven focused tests pass: the four existing definition-selection tests and
three inventory tests for original/decoded plans, cache reuse, ordered unique
IDs, sparse/duplicate invalid descriptive outputs, unchanged validation demand
and separate plan owners. Deliberately malformed metadata remains confined to
descriptive queries and never enters emission, admission or execution.

Compiler checking takes 20.384 seconds, rebuilding 379.499 seconds and the real
native CLI check with `--features cranelift_build` 22.976 seconds. The focused
test build takes 148.074 seconds and execution 0.916 seconds. All are outside
program compilation timings. The first focused harness link omits matching
Rust ThinLTO; Apple's older libLTO cannot parse its Rust LLVM bitcode, and no
tests execute. Its original driver and logs remain. Adding `-C lto=thin` to
the harness fixes that setup while the production source and compiler binary
remain frozen.

The thirty-eighth
[repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
retains every command/log, source/patch, output/executable, native input, test
and failed setup. Compiler/test binaries and setup object products are excluded
and hash-pinned. Fresh extraction verifies every manifest/file/Git blob; pure
replay reconstructs the summaries from saved observations, checks source and
commit receipts, and compares all outputs without rerunning compilation or timing.
