# Managed projection batching: fewer instructions, neutral time

**Keep the retained compiler.** Combining adjacent managed buffer projections
reduces hardware instructions by **0.25198%**, but both unchanged timing cohorts
remain mixed and neutral. No elapsed gain is established. The complete source
patch is preserved in `candidate-source/`; the compiler, tests and CI workflow
are restored exactly to the published starting checkpoint. No established small
positive elapsed improvement is discarded.

The fresh profiles and disassembly narrow the next target to **ordinary record
packing and scalar record-field admission**. Merely combining callback boundaries
leaves the original per-field validation, publication and retirement work intact.

## Exact revisions and design

Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
Published starting head: `0d31783d7d7cbb4d0ee411710f77fd1c317d7126`.
Retained compiler source: `552e537fcbcfc3811ce442c982010b5e1e999e6c`.
Candidate implementation: `0e4ff5d90f65c77becdec29a16424190522dd855`.
Protocol acceptance tests: `79c3f33621a015c60b91818579764a3043d05f7c`.
Frozen release head: `fc86a1722e35f3b8e280049cd294361310226a6b`.
Restoration commits: `979654522c345908a302e417191a0281e7014743` and
`e0c231b90668989599203210f75223804a4fb180`.
The first restore script stopped at its new-file detection guard after restoring
four existing files; the second uses the tree inventory and completes all files.
An explicit final tree diff proves source/tests/workflow equality to the starting
checkpoint. No partially restored build is used for observations or validation.

The candidate derives immutable same-source managed U8/U16/U32/U64 projection
runs from the maintained plan, frame layouts, visits, predecessor facts, native
pieces, closed regions, owner regions and parallel regions. Interior sites have
one forward predecessor and the same iteration; assertions, activation, loop
progress, exits and selected native regions are excluded. Source preparation
and build-owned image restoration independently derive the same layout.

One typed helper combines native/Rust callback boundaries. It still invokes
the original `Invocation::visit` for every interior field and the unchanged
`buffer_frame_action` for every field. Thus schema checks, charges, temporary
owner drops, lexical retirement, error origins and failure cleanup stay ordered.
No arithmetic, source control or operation stream is interpreted by the helper.
There is no new mutable value bank, owner, loan, executor or budget price. The
original emitted path remains available on refusal before any action.

The private helper requires native ABI 38 in the experiment; IR/work/bytecode
remain 131/95/14. After restoration the retained versions are **131/95/37/14**.
There is no canonical bytecode revision or fixture migration in this experiment.

## Primary timing, including the neutral confirmation

Only complete level-12 compression of `abcdefgh` is timed. Each cohort uses four
independently prepared old/new linked executable pairs, 16 rotating-order blocks
per pair and 31 calls per version per block. Both workers are ready before
measurement; only the requested worker executes. No build, test, audit, profile
or instrumented diagnostic overlaps primary. Each reference, pair and summary
is committed immediately. `MEASUREMENT.md` fixes the first method;
`CONFIRMATION.md` fixes the unchanged confirmation before execution.

| Cohort | Retained baseline | Candidate | Shared Roc reference | Multiples, before → after | Change |
| --- | ---: | ---: | ---: | ---: | ---: |
| First four pairs | 4.3876875 ms | 4.3887915 ms | 0.285 ms | 15.39539× → 15.39927× | +0.02516% |
| Four unchanged confirmation pairs | 4.3770420 ms | 4.3766875 ms | 0.287 ms | 15.25102× → 15.24978× | −0.00810% |

Each cohort has two improving and two regressing pairs; descriptive improving
blocks are 27/64 and 35/64. Blocks share preparations and are not independent
executable preparations. There are **7,936 primary calls**, 7,968 including
warm-up/final, and two separate 20,000-call references. Every unfavorable result
is retained. The fresh reference denominator changes between cohorts; that is
not a compiler gain or regression. Prior validated baseline readings around
15.08× used their own 0.291 ms reference and remain valid.

