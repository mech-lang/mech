# Compiler executable allocator qualification

Using pinned mimalloc in the compiler executable improves complete Deflate
source-to-native-executable compilation from **18.538479 to 16.964989 seconds
(8.49%)**, with both balanced blocks faster and every executable byte identical.
The cost is higher compiler memory: maximum RSS grows **6.089 to 6.701 GB
(10.04%)**. The emitted program keeps its existing allocator and runtime image.
Compilation below ten seconds remains open.

Qualified source `d834508336e4533320cea8587b3e0e7a53124a94` has exactly the
measured seven-file diff from `7931ea0f68521eea56b971cf4a3121c8c066274d`:
SHA-256 `813af33544cdeada7ec4bc8347442a6ee8c8cb02406ab826a699411e74f3c8f8`.
The source commit precedes this evidence publication. Its complete patch,
parent, paths and source snapshots are checked by replay.

## Change and boundaries

One shared module registers `mimalloc::MiMalloc` in the native root CLI when
the `compiler` feature is enabled, and in the profiling executable. Libraries
do not register it. No emitted-program allocator, compiler algorithm, grammar,
proof check or finite admission rule changes. The control is the retained
standard compiler from source `972039beaa`; the candidate uses the same
compiler profile, current Deflate source/runtime, 61 program flags and two
workers. No PGO or native-CPU flags are used.

The two added packages are `mimalloc 0.1.39` and `libmimalloc-sys 0.1.35`, with
empty allocator feature sets. There is no malloc override, debug or secure
feature selection. Cached crate archives match Cargo's cached registry
checksums and are included in the evidence. Other locked package versions and
dependencies are unchanged; some existing root lockfile blocks are reordered.

## Complete builds and compiler work

Eight complete builds run serially in ABBA/BAAB order after compiler
construction, with no concurrent heavy build, test, sampling, hashing or
archival. Timings cover fresh source assembly, parsing, checked frontend,
preparation, optimization, code generation, Object/metadata writes, Object
handoff and final native link completion. Fixed runtime/adapter inputs are
pinned and cached; their first-use construction is excluded.

| Complete-build boundary | System control | mimalloc compiler | Change |
| --- | ---: | ---: | ---: |
| Source to executable | 18.538479 s | 16.964989 s | -8.49% |
| Source-to-Object process within those builds | 18.403265 s | 16.826002 s | -8.57% |
| Native link | 0.134255 s | 0.137997 s | +2.79% |

The two complete-build blocks improve 8.58% and 8.40%. Link time increases
about 3.7 ms and Object handoff about 31 microseconds; both adverse observations
remain. There is no linker change. All individual observations remain in the
[complete result](../../benchmarks/mech-deflate/native-profile/compiler-mimalloc-apple-m3-20261115.json).

A separate eight-run source-to-Object cohort uses the same order and settings:

| Compiler work | System control | mimalloc compiler | Change |
| --- | ---: | ---: | ---: |
| Internal compiler phases | 18.529446 s | 16.778546 s | -9.45% |
| Complete source-to-Object process | 18.586927 s | 16.856573 s | -9.31% |
| Parsing | 2.118580 s | 1.862715 s | -12.08% |
| Checked frontend | 2.547806 s | 2.485698 s | -2.44% |
| Native preparation | 2.410935 s | 2.008170 s | -16.71% |
| Numeric helper generation | 2.533289 s | 2.066483 s | -18.43% |
| Source CFG construction | 2.404341 s | 2.353352 s | -2.12% |
| Cleanup including value compaction | 3.363862 s | 3.093168 s | -8.05% |
| Instructions retired | 275.206 billion | 237.869 billion | -13.57% |
| Maximum RSS | 6,089,154,560 B | 6,700,613,632 B | +10.04% |
| Peak memory footprint | 6,058,628,412 B | 6,679,209,414 B | +10.24% |

Both Object-cohort blocks reduce compiler time, frontend, source CFG and
instructions. These intervals come from their own cohort; their percentages
are not added to the complete-build gain or earlier parallel/PGO results.
Fewer allocator instructions do not establish that redundant schema copying
or SSA lookup has been removed. Those algorithms are unchanged.

## Validation and preserved failures

All source, bytecode, Object and complete metadata bytes match in every
observation. Fourteen paired numeric/scalar fixtures pass. The two initial
cached executable gates, eight primary complete builds and later default-serial
gate all reproduce the entire 2,060,440-byte executable with SHA-256
`d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
The default-serial observation is an output gate, without a paired speed claim.
No fresh emitted-program runtime timing campaign or scalar retirement is claimed.

The profiling compiler check/build take 12.106 and 253.638 seconds, separately
from compiling Deflate. The real root native CLI check passes with
`--features cranelift_build` in 51.990 seconds. An initial default-feature CLI
check fails on two `native_collection_templates` feature-gating errors. The
original root manifest, lockfile and binary reproduce those exact errors;
this qualification does not fix or claim a passing default-feature CLI build.
Both failures, source pins and their comparison remain in the packet.

The first source-pinning setup fails before repository mutation because an
extracted registry source lacks `.cargo-checksum.json`; cached registry-index
checksums and complete crate archives provide the corrected pins. A later
default-serial compiler completes with matching files, but its timing wrapper
fails on sandbox-denied `sysctl kern.clockrate`. That failed observation is
preserved outside primary means. The same frozen binary, flags and scripts
pass with the required counter permission; automatic approval review does not
reject the action.

The thirty-fifth [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains all seven source snapshots and the complete patch, build/check logs,
crate archives, failed attempts, fixtures, observations and complete outputs,
fixed native inputs, source-commit proof and pure offline replay. Compiler
executables and dependency libraries are excluded and hash-pinned. A fresh
repository extraction verifies every manifest/file/Git blob, then reproduces
the compact result byte for byte without compiling or timing again.
