# Late known-origin SSA trials

Reusing known incoming SSA values removes millions of temporary values but
does not improve compilation on the current Deflate workload. The larger trial
increases source-to-Object compiler phases **18.352 to 18.705 seconds (+1.92%)**
and source CFG construction **2.339 to 2.775 seconds (+18.67%)**. Both balanced
blocks are slower. The production source and qualified compiler executable are
restored; no complete-build improvement is claimed.

The frozen baseline is `bfe3ee58198a18c6039c638e434c7aa3800c4d5f`, whose compiler
source retains `972039beaa`. Both subjects use the current Deflate/runtime,
standard compiler profile, the same 61 program flags and two workers. This is a
separate experiment from the [parallel scheduling comparison](native-refreshed-parallel-compile-profile.md).
Earlier smaller-program, native-CPU and PGO percentages are not added.

## Two conservative prototypes

Both prototypes change only `vendor/cranelift-frontend/src/ssa.rs`. They keep
the original CFG edges, sealing schedule and predecessor fallback. A bounded
structural check recognizes predecessor paths with no actual SSA assignments.
The shortcut uses an incoming value only when it already exists; unknown
values keep the original allocation path. The bounds decline an optimization
and impose no language or CFG limit.

The first prototype handles pending parameters at ordinary block sealing.
Its compiler build, 30 copied-module unit tests and 304 differential cases pass.
It is superseded by the larger lookup prototype before complete Deflate
qualification. It has **no primary program timing**, and is not described as a
measured performance regression. Its separate compiler check/build take 21.185
and 379.855 seconds.

The second also records the unchanged incoming origin at ordinary sealing. A
read-only lookup can then avoid allocating a sealed join parameter that the
original algorithm would immediately retire. Unknown origins still create
temporary and retained parameters at their original points. Its compiler
check/build take 6.805 and 248.734 seconds; these are compiler construction
times, excluded from program compilation measurements. Its 32 unit tests pass.

## Measured result

Eight primary source-to-Object observations run serially in ABBA/BAAB order,
without concurrent heavy builds, tests, sampling, hashing or archival. Initial
screens, fixtures and cached native link gates are separate from these means.

| Work | Retained control | Known-origin trial | Change |
| --- | ---: | ---: | ---: |
| Compiler phases, source to Object | 18.352094 s | 18.704612 s | +1.92% |
| Complete source-to-Object process | 18.409816 s | 18.762904 s | +1.92% |
| Source CFG construction | 2.338513 s | 2.775128 s | +18.67% |
| Cleanup including value compaction | 3.315535 s | 3.237286 s | -2.36% |
| Value compaction | 0.166861 s | 0.124021 s | -25.67% |
| Instructions retired | 274.694 billion | 273.160 billion | -0.56% |
| Maximum RSS | 6,089,265,152 B | 6,090,739,712 B | +0.024% |

The two blocks increase compiler time 1.74% and 2.10%, and source CFG time
18.69% and 18.65%. Raw value IDs fall from 21,152,385 to 15,871,274, while both
compact to the same 1,307,512 retained values. Avoiding 5,281,111 temporary
values saves about 43 ms of compaction, but the measured construction cost
increases about 437 ms. Fewer values and instructions therefore do not establish
a useful time saving. This experiment does not isolate a hardware or exclusive
algorithmic cause for that difference.

Every observation remains in the [complete result](../../benchmarks/mech-deflate/native-profile/known-origin-ssa-trials-apple-m3-20261115.json).
No balanced complete source-to-executable cohort is run for this rejected
trial because the targeted construction and total compiler phases already
worsen in both blocks. The retained complete-build qualification remains
[18.646 seconds](native-scoped-source-facts-compile-profile.md); compilation
below ten seconds remains open.

## Equivalence and the earlier forwarding rejection

Each prototype passes 304 paired comparisons of complete compacted IR,
aliases/types/definitions, normalized queries, side effects, unique resolved
variable-definition sets and AArch64 machine bytes. These include assignment
fallback, loops, sparse IDs, manual parameters, three-way guards and an unsealed
enclosing origin. Raw cached-definition enumeration order and duplicate counts
are not equated.

Of those cases, 144 additionally compile the original early-sealing forwarding
implementation from `d4efd4465c5a8689016abd5b42e16f73e475af63` with current value
compaction. It still changes compacted IR in 87 cases and machine bytes in 30.
Compaction preserves the relative order of retained definitions; it does not
erase every change caused by creating retained parameters earlier. The original
[runtime regression evidence](../../benchmarks/mech-deflate/native-profile/ssa-forwarding-rejected-apple-m3-20261114.json)
remains applicable to that rejected implementation. It is not reapplied.

The larger prototype also passes fourteen fixture pairs and exact comparisons
of all source, bytecode, Object and complete metadata bytes. Both cached native
link gates reproduce the entire 2,060,440-byte executable with SHA-256
`d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
No new emitted-program runtime timing campaign, grammar change, weaker proof,
finite-admission change or scalar retirement is claimed.

The thirty-fourth [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains both frozen sources/patches, builds, tests, differential observations,
all timing and output records, fixed native inputs and pure offline replay.
The first Rust harness type-inference failure, runner-path setup correction and
wrapper preflight correction remain. Compiler/test/probe executables are
excluded and hash-pinned. A fresh extraction verifies all 657 files and their
Git blobs, then reproduces the compact result byte for byte without compiling.
