# Scoped source range facts

Retain this compiler change. Complete level-12 Deflate builds improve
**19.160918 to 18.646472 seconds (2.68%)** with both balanced blocks faster and
every executable byte identical. Main cleanup improves 13.21% and retired
instructions 3.26%. Complete compilation below ten seconds remains open.

Source commit `972039beaaf9f04a609393905b518f4c16cbab67` has exactly the measured
two-file diff, SHA-256
`3003f121c71bc31dd6437fa87116bc2eb1f18fa9454fb9a0ba36bc95708fc25a`.
Its parent `f450d690cb` adds documentation to the measured source base
`934bde961d`, which includes the newer Deflate/runtime changes. Both subjects use
the same 61 program flags and two workers, the standard `compiler` Rust profile
with Thin LTO/one codegen unit, and no PGO or host-CPU override.

## Work removed

Threaded-edge discovery previously cloned the complete current source range map
for each candidate edge. A route changes at most one branch-condition entry in
that map. It now borrows the map, records the prior entry and restores it on
every exit, including early refusal, before considering another edge.

The base facts are still derived afresh for the current pass and immutable
function. Each route retains its separate propagated facts. All ordinary/
exceptional-edge, range, dominance, live-out and current-visit checks keep their
original order. No grammar/language change or cached shape/factory/ownership
proof is introduced.

## Results

Eight source-to-Object compiles and eight separate complete executable builds
run serially in ABBA / BAAB order, four observations per subject. No concurrent
heavy build, test, sampling, hashing or archival runs during primary timing.
Screens, construction of the compiler and cached link gates remain separate.
The fixed current runtime/entry adapter is pinned and cached; its first-use
construction is excluded from complete native timings.

| Boundary or counter | Control | Scoped facts | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 19.160918 s | 18.646472 s | -2.68% |
| Complete source-to-Object process | 18.957377 s | 18.497868 s | -2.42% |
| Internal compiler phases | 18.900379 s | 18.437835 s | -2.45% |
| Main cleanup including compaction | 3.805963 s | 3.303145 s | -13.21% |
| Retired instructions | 284.431406 G | 275.151030 G | -3.26% |
| Maximum RSS | 6,091,141,120 B | 6,094,548,992 B | +0.06% |

Complete-build blocks improve 2.46% and 2.91%. The separate Object cohort's
compiler blocks improve 2.89%/2.01%, cleanup 14.30%/12.12%, and instructions
3.25%/3.27%. The complete-build saving averages about 0.51 seconds.

Frontend (+0.71%), preparation (+1.45%), helper generation (+0.50%), encoding
(+1.10%) and peak footprint (+0.06%) are adverse. All observations and stages
remain in the [complete result](../../benchmarks/mech-deflate/native-profile/scoped-source-apple-m3-20261115.json).
Earlier smaller-program, PGO and parallel-only cohorts stay separate; their
percentages are not added to this result.

## Equivalence and focused proof

Five focused tests pass, including 112 complete post-pass IR/alias/machine-byte
pairs and 1,760 route queries under five source-map modes. All plan fields
match; each query checks restoration of the entire source map, with 456
successful plans. Cases include loops, shared branch-argument storage,
conflicting/absent range facts, integer/float payloads and effect/load refusals.
The unchanged visit tests also cover query isolation, wrap and ordered walks.

The predecessor build starts at `213d90cc8e`, before PR #973 advances. Its actual
changed source snapshots are byte-identical to the refreshed `934` candidate,
so that focused proof applies exactly. The refreshed candidate then passes all
fourteen paired fixtures, all primary full-output gates and complete executable
gates. After timing, the default serial path also reproduces every program and
complete executable byte. The executable remains 2,060,440 bytes with SHA-256
`d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
No scalar retirement or fresh emitted-program runtime campaign is claimed.

## Current profiles and evidence

Two separate post-qualification stack diagnostics use this retained compiler
and preserve all outputs. Sampled timings/counters are excluded from primary
means; waiting or inclusive stacks are not phase CPU percentages. The largest
named Rust leaf counts in both samples include SSA nonlocal lookup and dimension
canonicalization. Raw/demangled samples and the pinned Rust demangler recipe are
included for subsequent work.

The thirty-first [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains exact source/patches, the full source-commit diff receipt, superseded
predecessor builds/tests, every observation/output/executable, current native
inputs and pure replay. Compiler/test/analysis executables and intermediates are
excluded and SHA-pinned. The initial missing-dependency/module test setup, the
next harness's stale CFG before machine compilation, a demangler format typo and
a replay identifier error all remain. Corrections change no production source
or timed observation. A fresh repository extraction verifies every manifest/
file/Git blob and replays the compact result byte-for-byte.