Each call checks the exact ten raw bytes, independent inflation, unchanged input
contents/storage aliases, output capacity 13/reuse and exact work
**60,260,219,686**. Every worker finishes at zero managed bytes/nodes. The complete
Mech source/bytecode, pinned algorithm, tables, depth 300, nice 258, original
near-EOF limits, 262144-entry U16 tree, 1500513-word U32 cache, 305257-word U64
path capacity, source allocation sites, pinned mimalloc 2.1.7 and original timer
remain unchanged. No larger input or throughput result is claimed.

## Hardware work and memory

These are separate linked diagnostic cohorts, excluded from primary elapsed.
CPU medians use 32 steady calls per executable; allocations use eight Mech and
32 reference calls. Hardware instruction/cycle deltas include the recorded
uncorrected boundary-read overhead of about 5,010 instructions. All observations
have zero page-ins. Logical work is a different quantity.

| Per compression | Roc | Baseline | Candidate |
| --- | ---: | ---: | ---: |
| Actual hardware instructions | 1,364,372 | 49,005,195 | 48,881,711.5 |
| CPU cycles, diagnostic | 890,968.5 | 13,818,663 | 13,996,810 |
| Allocation requests | 95 | 13,994 | 13,994 |
| Requested allocation bytes | 9,021,923 | 98,586,843 | 98,586,843 |
| Additional peak tracked live heap | 9,257,328 | 20,575,432 | 20,575,432 |

Instructions fall 123,483.5 (0.25198%); diagnostic cycles rise 1.28918%. Neither
alone replaces the isolated latency result. Large backing requests remain
**35 path / five tree / one cache**. No copy or backing allocation is removed.
Prepared tracked heap rises by 6,000 bytes, from 141,595,920 to 141,601,920.
Generated compressor text rises 13,885,696→13,898,456 bytes and stack
108400→108416 bytes; the experiment contains 107 static batches over 282 sites.
Static sites are not reached visit counts.

Whole-task linked footprint is 227,804,864→227,919,616 bytes, and RSS
318,783,488→318,980,096. These prepared executables include setup/compiler/catalog
state. Additional tracked peak excludes direct VM/private heaps and transient
overlap internal to realloc; it is not total RAM. Both APIs receive whole input
already in memory; neither streams file input during the timer. Total copying,
branch/cache misses, bandwidth, energy and input scaling remain unmeasured.

## Fresh profile and the next bounded target

Fresh main-thread samples use 695 calls per version, excluded from primary.
Baseline partitions 1,151 weights into frame actions 304, ordinary visits 180,
other helpers 157, selected library calls 101 and remaining 409. Candidate
partitions 1,008 weights into 268/156/128/100/356. Frame actions remain about
**26.4% and 26.6%**. These include surrounding harness work and are not precise
compression-timer percentages or promised optimization bounds.

The same unchanged `buffer_frame_action` is inspected in both executables.
Sampled offsets +2500 and +4056 are return/result handling immediately after
calls to **`admit_buffer_frame_words`** and **`admit_buffer_record_word`**.
Their full disassembly and exact commands are in `post-counter-cleanup/`.
These offsets identify reached admission paths, not proof that the subsequent
result-discriminant load itself consumes all sampled time.

The maintained source already has immutable pack shape/footprint recipes, so
redoing that general work package is unwarranted. A more specific repeated cost
remains in `native_collection_buffer_frames.rs`: a scalar field projection has
an already resolved ordinal, yet `check_buffer_record_ordinal` invokes
`named_schema_ordinal`, hashes field names and repeats character accounting on
each visit. This is the next operation to measure and simplify.

