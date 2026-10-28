# Complete compression: current compiler path

The level-12 compressor is implemented as Mech source and executes as one
compiled computation. It is not a native compression library call. The
maintained source owner is
[`deflate_assembly.rs`](../../hosts/gpu/tests/fixtures/native/deflate_assembly.rs),
shared by correctness tests and the public application example. Its 25 source
components and entry remain 160992 bytes with SHA-256
`345324ce551aaa2485dcd0fe4262da96e660d6b31f5f772bff89b034e9b1997f`.
The six deliberate Huffman initialization sites remain.

## Components and required capabilities

| Source responsibility | Permanent entry points | General compiler capabilities |
| --- | --- | --- |
| Match finder and binary tree | node indexing/rebasing, tree search and skip, match extension | U16/U32 buffers, exact conversions, wrapping address arithmetic, checked reads, lexical checks |
| Match and block collection | `optimal-collect-position`, prefix collection, observations, `optimal-end-block-check`, block loop/carry | Recursive carriers, current scalar arguments, search/skip state and cache headers |
| Path selection | `optimal-find-min-cost-path`, `optimal-optimize-loop`, `optimal-choose-all-literals` | Checked integer costs, U64 reserved path nodes, append and clear with retained capacity |
| Models and costs | `huff-make-code`, `optimal-tally-and-build-codes`, default/true costs, `optimal-select-block`, `optimal-plan-block` | Typed comprehensions, indexed updates, records, exact narrowing, comparisons and histograms |
| Code words and payload | static/full length codes, path items, sequence writer | Exact bit operations, original reversal table, U8/U16/U32/U64 typed buffers |
| Block emission | cost selection, dynamic headers, stored blocks, flush | Reserved output, little-endian append, checked shifts, native control flow |
| Complete controller | `optimal-compress`, `deflate-compress-into` | Consecutive block decisions, carryover, window slides and reusable output |

The real tree has 262144 U16 entries; the cache has 1500513 U32 words. Search
depth remains 300 and nice length 258. Near-EOF limits, tables, optimization
passes, buffer sizes, allocation sites and timing boundaries are unchanged.
Search and skip remain separate; skip has no cache argument or recording path.

## Owners and execution boundary

Source semantics produce the canonical `ProgramArtifact`; the codec validates
decoded artifacts at the current revision and rejects older revisions. Engine
native lowering derives private selected-binder proofs. Shared preparation
feeds both JIT and Object emission. Generated applications restore and check
their build-owned descriptor and linked identity independently, then use the
same native runtime state. There is no compressor-specific executor, source
name recognition, buffer-size recognition or compression intrinsic.

| Mechanical reasoning/storage | Owner | Consumer responsibility retained |
| --- | --- | --- |
| Dimension relations and mappings | `mech-core::schema::dimension_relations` | Source renaming, freshening and generalization have distinct policies |
| Old-value read before redefinition | Engine `NativeProgramFacts` | Frames and consuming buffers retain separate eligibility and runtime ownership checks |
| Per-site selected-call route | Shared native preparation and independent image restoration | Current loans, account identity, exclusivity and error ordering |
| Mixed argument frames | Engine frame facts; host runtime word/owner arenas | Exact scalar kinds, current-visit validity, managed alias retention and retirement |
| Flat frame shape recipe | Prepared resident library call | Fresh input extents and fixed/repeated-axis checks at each call; other relations use the core constructor |
| Unsigned buffer backing | Core `UnsignedMatrixBuffer` and snapshot owner | Capacity, copy-on-write aliases and failure recovery |
| Runtime resource accounting | Existing resident budget and native work ledger | Physical admission, logical output bounds, finite denial, cleanup and atomic publication |

Native self recursion uses supported structured loop carriers rather than a
general native call stack. Direct and piped checked self-calls use fresh
arguments; nonrecursive immutable captures retain their existing optimization.
Do not infer support for arbitrary non-tail or mutual recursion from the
qualified compressor's structured self-calls. Primitive Bool/U64/I64/F64 calls
retain direct native scalar execution. Narrow native support is operation and
binder specific: checked U8 addition, U32 addition/subtraction, unsigned
comparisons/conversions and the documented word operations do not imply every
narrow arithmetic operator is available. A decoded U8-add witness gap has a
separate minimal reproducer in the hardening receipts.

## Qualification boundaries

The [hardening work log](native-compression-hardening.md) and its linked receipts
separate fresh exact-revision results from historical execution. Existing
small-stream qualification includes stored, fixed and dynamic choices,
nonempty prefixes and overlap. Complete stream tests compare exact pinned raw
bytes, independently inflate them, retain input aliases and require zero
managed bytes/nodes after release.

