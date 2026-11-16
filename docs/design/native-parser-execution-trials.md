# Parser execution work and independent trials

Fresh diagnostics identify substantial repeated recognition in the existing parser. Three independent standalone prototypes reduce parser time by 2.51%, 4.63% and 2.16%. Each misses the predeclared 10% promotion threshold. None is retained or complete-build qualified. The selected ThinLTO/PGO compiler `8043` and its original 13.414825-second complete-build primary result remain unchanged.

## Existing parser work

The frozen level-12 source has 164,243 bytes and 1,981 lines, SHA-256 `55cb2e698afb73da536164504b991dd514be855c9f4da9c3aadd046a55e1dba4`. The fresh physical census records 29,739,969 base frames, 5,296,706 operator frames, 13,617,156 checkpoints, 9,099,515 rewinds and 9,250,556 mutable journal-page accesses. These are overlapping work counts, not an exclusive time budget.

A second census records successful ordinary rule exits by source position:

| Existing rule | Successful recognitions | Distinct successful starts | Recognitions per start |
| --- | ---: | ---: | ---: |
| identifier | 164,885 | 15,914 | 10.36 |
| digit-sequence | 124,770 | 3,026 | 41.23 |
| identifier-path-segment | 36 | 6 | 6.00 |

These groups use source position only. Different consume/context bounds may share a position; the counts do not prove all recognitions can safely be memoized. Fixed-literal early cache returns bypass ordinary exits. The selected identifier and digit-sequence successes use ordinary exits. Next measure exact rule, start and source-view bounds before designing bounded recognition reuse. Fresh node identities, ordered errors, resource admission and original grammar must remain.

A separate ten-second, one-millisecond CPU sample runs twelve complete parses using the actual libraries and mimalloc selected by compiler `8043`, with its PGO profile. Of 7,732 leaf samples, 7,676 have parser ancestry. Named leaves include base-continuation advance (2,011), tree-cache advance (1,253), literal advance (477), marker completion (382) and parser start (310). Inlined work remains attributed to its enclosing function. These sampled stacks do not establish exact exclusive percentages or additive savings. The parser-only driver's link may change inlining relative to the complete compiler. Every diagnostic elapsed time is unqualified.

## Independent standalone screens

Each variant starts from the unchanged production document module. The three changes are not combined. Each screen has eight fresh serial ABBA/BAAB observations, four per subject, after tests and builds finish. The internal timer wraps the complete parse including tree construction, excluding JSON serialization. Whole-process observations also remain in the packet. These coherent release-dependency, System-allocator, opt-level-three screens are separate from the selected PGO compiler; their roughly two-second times must not be pooled with its roughly 1.51-second parse.

| Prototype | Control seconds | Trial seconds | Change | Changes in the two blocks | Units passed |
| --- | ---: | ---: | ---: | --- | ---: |
| Grammar-derived ASCII terminal actions | 2.089431 | 2.037044 | -2.507% | -3.028%, -1.992% | 526 |
| Release finished cache request before publication | 2.088804 | 1.992017 | -4.634% | -4.218%, -5.043% | 526 |
| Bounded ASCII Unicode-scanner window | 2.113036 | 2.067356 | -2.162% | -2.104%, -2.220% | 527 |

Promotion required at least 10% parser mean reduction, both blocks faster, exact complete parses and all unit tests passing. All complete outputs match; each timing gain misses the threshold. No expensive candidate PGO or complete-native qualification was started, and no complete-build speedup is claimed.

The classified-action prototype derives ASCII terminal/alias actions from the existing shared base specification. It retains the original scanner, token emission and logical resource accounting, with fallback for unsupported Unicode and finite-budget cases. Its internal cache-storage test originally required nested entries eliminated by fusion; the corrected assertion retains the same cache-owner/storage requirements for remaining entries. The initial failure, original assertion, exact repair and all subsequent results remain. A missing unchanged fixture tree also caused an initial standalone test-build failure, before any test or timing ran.

The cache-publication prototype drops only the completed request and immutable sink journals before mutating the current cache page, retaining the same identity check and publication order. This may reduce copy-on-write work, but allocation counts were not separately measured. All 526 original document tests pass unchanged.

The ASCII-window prototype supplies previous/current/successor ASCII bytes in one existing Unicode scanner call while preserving the original charged work, scanned bytes and exact cursor state. It falls back for Unicode, EOF, limited allowances and suspended scan states. A new independent exhaustive ASCII-window test compares the original path, cursor and accounting, alongside all 526 original tests. A generator quoting error and following missing-driver invocation occurred before compilation; both failed commands and the repair remain.

## Reproduction and disposition

All 28 full parse outputs are byte-identical: 12,802,946 bytes each, SHA-256 `09fb23613e8ce330c3874ecea80ef464bad60a8f60c182a384cebf00366055ae`. The packet stores them losslessly in a nested solid XZ archive and records every original logical path. It retains source snapshots, patches, original fixtures, tests, commands, failed setups, raw/demangled samples, census bitsets, every timing observation and all adverse fields. Thirteen compiler/probe binaries are excluded and hash-pinned. A failed first profile link lacked the host dependency search path; its command/logs and corrected successful link remain.

Production source stays `ef5e255e817eb5bbc6df61b094a3741b31c49da2`; no grammar, language, compiler setting or emitted-program change is retained. The fifty-third [repository packet](../../benchmarks/mech-deflate/evidence/compiler-profile-20261114/README.md) is freshly extracted, manifest-verified and replayed byte-identically to the [compact result](../../benchmarks/mech-deflate/native-profile/parser-execution-trials-apple-m3-20261115.json). Run `python3 replay.py` in the extracted archive for the pure offline derivation. Compilation below ten seconds remains open.
