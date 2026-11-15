# Shared schema owners through lexical controls

Retaining existing schema owners through control captures and parameters
improves complete Deflate source-to-executable compilation **16.903674 to
16.083709 seconds (4.85%)**, with both balanced blocks faster and every
executable byte identical. Frontend time falls **34.41%** and compiler maximum
RSS falls **32.99%, from 6.701 to 4.490 GB**. This change is retained.
Compilation below ten seconds remains open.

Qualified source `12c50010866066952345c63223bce770cf7578b1` has exactly the
measured four-file diff from `e7b4590c48957206d5b24b2d989802976dd91bda`:
SHA-256 `320944e9ee9f5509a1379d5ae3eeb4921af47598a9034ce1201c62999f41ea46`.
The source commit precedes this evidence publication; its complete diff,
parent, paths and source snapshots are retained and checked.

## Removed work

Private control captures and parameters previously materialize independent
`SchemaDraft` copies from an existing `PendingSchema`. They now retain that
same immutable owner. Existing pure canonicalization results survive through
validation, schema-tree visitation and schema-table handoff. Mutation keeps
the existing copy-on-write behavior and invalidates the canonicalization cache.
Raw structural-pattern drafts keep their ordinary uncached finalization.

Selection preparation also borrows its input schema while constructing the
original fresh remapped inventory, avoiding the preceding full draft copy.
The existing no-operation paths still materialize their owned result.

No new cache, public API, grammar rule, input dependency removal, dimension
inventory pruning or identity-limit change is introduced. All original
validation call sites remain; pure canonical data can reuse the result for
the exact immutable owner. Shape, factory, current-visit and admission checks
retain their existing paths. Schema-table lookups still happen at handoff.

## Complete-build qualification

Both subjects use the retained pinned compiler allocator, standard compiler
profile, current Deflate source/runtime, the same 61 program flags and two
workers. No PGO, native-CPU or emitted-program allocator change is involved.
The control is the qualified `d834508336` compiler with binary SHA-256 beginning `fe37`;
the candidate binary SHA-256 begins `c5b0`.

Eight primary complete builds run serially in ABBA/BAAB order with no concurrent
heavy build, test, sampling, hashing or archival. The boundary covers fresh
source assembly, parsing, checked frontend, native preparation, optimization,
code generation, Object/metadata writes, Object handoff and final link completion.
Fixed runtime/adapter inputs are pinned and cached; first-use adapter construction
is excluded.

| Boundary | Retained control | Shared owners | Change |
| --- | ---: | ---: | ---: |
| Complete source to executable | 16.903674 s | 16.083709 s | -4.85% |
| Source-to-Object process within those builds | 16.765964 s | 15.956388 s | -4.83% |
| Final native link | 0.136733 s | 0.126357 s | -7.59% |

The two complete-build blocks improve 4.36% and 5.33%. Every observation remains,
including the faster first control block. Link movement is observational;
there is no linker change or independent linker improvement claim.

A separate eight-run source-to-Object cohort uses the same settings and order:

| Work | Retained control | Shared owners | Change |
| --- | ---: | ---: | ---: |
| Internal compiler phases | 16.817408 s | 15.901183 s | -5.45% |
| Complete source-to-Object process | 16.895392 s | 15.966643 s | -5.50% |
| Checked frontend | 2.515671 s | 1.649933 s | -34.41% |
| Instructions retired | 238.125 billion | 227.672 billion | -4.39% |
| Maximum RSS | 6,700,613,632 B | 4,489,805,824 B | -32.99% |
| Peak memory footprint | 6,679,209,414 B | 4,467,035,832 B | -33.12% |

Both Object-cohort blocks improve compiler time, frontend and instructions.
Parsing increases 0.16%, native preparation 0.52%, cleanup including compaction
0.64% and catalog time 0.40%; these adverse observations remain. This measures
one combined owner/borrowing change, without separate component savings or an
additional parallel gain. Earlier allocator, smaller-program and PGO percentages
are not added. All stage clocks and observations are in the
[complete result](../../benchmarks/mech-deflate/native-profile/control-schema-owners-apple-m3-20261115.json).

## Evidence and verification

Two preceding stack diagnostics use the current control compiler after the
allocator change. Both reproduce all program outputs and the complete executable.
Call-graph analysis identifies dimension canonicalization, SSA nonlocal lookup
and bulk copying as remaining work. Deeper copy paths point primarily to
recursive control-frame storage, with field selection and function-argument
embedding also present. Waiting and inclusive stack counts are not CPU shares
or exclusive phase timings. The parser is bounded to the call graph and excludes
the later flat summary table; raw-to-demangled symbol mapping and the saved
analysis replay exactly. Sampled observations stay outside all primary means.

All source, bytecode, Object and complete metadata bytes match in every
qualification observation. Fourteen paired numeric/scalar fixtures pass. The
two preliminary cached executable gates, eight primary complete builds and
later default-serial gate all reproduce the entire 2,060,440-byte executable:
SHA-256 `d6c65cc3fb18acc4be77dce329962105bf0f2b56dd2623a12f466256463eca61`.
The serial observation is an output gate without a paired speedup claim.
No fresh emitted-program runtime timing campaign or scalar retirement is claimed.

The unchanged owner/annotation modules pass all 13 existing tests for cached
success/errors, copy-on-write isolation, mutation invalidation, axis identity
and concurrent readers. This foundation harness does not compile the changed
frontend sites; complete-program and fixture gates cover those sites. The real
native CLI check passes with `cranelift_build` in 22.006 seconds. The frozen
compiler check/build take 19.722 and 376.338 seconds, excluded from compiling
Deflate. The first draft check also passes; it remains as a separate setup record.
The earlier default-feature CLI failure documented in the allocator qualification
is not repaired or claimed to pass here.

The thirty-sixth [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md)
contains all four source snapshots and the complete patch, whole-commit proof,
build/check/test logs, fresh selection profiles and analysis, fixtures, every
observation and complete output, fixed native inputs and pure offline replay.
Compiler/test/demangler executables and dependency libraries are excluded and
hash-pinned. A fresh repository extraction verifies every manifest/file/Git blob,
then reproduces the compact result byte for byte without compiling or timing.