Historical saved-decoded runs completed 16384-byte block-transition and
33027-byte window-transition inputs. Those runs are slow baselines; they do
not establish the final compiler's original/decoded/deployed boundary coverage.
Repeated slides and full-corpus agreement remain unqualified. The cancelled
full-corpus run supplies no timing result. Ignored long tests are not passes.
No cancelled campaign is restarted by this work.

The [native profile](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/profiling/REPORT.md)
records complete-program indexed-copy and budget-traversal costs. The focused
[frame protocol](../../benchmarks/native-frame-transport/protocol.json) has
separate original/decoded mixed-buffer and scalar controls. A frame improvement
does not measure its contribution to whole-compressor time.

The [public application instructions](../../benchmarks/mech-deflate/public-application/README.md)
build the complete program from source or decoded bytecode using explicit
finite compilation limits. Both forms produced byte-identical Objects,
descriptors and identities on the normal process stack. Generated runtime-only
applications each executed the eight-literal input with exact raw bytes,
independent inflation and complete managed cleanup. Their actual Cargo artifact
inventories contain no parser, Cranelift, source compiler or JIT dependency.
The other eight deployed small cases were not rerun. This does not qualify the
final compiler's block/window transitions or full corpus.

Compilation policy bounds code generation and one definition table; it is not
a total compiler-memory bound and confers no runtime launch authority. Existing
generic defaults remain unchanged. Compilation and launch policies are strict,
separate JSON inputs; the public CLI also accepts `--native-compile-limits`.

## Separate source experiment and next compiler slice

The [round 09 packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-09/REPORT.md)
uses exactly the same compiler executable with an optional one-dimensional
cost-table source variant. Its fresh paired eight-byte result is **448.75× →
421.53× Roc**, 0.130136250 s to 0.122244917 s against 0.000290 s: **6.06% less
elapsed time**. This is a source improvement, not a compiler improvement. The
pinned 25 components remain the default. Original/decoded vector tests and six
focused Rust tests pass; bytecode grows 8.32% and nonstandard vector fallbacks
perform an additional row copy. General multidimensional use is not qualified.
No larger input, new scalar gate, or deployed application was run for this
same-executable experiment. All failed attempts and actual coverage are retained.

The read-only packed unsigned leaf slice is implemented at compiler
`06885e0d9a1466ba477d642ee9fd9fd8510b06f2` through the existing canonical
Program, scalar emitter and closed-tail account. All-edge provenance retains
invariant owners; per-invocation views and native scalar state support an ordinary
suspended parent. Native loads preserve current matrix geometry, checked errors
and short-input guards. Independent restoration rederives proofs and prices.
The full Object selects 45 parented regions with 72 typed load sites and zero
calls on successful hot backedges. Private writes, growth and arbitrary nesting
remain separate follow-on work.

## Current small timing

The [round 11 packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-11/REPORT.md)
measures **408.21× → 346.11× Roc**, with **Roc = 1×**: 0.117155000 s to
0.099332833 s against a fresh 0.000287 s reference. This is **15.21% less elapsed
time**, using the unchanged eight-byte source, artifact, allocator and timing
boundary. All retained calls produce exact raw bytes and independent inflation,
preserve input aliases and finish with zero managed bytes/nodes. Logical work
changes 875610515844 → 789930572624 under explicit native work policy 91/IR116.

The caller retains capacity 13 and clears/rebinds the previous result outside
timing. However, `same_backing` is **false on every before/after call**: compression
copies the backing across the immutable public argument boundary. This overhead
is timed; physical output-backing reuse is not established.

There are **128 scoped Rust passes**, twelve passing static contracts and four
late changed-file checks. Original/decoded weighted reductions have fixed
245 requests/40715 bytes/six ordinary visits over 0/1/8/64/256 elements. A real
U16/U32-buffer diagnostic completes eight positions in about 0.83 ms; its exported
tuple and System/probe allocator are separate from compressor timing. Ten observed
finite-fuel exits restore the current scalar carrier, release the loan, preserve
publication on rejection and retry. A real runtime-only linked executable passes
valid → invalid → valid, preserving assertion-before-division and cleanup.

Fresh direct scalar timing/allocation gates pass (slowest ratio 1.020 against
1.05), with exact request/byte counts and zero scalar-loop allocation slope. The
historical round-10 decoded-U64 failure remains unresolved; its failed cohort is
retained. Larger inputs and broader deployed coverage are not refreshed. The new
profile still places ordinary frame admission and loop visits prominently; the
next measured compiler slice is private indexed-write recurrences.

