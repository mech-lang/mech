# Round 38: completed-call buffer ownership and executable scorecard

The general completed-call handoff fix removes four large path-buffer requests.
Requested allocation bytes fall **9.01545%** and additional peak live usable heap
falls **20.31194%**. It establishes **no speed gain**: actual instructions rise
0.01579% and the four-pair elapsed median rises 0.33353%. Keep that small latency
regression explicit; retain the independently demonstrated memory reduction.

Only complete level-12 compression of the eight bytes `abcdefgh` is measured.
Both implementations produce exactly `[75,76,74,78,73,77,75,207,0,0]`; independent
inflation, input contents/storage aliases, capacity 13 and final zero managed
bytes/nodes are checked. There is no larger-input throughput or parity claim.

## Revisions and maintained behavior

- Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.
- Starting published head: `ba198160dc51162da49d962866fe4a19e0aa051c`.
- Frozen comparison compiler: `3a54f1a1d64db9eee01db97763a3f95a138db93a`.
- Regression first: `43bf897c5`; retained-future-seed/recovery test: `c40387e2e`.
- Ownership implementation: `f64c9e65f26e6fc47cd16c0ea419643712196fbd`.
- Separate inherited feature repair, and final measured compiler source:
  `dc77024f29f373f0938695531ab2b7c975bb1e7e`.
- IR/work/ABI/bytecode remain **131/95/37/14**. Subsequent commits record evidence
  and documentation, not additional production changes.

