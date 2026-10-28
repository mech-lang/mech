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

## Current small timing

The [latest eight-byte timing receipts](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-03/REPORT.md)
use the real tree/cache and unchanged 25 components. With **Roc = 1×**, the
matched-allocator, uninstrumented comparison improves from **2753×**
(0.757143375 s) to **1904×** (0.523613000 s), against a fresh 0.000275 s reference.
This combines an immutable schema-size summary with a selected-operation build
profile; it does not isolate their individual effects. The summary's real
16-byte metadata increase remains charged, raising reported work from
939091080723 to 941491317987. Copies, aliases and finite-denial behavior remain
qualified by the recorded tests. There are three retained observations per
Mech executable; this is a small diagnostic, not a full-corpus result.
Preparation and verification are outside the compression timer.

The [previous round](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-02/REPORT.md)
was 2494× using a 0.000305 s reference; its denominator differs from this fresh
comparison. The new profile no longer shows the repeated no-callback schema
walk. Ordered precharge walks, ownership accounting, allocation, copying and
resource checks remain. No next optimization is presumed.

The [focused build](../../benchmarks/mech-deflate/native-profile/README.md)
selects only the required machine operations through the ordinary catalog.
It avoids the full-library dev-dependencies pulled in by GPU examples. The
first selected release build took 990.50 seconds; the prior broad build was
stopped unfinished at 1376.43 seconds. These are build receipts, not a clean
build comparison. Exact source/artifact identity, 177 Rust tests and protected
scalar controls pass. Full GPU component/linked and generated runtime-only
application qualification has not been refreshed for this runtime change.

A separate matching-System-allocator, matching-instrumentation compiler control
improves from **8659×** to **6599×**. That comparison spans the intervening
hardening commits; it does not isolate the typed-frame change. The focused
balanced frame comparison has its own scalar controls and acceptance gates.
Allocator experiments are permitted; a compressor source change needs a
specific performance justification and separately recorded fidelity review.
