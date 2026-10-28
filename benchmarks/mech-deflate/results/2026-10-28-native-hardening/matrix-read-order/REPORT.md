# Unsigned matrix read ordering repair

Performance checkpoint: `f7e17d7c88730e1308c7cc6cbe3e17333061a2c5`.
Reproducer tests and protocol: `57abc1daf9c2c1223e6aed15f21f22094ec195a2`.
Corrected implementation: `d19532830206dc51df584c1f39b5d42724305c34`.
Stack parent: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The [review finding](https://github.com/mech-lang/mech/pull/973#discussion_r4401440082)
is reproduced before changing production code. A 2x3 canonical U8 matrix with
cells [128,2,3,4,5,6] returns 2 at linear selector 2 in native execution;
the maintained resident execution returns 4. The corrected U64 test also fails.
The old implementation passes four read tests and fails these two oracle tests.
Its reader/numeric files are byte-identical to published head 88b9b89a3;
`red-provenance.json` records that comparison. The selected read methods are
absent from the clean stack parent. This defect was introduced in the compression
stack and predates the current metadata optimization.

The repair reuses the existing numeric dimension validator. After the original
one-based index/error check, it converts column-major linear selection to
row-major canonical storage. Vector offsets are unchanged. One translation
precedes every unsigned storage branch; current dimensions are read on each
visit. Selection/admission, scalar lanes, resource charges and publication
remain unchanged. Production changes add an 11-line translation and widen one existing helper's
visibility within the resident owner. No new language construct or compiler route is added.

The previous U64 test asserted direct storage order. It now compares the full
resident result and an independent literal column-order sequence. A new test
covers U8/U16/U32/U64, original/decoded artifacts, direct/function-local calls,
and changing shapes 2x3, 3x2, 1x6, 6x1, 2x2, 0x3 and 3x0. Invalid/zero/oversized
indices preserve prior publications and aliases, release temporary accounting,
and allow subsequent successful calls. Zero clone allowance remains enforced.

## Repeated small timing

Apple M1, unchanged eight-byte level-12 input `abcdefgh`, **Roc = 1x**:

| Subject | Median seconds | Divided by fresh Roc |
| --- | ---: | ---: |
| Pinned reference, 20000 observations | 0.000290 | 1x |
| Before correctness repair | 0.491693208 | 1695.49x |
| After correctness repair | 0.490911542 | 1692.80x |

The 0.16% elapsed difference does not establish a performance improvement.
The preceding metadata optimization's separate comparison was 1796.31x to
1702.31x (5.23% less elapsed time); reference denominators differ between
cohorts. Each subject here retains three primary calls, an excluded warmup and
an excluded extra call. The separate two-second profile and all nine profiling
repetitions are excluded from primary statistics.

Before executable: `6a9080cdaa8943c370253555ca33a39e2d001cceb5cc7052cbee875b4604790f`.
After executable: `929f96c685c155f632a0b099660c4c5246e05667f389898063129b3c7247f1ae`.
The same selected catalog, unchanged harness, source/artifact, pinned mimalloc
object and evaluate_into-only boundary are used. No compilation overlapped
timing. Exact raw bytes, independent inflation, work **941491317987**, aliases,
capacity 13 and zero terminal managed bytes/nodes pass every call. Source,
algorithm, all 25 components, tables, buffers, search/EOF limits, six Huffman
initializers and allocation sites are unchanged. No larger input was timed.

The real-buffer witness passes all 24 observations with unchanged allocation
requests, bytes and work: eight stores use 8833 requests and 5177904 bytes for
the U16 tree, 8833 requests and 49000016 bytes for the U32 cache. Physical copies
remain. The separate profile still shows allocation, budget and snapshot
bookkeeping; it supplies no evidence for a new optimization in this repair.

## Validation and disposition

**80 Rust tests pass at this implementation:** 69 existing/new component tests,
including corrected reads, narrow words and match-finder operations, plus eight
native scalar and three bit-operation tests. Test bodies and allowances are
unchanged except the explicit oracle correction and new regression. The all-
primitives scalar case remains skipped because not/xor are outside this catalog.
The source compiler and decoded re-encoding reproduce the exact pinned artifact.
All twelve architecture/catalog/routing/inventory/warning/boundary/format checks
pass. Protected scalar controls retain exact allocation counts/bytes and pass
the predeclared 1.05 gate (ratios 0.9984–1.0475); scalar-loop allocation slopes
remain zero. The reduced release rebuild takes 449.31 seconds and reuses all
machine libraries. Prior metadata-round tests retain their own revision rather
than being counted as fresh passes here.

The initial new-test compilation needed an explicit Box<[u64]> annotation;
that failed receipt is retained separately from the real red test result.
The corrected six-test read cohort fails twice on the old implementation and
passes as part of the final 69-test cohort. No production failure remains in
the executed scope. Linked subprocess/runtime-only application builds and
full-input qualification were not rerun. Historical whitespace findings remain
assigned in the earlier review packet. CI is cancelled by request; this remains
a draft PR pending fresh review, with no green-CI or merge-readiness claim.
