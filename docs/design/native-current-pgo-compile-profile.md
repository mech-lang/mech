# Current compiler: native CPU and PGO

The current-source combined build strategy reduces complete level-12 Deflate
compilation **17.347076 to 15.920913 seconds (8.22%)**, with both balanced blocks
faster and every executable byte identical. This includes fresh source
compilation and final native linking with the fixed runtime adapter cached.
Complete compilation below ten seconds remains open.

Both subjects use frozen source
`aaef2a932bf81c7638c522dc777a2b78f0abcb5d`, including the retained frontend
changes and [actual phase workers](native-parallel-phases-compile-profile.md).
Both enable the same 61 program flags and `MECH_COMPACT_COMPILE_THREADS=2`.
The candidate combines Rust `target-cpu=native` and instrumentation PGO; the
matched control has neither host-build override. This cohort measures that
combined strategy. The earlier worker and frozen-source PGO cohorts stay separate.

PR #973 advances to `9e34dc07e0` after these measurements, changing Deflate source
and the runtime allocator. The numbers here remain pinned to the preceding
`aaef` source and its exact program/runtime outputs; they do not measure that
newer compressor/runtime revision.
The [updated-workload parallel cohort](native-refreshed-parallel-compile-profile.md)
records the newer source separately, without PGO.

## Build and qualification

All four compiler-tool builds use `nightly-2026-03-03`, explicit
`aarch64-apple-darwin`, the `compiler` profile with Thin LTO and one codegen unit,
four Cargo jobs, disabled incremental compilation and warnings-as-errors.
They take 458.536 seconds (standard control), 450.114 seconds (native CPU
control), 632.126 seconds (instrumented) and 476.545 seconds (profile use).
Compiler rebuilding and training are excluded from program compilation timings.

The standard/native-CPU controls first pass complete program-output and
executable gates. Eight fixed programs then train the instrumented compiler;
all training outputs match unprofiled controls. Matching LLVM 22.1.0 tools merge
the eight raw profiles. The final candidate uses the absolute merged profile
path with `-C llvm-args=-pgo-warn-missing-function`. Its Cargo JSON log contains
**18,607 missing-function warnings** and no hash/control-flow mismatch
diagnostics. All commands, profiles, configuration and diagnostics remain.

Qualification comprises fourteen paired fixtures, seven held-out programs with
eight balanced compiles each, eight primary source-to-Object compiles and eight
separate complete source-to-executable builds. Each primary cohort runs serially
in ABBA / BAAB order, four observations per subject, after all tool builds and
training finish. No concurrent heavy build, test, sampling, hashing or archival
runs during those cohorts. Screens and standalone link gates remain separate.

## Complete source to executable

| Boundary | Standard compiler | Native CPU + PGO | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 17.347076 s | 15.920913 s | -8.22% |
| Source to Object within those builds | 17.218415 s | 15.785290 s | -8.32% |
| Object handoff | 0.000890 s | 0.000929 s | +4.41% |
| Final native link | 0.127770 s | 0.134694 s | +5.42% |

Complete-build blocks improve 8.19% and 8.25%. The native-link increase is
observational; no linker change is made. Both subjects freshly assemble/analyze
source, optimize/generate code and write Object/metadata before final linking.
The runtime/entry adapter and native inputs are pinned and cached; adapter
first-use construction is excluded.

## Compiler work and adverse results

The separate source-to-Object cohort measures precise process completion and
macOS counters. Its complete process improves 9.06%, internal compiler phases
9.15% and retired instructions 9.24%. Both blocks improve each of those totals.

| Work or counter | Standard compiler | Native CPU + PGO | Change |
| --- | ---: | ---: | ---: |
| Complete source-to-Object process | 17.339400 s | 15.767929 s | -9.06% |
| Internal compiler phases | 17.287224 s | 15.705360 s | -9.15% |
| Parsing | 2.089976 s | 1.770118 s | -15.30% |
| Frontend | 2.401052 s | 2.473214 s | +3.01% |
| Native preparation | 2.013189 s | 1.864738 s | -7.37% |
| Source CFG construction | 2.378184 s | 2.109015 s | -11.32% |
| Main cleanup including compaction | 3.458119 s | 3.104505 s | -10.23% |
| Helper generation | 2.009557 s | 1.744350 s | -13.20% |
| Retired instructions | 256.254418 G | 232.571358 G | -9.24% |
| Maximum RSS | 5,913,161,728 B | 6,023,917,568 B | +1.87% |

Encoding (+2.52%), artifact output (+0.76%) and peak footprint (+1.92%) also
increase. All stages, observations and blocks remain in the
[complete result](../../benchmarks/mech-deflate/native-profile/current-native-pgo-apple-m3-20261115.json).
All seven held-out programs lower mean compile process time, internal compiler
time and instructions; both process-time blocks improve in each case. These
are compilation measurements, without a pooled tiny-program percentage.

## Output equivalence and evidence

Every validation, screen and primary compile preserves source, bytecode, Object
and complete metadata. Every complete executable retains 1,878,568 bytes and
SHA-256 `557d0f5ef2b979e01851ae23b5b3944453a0bd9e772d68161be198916ca06c2c`.
No grammar/language change, weaker proof, scalar retirement or fresh program
runtime campaign is claimed.

The initial read-only CPU metadata query is denied by the default sandbox after
source and Rust configuration capture, before any build or test. Its script,
logs and source-state remain. Remaining metadata is resumed with scoped command
approval, reusing the immutable source snapshot. This is separate from compiler
builds and program timings.

The twenty-ninth
[repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
includes all raw/merged profiles, build/training/timing logs, exact scripts and
their derivation, source configuration/harness, program/fixture/executable bytes,
native inputs, initial setup failure and pure replay. Compiler binaries and
intermediates are excluded and SHA-pinned; unchanged compiler source is pinned
by Git revision. A fresh extraction of the repository archive verifies every
manifest/file/Git blob and replays the compact result byte-for-byte without
rerunning builds or measurements.