The preceding [key-reuse packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-10/REPORT.md)
measures **448.70× → 404.86× Roc**, 0.129673750 s to 0.117005125 s against a
fresh 0.000289 s reference: **9.77% less elapsed time**. Managed projections reuse
the existing prepared key after exact slot/schema/definition alignment checks in
preparation and restoration. No new cache or mutable-geometry reuse is added.
All 41 scoped Rust tests and twelve static checks pass; logical work is unchanged.
The scalar allocation gates pass, but one scalar timing gate is unresolved:
decoded U64 measures 1.0526× against the unchanged 1.05 limit. It is retained
without rerunning for acceptance. This patch has no blanket scalar-latency pass.
The round-10 receipt predates native numeric selection; its coverage remains
separate from the completed round-11 slice above.

The [preceding mixed-state timing receipts](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-08/REPORT.md)
use the real tree/cache and unchanged 25 components. With **Roc = 1×**, mixed
scalar state measures **490× → 455×**, 0.139637125 s to 0.129644084 s against a
fresh 0.000285 s reference, or 7.16% less elapsed time. The existing private word
layout now carries exact Bool/U8/U16/U32/U64/I64/F64 tuple/enum leaves. The actual
program selects 91 private values and eight mixed optimal-path Tail carriers.
This is a modest gain, short of the larger reduction still needed.

The same qualification exposed an inherited closed-Tail failure exit that
returned before unwinding its fuel loan and masked an assertion error. The
retained old executable reproduces it; the corrected shared exit returns the
original error, preserves publication, cleans up and recovers. All 109 scoped
Rust tests, static checks and protected scalar gates pass. Complete logical work
is 875610515844, distinct from executed instructions. Unsupported nested state
and escaping results keep canonical storage; source and artifact stay unchanged.

The [preceding metadata round](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-07/REPORT.md)
measures **574× → 495×**, or 13.67% less elapsed time. Successful scans prepay the
same debit once; tight bounds and worker loans keep ordered incremental charging.
The separate table experiment below retains this pinned-source compiler baseline.

The [preceding consuming-store change](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-06/REPORT.md)
measures **1626× → 574×**, 0.465164708 s to 0.164188708 s against its fresh
0.000286 s reference: **2.833× faster** than that paired Mech baseline. Native
scalar operands, same-account ownership transfer and one-word changed detection
avoid the old binding, payload-copy and full-equality path. External aliases keep
their first necessary immutable copy; private recursive chains reuse backing.
The actual six-position collector falls from 25682 to 8362 requests and 79457091
to 35045914 requested bytes, with unchanged golden state and finite policy.
Some copies remain. The next substantial target is repeated canonical object
construction and runtime transitions in the real numeric loops. Parity is unmet.

The [preceding length change](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-05/REPORT.md)
measures **1727× → 1632×** against a different reference cohort. Numeric length
counting now uses saturated multiplication; bytes/hashes and ordered work retain
their paths. Its scoped core snapshot/memory tests remain separately recorded.

The [preceding matrix-read repair](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/matrix-read-order/REPORT.md)
measures **1695× → 1693×** against a different reference cohort. Its 0.16%
difference is not a speedup claim. It makes all unsigned widths agree with
resident column-major selection over canonical storage.

The [preceding performance change](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-04/REPORT.md)
removes a redundant full call-template clone. That isolated comparison improves
**1796× → 1702×**, 0.517336500 s to 0.490266166 s, with a 0.000288 s reference
and the same reduced catalog on both sides: 5.23% less elapsed time. Eight real
buffer stores use 8833 rather than 9913 allocation requests. Reported logical
work remains 941491317987. The prior 1904× anchor used a different denominator.
There are three retained observations per subject. Preparation and verification
remain outside the timer; only the eight-byte complete case was timed.

The [focused build](../../benchmarks/mech-deflate/native-profile/README.md)
uses the existing catalog feature gates. The length change's rebuild takes
1007.38 seconds and rebuilds the machine libraries because core changed;
the store change takes 454.37 seconds and reuses them. All 84 scoped native,
component and collector tests pass at the store change. The length change's
248 tests and no_std check retain their own revision, as do earlier selections.
Protected scalar gates, exact source/artifact agreement, real-buffer witnesses,
and architecture checks pass. Linked subprocess and generated runtime-only
application qualification has not been refreshed for these changes. The PR
remains draft pending fresh review; CI is cancelled by request.

A separate matching-System-allocator, matching-instrumentation compiler control
improves from **8659×** to **6599×**. That comparison spans the intervening
hardening commits; it does not isolate the typed-frame change. The focused
balanced frame comparison has its own scalar controls and acceptance gates.
Allocator experiments are permitted; a compressor source change needs a
specific performance justification and separately recorded fidelity review.
