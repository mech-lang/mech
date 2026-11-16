# Index-range definition facts: compilation qualification

Complete fresh level-12 compilation takes **18.691–18.903 seconds** on the Apple
M3 Air. Eight matched runs show **1.61% lower whole-process time, 1.79% lower
compiler time, 10.74% lower numeric-helper generation time and 2.06% fewer
instructions**. Source, MBC, Object and complete metadata bytes remain identical.
Memory is an adverse 0.0068% higher with overlapping ranges. The ten-second
compilation goal remains open.

`Indices::definition` previously constructed new same-plan descriptive facts on
every lookup. The range walk looks up each nonconstant definition during discovery
and again when finishing it, repeatedly rebuilding the complete definition table.
Each analyzer now owns one `NativeProgramFacts` scratch object borrowing exactly
its own immutable `CodePlan`. The table is still constructed on first demand and
is discarded with the analyzer. No facts survive a rewrite or different owner.

The entire `proves` method, supported-operation selection and scalar-range
arithmetic remain byte-for-byte unchanged. Unique-definition filtering,
conservative ambiguous/opaque/cyclic handling, constant enumeration and each
current minimum comparison keep their existing behavior. No new range, guard,
resource, shape, ownership or current-visit answer is cached. No execution
machinery is retired; grammar, language, checked errors, direct scalar emission,
admission limits, runtime accounting and backend settings retain their behavior.

Source `4a451c748295164aa300d4e03ff9b6a4fdbd740f`, parent `7612f873060a45f69e8f6eccf4e603827cd2eb1d`.
The frozen control comes from `ba9298bca36ca7879501060ad7d3e7c2d5222a7d`; its production
source equals the documentation-only parent. Both subjects use the same opt-in
`compiler` profile, full fresh source-to-Object pipeline and 60 runtime flags.
Hardware: Apple M3 Air, 8 cores (4P+4E), 16 GB, macOS 26.2.

| Measurement | Control range (4 runs) | Candidate range (4 runs) | Mean change |
| --- | --- | --- | ---: |
| Compiler, s | 18.846725–19.220871 | 18.612269–18.679272 | -1.79% |
| Whole process, s | 19.101754–19.320744 | 18.690837–18.903411 | -1.61% |
| Parse, s | 2.429130–2.456250 | 2.425655–2.431546 | -0.57% |
| Frontend, s | 2.464178–2.495466 | 2.463476–2.484255 | -0.49% |
| Native, s | 13.883071–14.199088 | 13.635632–13.699774 | -2.24% |
| Canonical lowering, s | 0.098155–0.099450 | 0.097583–0.098290 | -0.79% |
| Preparation, s | 3.017505–3.083236 | 2.980172–3.022152 | -1.18% |
| Numeric generation, s | 2.374027–2.387083 | 2.121572–2.128569 | -10.74% |
| Source CFG, s | 2.360927–2.439008 | 2.314268–2.336447 | -2.41% |
| Cleanup including compaction, s | 3.345252–3.452610 | 3.374968–3.487506 | +1.34% |
| Compaction, s | 0.167192–0.169626 | 0.167154–0.170194 | +0.05% |
| Retired instructions | 283,163,549,389–283,410,846,736 | 277,450,353,992–277,541,264,072 | -2.06% |
| Peak RSS, bytes | 5,911,494,656–5,911,920,640 | 5,911,560,192–5,912,739,840 | +0.01% |
| Peak footprint, bytes | 5,880,976,568–5,881,402,552 | 5,881,042,104–5,882,221,752 | +0.01% |

Runs are serial control/candidate/candidate/control then
candidate/control/control/candidate. Both blocks improve compiler time (1.26%
and 2.31%). Every candidate compiler, process, native, helper-generation and
instruction observation falls below every control. Cleanup including compaction
is an adverse 1.34% higher; its ranges overlap. Both memory means are slightly
higher with overlapping ranges, so no memory saving is claimed. All observations
remain. These finite cohorts are descriptive, not population estimates. Both
subjects include SSA value compaction and its cost; native intervals are nested.

Builds, screens, fixtures, hashing, copying and archive work do not overlap primary
timing. The observed 273.188-second compiler rebuild uses existing Cargo caches,
is excluded from Mech compilation timing and is not a matched cold-bootstrap
comparison. Both screens and every primary output match the qualified program
byte for byte. Fourteen successful fixture pairs cover six array/enum programs
and eight checked scalar programs, matching all four files. Generated program
machine code and metadata remain identical; no new compressor-runtime timing
campaign is claimed.

This field-lifetime change is verified by the successful production compiler build,
complete compressor and fixture output checks, and static comparison of the exact
range/guard calculation body. No new Rust unit-suite run or CPU sampling campaign
is claimed. The prior 18 focused tests and bare runtime-only lint failures keep
their original source pins in the helper-liveness packet. This file is only used
by native code generation; the unrelated bare-feature failure is not rerun or
silenced here. The packet also retains a replay-source-pattern correction for
rustfmt line wrapping; compiler source and observations are unchanged by it.

The next recorded candidate is `numeric_exits::live_query`, which still loops over
all plan values and queries definitions even for unread values. A borrowed
same-plan inventory could avoid those repeated lookups, provided the original
first-demand validation behavior is preserved for empty/wanted subsets and
invalid unused definitions. No lookup/validation removal or saving from that
future work is applied or credited here.

The [compact result](../../benchmarks/mech-deflate/native-profile/index-facts-compile-apple-m3-20261114.json)
and [receipt](../../benchmarks/mech-deflate/native-profile/index-facts-compile-apple-m3-20261114.archive.json)
pin the complete [benchmark packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md).
It includes source/base snapshots and patches, lazy-constructor source, commands,
flags, raw cohorts, outputs, corrections and pure offline replay. Compiler
executables are excluded and hash-pinned. Python 3.11+ replay needs no compilers,
benchmark execution, original scratch paths or external services.
