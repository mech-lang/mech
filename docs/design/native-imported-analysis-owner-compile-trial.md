# Imported-cleanup analysis-owner trial

Sharing analyses across the unchanged cleanup suffix is **rejected** because
complete builds are effectively flat: **15.287110 to 15.281101 seconds
(-0.04%)**, with mixed balanced blocks. The compiler process within those builds
increases 0.05%; the faster final link accounts for the small overall reduction.
The separate Object cohort lowers compiler time 0.35%, cleanup including
compaction 1.01% and instructions 0.38%. This establishes a small work reduction,
with no useful complete-build gain. The original source and standard compiler
are restored. Compilation below ten seconds remains open.

## Frozen implementation and selection

Base source is `a90a3d81d9f155242a29c593b1df49651cdc561c`. The entire two-file
trial patch is SHA-256
`219ca96bfb85057caf061ce5259fd8f001ba7f77469a720d80d760488ffcfeb3`.
It introduces one owner exclusively borrowing the same Cranelift Context across
the fixed-window, ordered-span, adjacent-address, Boolean-home and selected-
increment suffix. The original pass sequence, enable/eligibility checks,
round limits, transformation decisions and verification calls remain.
Earlier passes retain their existing preparation.

The baseline's main cleanup report records 490,850 blocks/1,342,586 instructions
before cleanup and 82,895 blocks/162,023 instructions afterward. All six report
counters for these final five passes are zero. The source nevertheless repeats
alias normalization and full CFG/dominator construction at their entries.
Those counts identify a candidate; they do not establish per-analysis cost or
removable time. The reference metadata hash and exact fields are retained.

The trial prepares aliases/graphs once, reuses them across audited zero-report
passes, and invalidates both facts before every reported mutation's original
post-cleanup. Invalidation includes successor ordering, layout and aliases;
it never relies only on edge-set equality. The selected-increment stage keeps
its original graph-only request, with no added alias sweep. Standalone production
wrappers start with a fresh owner. The standalone home wrapper becomes test-only
because the shared sequence is its only production caller.

The five pass implementations are included with a mutation audit. Their zero
reports preserve graph/layout/successor order and normalized aliases. A zero-
report home pass can reconstruct identical argument-list storage; its values
and graph remain the same. No permission, factory, shape or current-visit proof
is cached. Grammar, scalar emission, checked failures and finite admission paths
retain their existing implementation.

## Matched measurements

Control dd7d and candidate 017c use the standard ThinLTO compiler profile and
pinned compiler-only mimalloc, with the same 61 program flags and two workers
on the Apple M3. No PGO or native-CPU flags are added. The separately qualified
5a1687 PGO tool stays unchanged on d26 source; no combined gain is inferred.

Eight complete builds run serially in ABBA/BAAB order, without overlapping
heavy build, test, sampling, hashing or archive work. The boundary includes
fresh source assembly through final native link completion. The fixed runtime/
entry adapter and native inputs are cached and hash-pinned. Its historical
first-use construction of 11.882560208 seconds is excluded.

| Boundary | Control | Analysis owner | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 15.287110 s | 15.281101 s | -0.04% |
| Source-to-Object process within those builds | 15.151399 s | 15.159681 s | +0.05% |
| Final native link | 0.134633 s | 0.120388 s | -10.58% |

The first complete-build block is 0.40% slower; the second is 0.47% faster.
Object handoff falls 4.24%, about 0.046 ms. All observations remain. Fixed link
inputs and linker implementation are unchanged, so their movement is not
credited to the owner.

A separate eight-run Object cohort uses the same configuration and order:

| Boundary | Control | Analysis owner | Change |
| --- | ---: | ---: | ---: |
| Internal compiler phases | 15.191553 s | 15.138006 s | -0.35% |
| Complete source-to-Object process | 15.258198 s | 15.207109 s | -0.33% |
| Cleanup including compaction | 2.971528 s | 2.941380 s | -1.01% |
| Cleanup excluding compaction | 2.828893 s | 2.792570 s | -1.28% |
| Value compaction | 0.142635 s | 0.148809 s | +4.33% |

Both Object compiler-time blocks improve, by 0.58% and 0.12%. Instructions fall
220.656304 to 219.810603 billion (-0.38%). Source CFG construction increases
1.20%, canonical lowering 0.09%, RSS 0.0127% and footprint 0.0136%. Compaction
increases 4.33%. Every stage/counter and adverse observation remains in the
[complete result](../../benchmarks/mech-deflate/native-profile/imported-analysis-owner-apple-m3-20261115.json).
The Object and complete-build cohorts remain separate. No new sampling or
emitted-program runtime campaign is run.

## Correctness and restoration

Three focused tests execute the exact candidate owner and new test/helper
bodies against the actual candidate Cranelift Context/CFG/dominator/DFG library.
They compare fresh recomputation for 64 unchanged-function seeds, 16 successor-
order/layout seeds and 16 graph-only/alias-normalization seeds. Full predecessor/
successor order, immediate dominators and children, cached postorder/RPO,
alias destinations and IR match. Swapping branch successors changes cached
traversal order even with the same edge set; invalidation refreshes it correctly.
Removed blocks and graph-only requests for newly introduced aliases also match.

Every source, bytecode, Object and **complete metadata** byte matches in the main
observations and all 14 scalar/numeric fixture pairs. Both preliminary complete
executable gates and all eight primary full builds reproduce the complete
2,060,440-byte executable, SHA-256
`d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
No additional candidate default-serial gate is run after rejection.

Compiler check/build take 13.398/260.305 seconds; the real native CLI check with
`--features cranelift_build` takes 15.559 seconds. Focused build/execution take
4.283/0.445 seconds. All pass outside program timings. No failed build, test or
measurement setup occurs in this trial.

Both production-file hashes match the original references after restoration.
The immutable dd7 standard compiler is restored and the selected PGO artifact
is untouched. Temporary GPU rlibs remain from the candidate until the next
genuine Cargo rebuild; fresh restored source mtimes force that rebuild before
using the cache as a new baseline.

The forty-third [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
retains the rejected source/patch, exact tests, audited pass sources, selection
metadata, every command/log and observation, outputs/executables, fixed native
inputs and restoration receipt. Compiler/test binaries are excluded and
hash-pinned. Fresh extraction verifies every manifest/file/Git blob; pure replay
checks source/test identities, reconstructs the timing summaries and selection
fields, and reproduces the compact result byte-for-byte without rerunning timing.
