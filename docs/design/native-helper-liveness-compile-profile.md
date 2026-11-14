# Packed helper liveness: compilation qualification

Complete fresh level-12 compilation takes **18.884–19.110 seconds** on the Apple
M3 Air. Eight matched runs show **3.74% lower whole-process time, 3.73% lower
compiler time, 24.15% lower numeric-helper generation time and 4.30% fewer
instructions**. Source, MBC, Object and complete metadata bytes remain identical.
Memory ranges overlap and no memory improvement is claimed. The ten-second
compilation goal remains open.

Helper liveness previously cloned and unioned ordered sets on repeated complete
graph sweeps. It now shares preparation's existing packed transfer kernel, using
query-local word ordinals. That kernel's transfer body is unchanged apart from
whitespace; its signature now supports both source u32 values and usize words.
The emitter's recording block is byte-for-byte unchanged. Aggregate and scalar
read/write effects, checked guards, handoff exports and selected edges retain
their existing behavior.

Local reads seed before local writes kill successor facts. Terminal outputs are
observed after each exit site's writes. Fallback nodes keep exactly their recorded
exports and no implicit result edge. The same 16M-cell cap bounds the two packed
planes to 256 MiB; larger products keep each caller's original solver. This cap
covers the two planes, not every graph/worklist allocation. No plan, resource,
guard, shape, visit or ownership result is cached. No execution machinery is retired.

Source `ba9298bca36ca7879501060ad7d3e7c2d5222a7d`, parent `4c2a8113374f26290771f851afe0506fc18937a3`.
The frozen control comes from `1774ca003111a511d9f961fded0005412e1f39df`; its production
source equals the documentation-only parent. Both subjects use the same opt-in
`compiler` profile, full fresh source-to-Object pipeline and 60 runtime flags.
Hardware: Apple M3 Air, 8 cores (4P+4E), 16 GB, macOS 26.2.

| Measurement | Control range (4 runs) | Candidate range (4 runs) | Mean change |
| --- | --- | --- | ---: |
| Compiler, s | 19.476354–19.605394 | 18.776748–18.849440 | -3.73% |
| Whole process, s | 19.533079–19.737376 | 18.883703–19.109594 | -3.74% |
| Parse, s | 2.451462–2.469664 | 2.427275–2.442396 | -1.02% |
| Frontend, s | 2.419169–2.432572 | 2.415687–2.432625 | -0.16% |
| Native, s | 14.516616–14.647398 | 13.861026–13.911637 | -4.80% |
| Canonical lowering, s | 0.098421–0.098625 | 0.098345–0.099077 | +0.13% |
| Preparation, s | 2.954251–2.991839 | 3.010466–3.022579 | +1.31% |
| Numeric generation, s | 3.133549–3.149147 | 2.371178–2.394157 | -24.15% |
| Source CFG, s | 2.332759–2.338981 | 2.360099–2.366508 | +1.14% |
| Cleanup including compaction, s | 3.298772–3.383438 | 3.351765–3.365489 | +0.10% |
| Compaction, s | 0.159803–0.160969 | 0.164525–0.165261 | +2.63% |
| Retired instructions | 295,333,561,499–295,497,349,317 | 282,523,635,898–282,889,663,375 | -4.30% |
| Peak RSS, bytes | 5,910,937,600–5,912,657,920 | 5,911,461,888–5,911,740,416 | +0.00% |
| Peak footprint, bytes | 5,880,599,736–5,882,320,056 | 5,880,943,800–5,881,222,328 | -0.00% |

Runs are serial control/candidate/candidate/control then
candidate/control/control/candidate. Both blocks improve compiler time (3.86%
and 3.61%). Every candidate compiler, process, native, helper-generation and
instruction observation falls below every control. Preparation is an adverse
1.31% higher, source CFG 1.14% higher and compaction 2.63% higher. All observations
remain. RSS and footprint means are effectively unchanged, with overlapping
ranges. These finite cohorts are descriptive, not population estimates. Both
subjects include SSA value compaction and its cost. Native substage intervals
are nested and must not be added to whole-process time.

Builds, tests, screens, fixtures, feature checks, hashing, copying and packet work
do not overlap primary timing. The observed 273.362-second compiler rebuild uses
existing Cargo caches, is excluded from Mech compilation timing and is not a
matched cold-bootstrap comparison. Both screens and every primary output match
the qualified program byte for byte. Fourteen successful fixture pairs cover six
array/enum programs and eight checked scalar programs, matching all four files.
Generated machine code and metadata are identical; no new compressor-runtime
latency or CPU sampling campaign is claimed.

All **18 unique focused tests pass**: 13 existing preparation cases and five helper
cases. The helper tests compare packed and forced original-set fallback paths
against an independent read-before-write path oracle on 512 seeded graph families,
including branches, cycles, duplicate/reordered sites, local and terminal writes,
fallback exits, missing headers, sparse usize IDs and scratch limits. Static
source checks separately verify the unchanged recording block and transfer body.
The full GPU/runtime unit suites were not rerun. Grammar, language, checked errors,
direct scalar emission, fresh proofs, finite admission, runtime accounting and
backend optimization settings keep their existing behavior.

An additional bare `canonical-native-program-runtime` check **fails on both parent
and candidate** with the same pre-existing unused-function errors for `derive` and
`derive_tree` in unchanged `readonly_numeric.rs`. No lint allowance is added and
this check is not counted as passing. The parent is checked from its exact Git
archive without altering the main worktree. Both commands, logs, source pins and
error comparison remain. The full compiler build used for measurement passes.
An early test-wrapper inner-doc-comment placement error is also preserved and
was fixed before any test or compiler measurement.

The next recorded target is `numeric_phase_bounds::Indices::definition`, which
creates fresh same-plan facts on every lookup. The prior pinned helper CPU window
contains inventory construction under that method. Keeping those descriptive
facts for one Indices lifetime is a future experiment; no saving is credited here.
Earlier CPU samples retain their original source pins.

The [compact result](../../benchmarks/mech-deflate/native-profile/helper-liveness-compile-apple-m3-20261114.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/helper-liveness-compile-apple-m3-20261114.archive.json)
pin the complete [benchmark packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md).
It contains source/base snapshots and patches, commands, flags, tests, raw cohorts,
outputs, corrections, feature-check failures and pure offline replay. Compiler/test
executables are excluded and hash-pinned. Python 3.11+ replay needs no compilers,
benchmark execution, original scratch paths or external services.
