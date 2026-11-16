# Constant-phi propagation trials

Neither worklist layout is retained. The compact revision reduces measured
instructions and main cleanup slightly, but its separate complete executable
cohort averages **0.46% slower**, with mixed blocks. Production retains the
original full-sweep solver. Complete compilation below ten seconds remains open.

Both trials start at `ab59d593f4dcc3e457d8c462b459f60eaf80851e`, using the
qualified compiler from `1090de011646c92e30c2d4b648ed0792c50f300d` as control.
Both subjects use the same 61 program flags and
`MECH_COMPACT_COMPILE_THREADS=2`. The Rust compiler profile, Thin LTO and one
codegen unit are unchanged. PGO is disabled in both subjects; the
[earlier frozen-source PGO result](native-pgo-compile-profile.md) is separate.

## Implementations

The first layout assigns compact formal indices, records dependencies and fixed
seeds, and propagates `None -> One -> Many` changes through a queue. It then
rebuilds a final `Value -> AbstractValue` map for the existing edits. This avoids
repeated global sweeps but adds graph construction and another map build.

The second layout retains a `Value -> u32` map and vector of abstract states
through editing. Block-relative target indices remove formal lookups while
encoding edges. Edge endpoints, successors and queue entries use compact `u32`
analysis indices, separate from the unchanged IR Value IDs. CSR offsets/counts
remain `usize`. No final abstract-value map is rebuilt in ordinary compilation;
the diagnostic oracle materializes one for the complete comparison.

For both layouts, summary construction and the entire IR-editing section are
byte-identical to the original source. Only analysis representation/propagation
changes. The original full-sweep map remains an optional pre-mutation oracle;
the test-only reference path also permits complete post-pass comparisons.

## Source-to-Object results

Each layout has its own eight-run serial ABBA / BAAB cohort, four observations
per subject, using the precise process-completion driver and macOS counters.
There are no concurrent builds, tests, sampling, hashing or archival during
primary timing. Screens, oracle diagnostics and compiler-tool builds are separate.

| Work | First layout control | First layout candidate | Change |
| --- | ---: | ---: | ---: |
| Complete source-to-Object process | 17.213456 s | 17.186754 s | -0.16% |
| Internal compiler phases | 17.159481 s | 17.105896 s | -0.31% |
| Main cleanup including compaction | 3.412370 s | 3.428564 s | +0.47% |
| Retired instructions | 255.562264 G | 255.739937 G | +0.07% |

The first layout's compiler-time blocks are mixed: +0.21% and -0.83%. Cleanup
increases in both, 0.82% and 0.13%; instructions increase in both, 0.08% and
0.06%. Favorable whole-compiler means do not establish a useful target-stage
improvement. No balanced complete-executable cohort is run for this rejected
layout; its standalone exact executable gates still pass.

| Work | Compact revision control | Compact revision candidate | Change |
| --- | ---: | ---: | ---: |
| Complete source-to-Object process | 17.335251 s | 17.286514 s | -0.28% |
| Internal compiler phases | 17.280766 s | 17.210235 s | -0.41% |
| Main cleanup including compaction | 3.420871 s | 3.395715 s | -0.74% |
| Retired instructions | 256.782784 G | 256.335099 G | -0.17% |

The compact revision improves compiler phases 0.34% and 0.47%, cleanup 0.91%
and 0.56%, and instructions 0.19% and 0.16% in the two blocks. The mean main
cleanup saving is about 25 milliseconds. Native preparation is adverse at
+0.56%; catalog is +5.89%, and frontend/encoding also rise slightly. All stages,
memory counters and observations remain in the
[complete result](../../benchmarks/mech-deflate/native-profile/constant-phi-trials-apple-m3-20261114.json).
Neither cohort is pooled with the other layout or earlier compiler results.

## Complete source to executable

The compact revision proceeds to eight additional serial ABBA / BAAB complete
builds. Each includes fresh source assembly/analysis, optimization/code
generation, Object/metadata writes, Object handoff and final native linking.
The fixed adapter and linker inputs are pinned and cached; adapter first-use
construction is excluded.

| Boundary | Control mean | Compact revision mean | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 17.337576 s | 17.416708 s | +0.46% |
| Source to Object within those builds | 17.204197 s | 17.284916 s | +0.47% |
| Final native link | 0.132488 s | 0.130872 s | -1.22% |

The complete-build blocks are mixed: the candidate is 1.80% slower in the first
and 0.85% faster in the second. Final-link movement is observational. These
results remain alongside the favorable source-to-Object work counters; no runs
are removed, and no useful complete-build gain is claimed. The small targeted
reduction does not justify retaining the additional solver/graph machinery for
this optimization round.

## Equivalence and work counts

Both layouts pass four focused tests. The tests compare 512 generated graphs
against the original full-sweep algorithm and an independent incoming-root graph
oracle, and compare 72 complete post-pass functions and alias definitions.
Explicit cases cover unseeded cycles, duplicated edges, conflicts, exception
arguments and a reverse chain. The direct harness compiles the exact changed
production module against pinned Cranelift APIs. A module-level `no_std`
configuration check passes with std Cranelift dependencies; this is explicitly
not a whole-library `no_std` feature build.

Each full Deflate diagnostic compares every tracked abstract value against the
original solver before mutation, over **1,782 actual pass invocations**. Both
layouts produce the same work totals:

| Diagnostic total across pass invocations | Count |
| --- | ---: |
| Tracked formals | 1,744,572 |
| Formal dependencies | 2,826,224 |
| Processed queue entries | 2,887,809 |
| Original full-sweep iterations | 4,982 |

The original solver takes three iterations in 1,409 invocations, two in 367,
four in five and one in one. These counts describe analysis work, not exclusive
time or distinct program values. They do not imply a large number of expensive
full sweeps on this workload.

All fourteen paired scalar/nested numeric fixtures per layout match. Every
screen, diagnostic and primary compile reproduces source, bytecode, Object and
complete metadata. Both layouts pass exact complete-executable link gates, and
every compact-revision native build retains the 1,878,568-byte executable with
SHA-256 `557d0f5ef2b979e01851ae23b5b3944453a0bd9e772d68161be198916ca06c2c`.
No grammar/language change, weaker proof check, scalar retirement or fresh
emitted-program runtime campaign is claimed.

The first direct test harness omits the existing `log` dependency. Its next
link attempt omits the Rust Thin LTO flags needed by LLVM 22 dependencies, which
the system LLVM 15 linker cannot read. Neither attempt executes tests; both
failures remain. Correcting the harness changes no production source. Both
successful focused runs pass all four tests. Full compiler checks take 14.222
and 13.502 seconds, and rebuilds take 260.658 and 261.864 seconds, all excluded
from program compilation measurements.

## Evidence and follow-up

The original solver source is restored exactly. The twenty-seventh
[repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains both tested source snapshots/patches, independent/reference tests,
every build and timing log, all adverse observations, program/fixture/executable
bytes, fixed native inputs and pure replay. Compiler/test executables and
compiler intermediates are excluded and SHA-pinned. The repository copy is
extracted, manifest/Git-blob verified and replayed byte-identically against its
compact result.

The next scheduling investigation concerns helper cleanup, which currently
remains on the coordinator. Cleanup can feed nested admissions and module
definitions, so moving it requires preserving those ordered dependencies.
Separate current-worker stack samples will guide that split; no additional
parallel saving is established by these solver trials.