Before changing it, establish a minimal compiled private-record reproducer that
actually reaches this library route. Prepare only immutable field-name/lookup
facts; keep current selector IDs, scalar words, shapes and admission fresh.
Acceptance must cover changed selectors/arguments, unknown and colliding IDs,
all supported scalar types, original/decoded and restored images, exact spent
prefix/error order at finite compute/comparison/memory caps, prior publication,
aliases, cleanup/retry, and the old source/component goldens unchanged. Preserve
the ordinary fallback when a proof does not hold. Then repeat the same complete
eight-byte timing/counter/memory/profile scorecard before claiming a gain.

## Validation, failures and skipped coverage

The candidate passes **315 maintained runtime tests and 31 work/memory/value
tests**, with warnings as errors and opt 1. The two new projection cases are
inside the runtime count, not counted twice. They exercise original/decoded
programs, original/restored build-owned images, the actual emitted batch and
its forced original per-field path, exact work and error-receipt agreement,
changing recursive Boolean/current arguments, nested/repeated calls, ordered
assertion/load/arithmetic errors, all four buffer widths, aliases, unchanged
publication, cleanup and retry. Malformed, foreign and repeated helpers reject
through the real callback and recover. All six resource dimensions retain
their exact/one-short boundary checks. Existing Huffman work 91,452,918 stays.

A separate actual linked executable passes **12 executions** across original
and decoded programs: valid inputs, assertion before invalid load/arithmetic,
load failure, arithmetic failure, changed valid inputs and recovery. It verifies
expected outputs, prior publication/aliases and zero final cleanup. These are
linked program executions, not 12 additional Rust test functions.

The initial broad selection reports three introduced failures from old ABI-37
test expectations; updating them to the experiment's ABI 38 fixes them. They
return to 37 with the restored source. The fourth failure is inherited in this
reduced catalog: the knucleotide test imports unavailable `stats`. A fresh build
of the exact clean-parent source reproduces that one failure while all three
ABI tests pass. It remains excluded from the established 315-test selection;
no unrelated catalog/compiler fix is added.

Diagnostic adapter errors are preserved: an initial work-only filter ran 16
passing cases while expecting the broader 31 (corrected to work/memory/value);
a copied CI adapter added three map tests beyond the unchanged workflow's
15-case selection (corrected to its exact filters). The first source-restoration
guard's new-file diagnostic was corrected before final validation. These are
tooling/setup failures, not product regressions. Initial helper-symbol matching
was narrowed to the actual function rather than its closure symbols; allocation
analysis reads allocator receipts from stderr. No primary observations are lost
or replaced in these corrections.

The optimized release completes in **77.931 seconds**, opt 3, locked/offline,
incremental off, one job, Rust nightly 2026-03-03 (`ec818fda3`), LLVM 22.1.0,
aarch64. All **seven non-host library hashes remain unchanged**, including
Engine and selected math libraries. No full math rebuild is performed. Exact
commands, configurations, hashes, stdout/stderr and test counts are archived.

The final restored build lists and executes the **unchanged 15-case CI
selection**, passes **132 configured workflow/inventory Python tests**, final
format/diff checks, and an explicit source/test/workflow equality diff. No new
unresolved introduced correctness failure is known in the restored compiler.
The removed candidate's broader 234-component/eight-composition qualification,
scalar/frame performance controls and architecture audits are not newly claimed.
Full Core/Engine/all-feature suites and larger inputs are also not newly run.
Prior baseline receipts remain authoritative. Hosted CI is cancelled by user
direction and is not reported green; PR #973 remains draft and review is
requested after publication. Parity remains outstanding at 1×.

`scorecard.json`, `scorecard.csv` and `analyze.py` preserve both cohorts and all
diagnostics. `python3 reconstruct.py EMPTY_DIRECTORY` independently checks every
archive entry's path/length/hash and regenerates both scorecards byte-for-byte,
without compiling or executing a benchmark. `reconstruction.json` records the
observed **39 archives, 273 file occurrences and 218 unique paths** verified.
The source patch and unchanged compressor input pins are preserved; the full
canonical compressor source/bytecode remain in the preceding round-40 packet.
