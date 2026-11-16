# Parallel compilation on the allocator/schema-owner compiler

This report preserves the frozen 12c500 compiler comparison. The later
[expanded cleanup-worker qualification](native-parallel-phase-cleanup-compile-profile.md)
measures source 0c949505cc: 15.554 seconds serial versus 15.218 seconds with two
workers (2.16%). Its cohorts and source remain separate from the result below.

The implemented helper machine-code workers save **0.167 seconds (1.03%)**
on its frozen level-12 Deflate build: **16.182550 seconds serial versus
16.015580 seconds with two workers**. Both balanced blocks improve and every
complete executable byte matches. This is a small gain; compilation below
ten seconds remains open. Default `MECH_COMPACT_COMPILE_THREADS=0` stays serial;
`MECH_COMPACT_COMPILE_THREADS=2` enables the measured worker configuration.

## Scope and timing

The frozen compiler source is `12c50010866066952345c63223bce770cf7578b1`, with
the retained compiler allocator and shared control schema owners. Both subjects
use the same immutable compiler executable, SHA-256
`c5b0b57e2cc870d3e024d2d2c38c9a478122da5f72faf3bbaa3a868cb494a56f`,
and the same 61 program flags. Only worker count zero/two differs. All seven
[worker implementation](native-parallel-phases-compile-profile.md) files are
unchanged since `1090de011646c92e30c2d4b648ed0792c50f300d`.

Eight complete builds run one at a time in ABBA/BAAB order on the Apple M3,
after two separate complete byte gates. No heavy build, test, sampling, hashing
or archival overlaps the primary observations. The boundary includes fresh
source assembly, parsing, checked frontend, native preparation, optimization,
machine-code generation, Object/metadata writes, Object handoff and final link
process completion. The fixed runtime/entry adapter and native inputs are
cached and hash-pinned. Its previously recorded first-use construction of
11.882560208 seconds is excluded; the program compilation is fresh every run.

| Boundary | Serial | Two workers | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 16.182550 s | 16.015580 s | -1.03% |
| Source-to-Object process | 16.048301 s | 15.881243 s | -1.04% |
| Final native link | 0.133106 s | 0.133085 s | -0.02% |

The two complete-build blocks improve 1.38% and 0.68%. All observations remain.
Internal compiler phase totals improve 1.05% and native time 1.30%; parsing
increases 0.15%, catalog time 1.01% and Object handoff 9.46% (about 0.108 ms).
Stage and link movement are observational; this scheduling experiment changes
no parser, frontend, handoff or linker code. No new instruction/RSS counter
cohort or exclusive cleanup timing is claimed.

The workers compile owned helper function Contexts to machine code. Function
construction and custom cleanup, parsing, checked frontend, native preparation
and the main function remain on the coordinator. The larger architecture that
moves helper construction and cleanup into jobs remains unimplemented.

This comparison is separate from the earlier smaller-program 1.57% gain, the
[updated-workload result](native-refreshed-parallel-compile-profile.md) with
mixed blocks and a 0.16% slowdown, and the
[shared-owner change](native-control-schema-owners-compile-profile.md).
Their timings and percentages are not pooled or added.

## Correctness and evidence

Both preliminary gates and all eight primary builds reproduce the entire
source, bytecode, Object and complete metadata. Every complete executable has
2,060,440 bytes and SHA-256
`d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
There is no emitted-program change or new program-runtime timing campaign.
No compiler source change, grammar change, weaker proof or scalar retirement
is introduced by this requalification.

Two preceding current-compiler stack diagnostics also reproduce all four
outputs and the complete executable. Both are excluded from primary timings.
Raw/demangled stacks, 965 observed symbol mappings and the Call-graph-only
analysis remain. Waiting and inclusive stack counts are not CPU shares.
The named leaves include SSA nonlocal lookup, register allocation, dimension
canonicalization, copying and parser journal mutation; they do not establish
removable time or a measured optimization opportunity by themselves.

The first measurement setup references the wrong source-proof filename and
fails before any compiler executes. Its original script and traceback remain.
The corrected driver uses the existing qualification-commit receipt. An optional
process-list shell command is denied by the sandbox; there is no automatic
approval-review rejection. No build or test is added for unchanged compiler code.

The thirty-seventh
[repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
retains every command/log, output/executable, source/worker snapshot, fixed
native input, profile and setup failure. Compiler and demangler binaries are
excluded and hash-pinned. Fresh extraction verifies every manifest/file/Git
blob; pure replay reconstructs all timing summaries and stack analysis and
checks every output without rerunning compilation or timing. The
[complete numerical result](../../benchmarks/mech-deflate/native-profile/current-parallel-apple-m3-20261115.json)
contains all phase values and adverse observations.