The pinned source SHA remains
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`,
and bytecode SHA remains
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
The pinned mimalloc 2.1.7 object is unchanged, SHA
`38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`.
The reference binary remains
`3941ef59b2c9343d0493cc6b6d14ca4803e6bd27acfe520b1fb59d9b63026324`,
with package revision `671077457ff7ef3a3d44caf715a3fc595addf379` and original
compiler revision `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`.
The earlier verified package/source receipts remain in `../deep-profile/`;
deleted reference checkout Git metadata is not presented as a fresh HEAD check.

Algorithm, tables, depth 300, nice length 258, near-EOF limits, the 262144-entry
U16 tree, 1500513-word U32 cache, 305257-word U64 path capacity, source allocation
sites, output reuse and timing boundary remain unchanged. Mech performs the
compression. No new owner/executor, source specialization, resource allowance,
language construct or revision change is introduced.

## Executable comparison

All ratios in the last column use Roc in that same row as **1×**. Elapsed, CPU,
allocation and sample-profile observations are separate cohorts. MB is decimal.

| Per complete compression | Roc | Mech before | Mech after | After/Roc |
| --- | ---: | ---: | ---: | ---: |
| Primary elapsed median | 0.291 ms | 4.6224375 ms | 4.6378545 ms | 15.93764× |
| Actual CPU instructions | 1,364,372 | 50,664,319.5 | 50,672,317.5 | 37.13966× |
| Actual CPU cycles, diagnostic | 931,804 | 14,651,785.5 | 15,782,050 | 16.93709× |
| Allocation requests, including realloc | 95 | 14,002 | 13,994 | 147.30526× |
| Requested bytes | 9,021,923 | 108,355,555 | 98,586,819 | 10.92747× |
| Additional peak live usable heap | 9,257,328 | 25,819,968 | 20,575,432 | 2.22261× |
| Reallocation calls | 0 | 1,213 | 1,213 | — |
| Moving realloc calls | 0 | 458 | 458 | — |

The paired elapsed result is **15.88466× → 15.93764×** the shared 0.291 ms
reference. Against the retained historical 0.300 ms ruler it is
**15.408125× → 15.459515×**. Do not compare ratios with different denominators
as though they were compiler improvements. All four independently prepared pairs
are retained: +0.16612%, +0.63177%, +0.51676%, +0.00136% elapsed. Only 26/64
descriptive blocks improve; blocks share four prepared pairs.

CPU values are `proc_pid_rusage(RUSAGE_INFO_V4)` instruction/cycle counters, not
the logical work receipt. Both Mech empty-boundary calibrations retain median
5010 instructions over 101 observations; no silent subtraction is made. Every
retained steady CPU compression has zero page-ins. Cycle observations vary with core/
scheduling conditions; their +7.71418% does not replace the +0.33353% paired
elapsed result. Allocation instrumentation is excluded from these CPU counters
and primary timings. Moving realloc copy upper bound stays 13,905 bytes; this
is not a measurement of total copying.

## The ownership change and its limits

The read-only census observes 68 actual large-buffer calls, using weak frozen
identities and raw backing addresses without retaining strong buffer owners.
The existing private-frame proof already places loop seed/next/return reads at
their exact control visits. Unsigned buffer retirement had instead collapsed
those roles into opaque fusion reads, retaining completed callee return slots.
The fix shares the existing mechanical graph query. Unknown publication,
collection and effect frontiers stay conservative; future reads, fresh
definitions, backedges, runtime identities and true exclusivity remain checked.
See `CANDIDATE.md` and the source commits above.

| Large scratch backing requests per compression | Roc | Before | After |
| --- | ---: | ---: | ---: |
| U64 path capacity | 1 | 39 | 35 |
| U16 tree | 1 | 5 | 5 |
| U32 cache | 1 | 1 | 1 |

The new census confirms precisely four first appends now reuse their backing:
sites 12411, 15288 twice and 26887, each with 257 initialized U64 words. This
avoids **9,768,224 requested capacity bytes**, but only **8,224 initialized
payload bytes of copying**. Net small-request savings account for the remaining
512 bytes in the 9,768,736-byte total reduction. Spare capacity was not zeroed or
copied, which helps explain why allocation traffic drops without an elapsed win.

The remaining 34 extra path requests comprise 32 later appends with no usable
consume proof and two clears retaining private-frame aliases. The four tree
copies still copy 2,097,152 initialized bytes total. Do not erase those aliases
without a real future-read/control-flow proof. There are still 13,678 admissions,
9,156 helpers and 3,037,416 logical transport bytes in the actual current census.
Logical work changes 44,443,887,150→44,443,824,446 under unchanged prices; that
is distinct from retired CPU instructions.

The fresh main-thread sample partitions 1332 weights into frame actions 365
(27.4024%), ordinary control visits 180 (13.5135%), other invocation helpers 180
(13.5135%), selected library calls 142 (10.6607%), remaining 465 (34.9099%).
These are disjoint ancestry-based weights including some harness/native body
work, not precise timer percentages or promised optimization bounds. The earlier
1351-weight profile is retained as a historical comparison, not a paired profile.
Frame/control/helper/library plumbing still accounts for roughly 65% of weights.

## Linked execution and memory scope

A new diagnostic links the SAME unchanged emitted compressor Object/identity/
descriptor against the final measured libraries. The build hashes Object,
descriptor, bytecode and allocator before link; the maintained checked image
loader verifies the binding. Only the full compressor is moved ahead of time.
Two tiny seed/clear helpers retain their original JIT setup outside compression.
This is not a minimal runtime-only application or a new primary elapsed result.

| Diagnostic memory/counter scope | Roc | Mech full JIT | Mech linked compressor |
| --- | ---: | ---: | ---: |
| Whole-task RSS after preparation/compression | 14,778,368 | 1,690,976,256 | 318,963,712 |
| Whole-task physical ledger footprint | 14,418,816 | 16,864,446,272 | 228,001,472 |
| Tracked live usable heap at compression entry | 32 | 148,214,600 | 141,537,520 |
| Additional peak during compression | 9,257,328 | 20,575,432 | 20,575,432 |
| Actual compression instructions | 1,364,372 | 50,672,317.5 | 50,686,564.5 |
| Actual compression cycles, diagnostic | 931,804 | 15,782,050 | 14,266,775 |

The linked compressor retains exactly 13,994 requests/98,586,819 requested bytes
and the same peak increment in all eight steady memory observations. Its 32
steady CPU observations are also byte/work/alias/capacity checked. Both harnesses
finish with zero managed cleanup. The footprint drop strongly separates full
compiler/JIT preparation from compressor working heap; it does not prove which
preparation allocation created the excess. Footprint includes swapped memory and
is not resident RAM on this 8 GiB M1 machine. The former ~16.9 GB figure must not
be presented as runtime-only compressor RAM, nor the before/after JIT RSS
difference attributed to this ownership patch.

Both receive already loaded whole input; neither streams file input inside the
compression timer. Input handle aliases remain shared/unchanged. Total input
copy bytes, total memcpy bytes, cache/branch misses, bandwidth, energy and
input-size scaling remain unmeasured.

Fresh generated Object, metadata and descriptor are byte-identical to the prior
snapshot (respective SHA d2c0185a…, 9b9cfb5a…, 7b6cfa95…). Generated compressor
text stays **13,801,616 bytes / 3,450,404 four-byte slots**, stack 108,560 bytes.
Reference whole executable text is 195,456 bytes / 48,864 slots. The Mech JIT
diagnostic text is 48,193,356 bytes and linked diagnostic text 62,813,572 bytes;
those whole binaries include compiler/catalog/verification/setup code, so this
is not a fair minimal application-size ratio. Static slots include padding and
are not executed instruction counts. No code-size improvement is established.

## Validation, failures and coverage

All final execution uses the final libraries in `premeasurement/receipts.tar.gz`.
Exact commands, feature lists, environment, profiles, hashes, stdout/stderr and
nonzero counts are archived. Release libraries use opt 3; maintained diagnostic
test roots use opt 1. Rust is nightly-2026-03-03, rustc `ec818fda361ca216eb186f5cf45131bd9c776bb4`,
LLVM 22.1.0, aarch64-apple-darwin. Warnings are errors, incremental is off, builds
are locked/offline with one job; already-built math dependencies are reused.

- **584 distinct final selected tests pass:** 311 maintained runtime, 31 work/
  memory/value, 234 existing component-selection cases including the new case,
  and eight composition cases. The two focused cases also pass separately and
  are not added again to that count. No previous expected value or allowance
  changes. Commands are in `final-unit-qualification/`, `final-focused-green/`,
  `final-components-existing/`, and `final-components-connected/` receipts.
- The real guard-builder capacity-slope assertion fails on the frozen compiler
  (two backings), then passes on the candidate (one). Original/decoded real-size
  requests change 294→293 and bytes 4,922,563→2,480,507. Existing recurrence,
  native-loop selection and zero per-append allocation slopes remain checked.
- The future-seed control passes on both compilers and retains both old/new
  arrays. Original/decoded success→assertion rejection→division rejection→retry
  checks resident agreement, error precedence, unchanged publication/aliases,
  unchanged budget/nodes on failure and final zero cleanup.
- All 12 protected scalar controls retain the unchanged 1.05 ceiling, maximum
  ratio 1.0273764935, unchanged requests/bytes and zero allocation slopes.
  Eight managed-frame controls retain requests/bytes/work and zero slopes.
- Ten architecture/catalog/factory/warning/unsafe/format checks pass, including
  `cargo +nightly-2026-03-03 fmt --all -- --check` and `git diff --check`.
- Actual Cargo checks pass for `native-artifact`,
  `native-artifact,sequence_buffers`, `native-artifact,pure_text` and
  `native-artifact,map_ops`, each with `--no-default-features --locked --offline`.
  Two initial errors are **inherited**, reproduced on clean parent ba198160d:
  buffer-only queried a text-owned getter; bare native queried unavailable typed
  primitive vocabulary. Separate repair dc77024f2 preserves feature ownership,
  full-profile predicate order and conservative optional-proof refusal. See
  `FEATURE-REPAIR.md`; these are compilation checks, not executed-test counts.
- Diagnostic setup failures (missing established Cargo environment/explicit
  arena lifetime, initially unpopulated sparse checkout and an attempted
  unconditional getter exposure) were corrected or reverted and their failure
  receipts retained. No unresolved introduced correctness failure is known.
  The small elapsed regression and large preparation-memory attribution remain
  explicit performance limitations.

Primary timing is four prepared pairs × 16 alternating blocks × 31 samples per
variant: 3968 primary calls, 3984 including warm-up/final calls, and 20,000
reference observations. No build/test/profile overlaps primary execution. CPU
uses 32 steady calls per executable; memory eight steady Mech/32 reference;
linked diagnostics use 32 CPU/eight memory calls; profile uses 695 diagnostic
calls, all excluded from primary. Each observation cohort is committed at once.

The unchanged reduced-catalog knucleotide case is excluded. Full Core/Engine/
all-feature suites, a fresh linked assertion/recovery suite, planning-quarantine
and the 132-case CI inventory suite are not newly claimed; their earlier
receipts remain retained. Hosted CI is cancelled on publication by user
direction. Cancelled CI is not passing coverage. PR #973 remains draft until
green CI and clean review; a new review is requested on publication.

`reconstruct.py` verifies **37 archives / 477 file occurrences / 377 unique
paths** and independently regenerates byte-identical `scorecard.json` and
`scorecard.csv`. It performs no new benchmark or compiler run. Run:
`python3 reconstruct.py /private/tmp/new-empty-round38-verification`.

The next bounded optimization should reduce demonstrated ordinary loop/frame
transport and its repeated accounting/dispatch, preserving the current owner and
ledger. Use the actual path-parser child-call boundary and its retained aliases
as the witness; establish a minimal real-operation regression and acceptance
criteria before expanding selection. Do not substitute more source rewriting,
larger allowances or full-scale timing for this evidence.
