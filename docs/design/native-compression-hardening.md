# Native compression hardening work log

Baseline: `48911f96183db8ac804503a0c894b054d629382d`, fetched and verified
against PR #973 before implementation. Parent: #939 at
`eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. The retained stack remains
#934 → #935 → #939 → #973; this work neither merges nor replaces those PRs.

The authoritative architecture and cleanup contracts are
[native-compiler-architecture.md](native-compiler-architecture.md) and
[native-post-architecture-cleanup.md](native-post-architecture-cleanup.md).
Source and decoded artifacts retain one canonical pipeline, private engine
proofs, shared preparation and JIT/Object emission, independent image checking,
and the existing runtime state and selected-call boundaries.

## Baseline evidence and limits

The compressor is integrated and runs as one compiled computation. Its 25
components and six deliberate Huffman workspace initialization sites are
preserved. Source SHA-256 is
`345324ce551aaa2485dcd0fe4262da96e660d6b31f5f772bff89b034e9b1997f`
(160992 bytes); the retained revision-14 artifact is 4140067 bytes, SHA-256
`a2daaaf746d6bcc8668c5439f84a782ecea2aff376546566c01d4fc0f8c6f585`.
The pinned reference implementation is `671077457ff7ef3a3d44caf715a3fc595addf379`;
its compiler is `d99c7b838fb05bb1047eb0c18d4d06d0f86f345d`.

Historical complete-stream, block-transition, and window-transition results
are recorded in the [complete qualification](../../benchmarks/mech-deflate/results/2026-10-27-complete-compression/requirement.md)
and [boundary qualification](../../benchmarks/mech-deflate/results/2026-10-27-boundary-compression/requirement.md).
The completed 16384-byte and 33027-byte runs used saved-decoded artifacts and
NoLimits. Their original/decoded manual boundary test did not complete. The
full-corpus attempt was cancelled without a result. Multiple slides, full-corpus
agreement, and deployed complete-compressor execution are not established by
those receipts. Historical execution does not qualify the new compiler head.

The baseline head passes the locked offline canonical-native-program test build
on nightly-2026-03-03 (480.190 seconds), routing/catalog/layout/CI-inventory and
format checks, and the strict unsafe-boundary audit. The latest head review
has no unresolved findings. CI is cancelled by user instruction; neither green
CI nor merge readiness is claimed.

Initial profiling preserves the prior optimized executable at
`c1d20645d7b58f6b21847f51bfbf37225b6e6679` (binary SHA-256
`6c23c247bb0c661e3627a7ca833ca6f724139b1b4e3d13d1008239f0b1393346`).
An eight-byte complete compression passes exact bytes, independent inflation,
alias checks and cleanup in a sampled 2.802 seconds, with 16658509 allocation
requests and 1760520789 cumulative requested bytes. These are profiling
observations, not controlled new timing claims or peak memory. A separate
generic eight-store probe on the real U32 cache requests about 49 MB and takes
about 0.59 seconds; its runtime/codegen source is identical to that executable.
Full profiling provenance will accompany the implementation evidence.

## Bounded implementation sequence

1. Preserve packed collection capacity during finalization, reconcile retained
   capacity and copy accounting, and test lengths 3/5 for U8/U16/U32/U64,
   uniform construction, exact denials, publication, cleanup and retry.
2. Scope and name the definition-table allowance accurately; preserve exact
   stage diagnostics and prove nested exact/one-short admission.
3. Share dimension relations, the read-before-redefinition graph query, and
   prepared per-site call routing. Each consumer retains its eligibility,
   ordering, provenance, and runtime ownership checks.
4. Measure mixed buffer/scalar frames before changing their representation;
   use typed scalar lanes and prepared immutable recipes while validating
   current shapes and ownership on every reached invocation.
5. Share the compressor assembly entry and thread caller-supplied finite
   compilation limits through the public build path. Keep ordinary defaults
   and runtime launch authority distinct. Qualify source, decoded, JIT, Object,
   linked/image-restored and runtime-only boundaries at the final revision.
6. Publish a current component/owner map, public commands and finite policy,
   fresh results, historical references, and remaining gates.

Do not replace generic owners, weaken scalar execution or exact finite denials,
change algorithm or fixture values, delete apparently unused initialization,
or restart cancelled long campaigns. New experiments remain focused until
measured execution makes larger runs practical.

## Frame comparison declared before primary timing

The focused frame study will carry native unsigned buffers plus scalar counters
through repeated reached visits. Include internally allocated exclusive buffers
and externally aliased inputs, changing dimensions, assertion/arithmetic errors,
previous-output preservation, and valid retry. Measure allocation requests,
cumulative requested bytes, and evaluation-only time; preparation, input setup,
validation, logging and output destruction stay outside the timing boundary.
Retain all observations and the exact baseline/candidate executables.

Use three balanced baseline/candidate rounds with ten retained observations per
case per executable and one disclosed warmup per round. An initial diagnostic
may select a practical fixed visit count; freeze it before primary observations.
Require at least a 30% reduction in targeted mixed-frame allocation requests and
requested bytes, with a median elapsed-time improvement. Scalar-only controls
must retain direct native arithmetic and zero per-visit allocation; their paired
median time may not regress by more than 5%. A failed gate remains a failed gate;
do not rerun an unchanged cohort in search of acceptance. Further explanations
or independent exploratory samples must be labelled separately.

All regression checks retain original/decoded artifacts, aliases, current
dynamic extents, lexical error order, publication atomicity, finite admission,
cleanup and recovery. Warnings remain errors. An ignored or cancelled test is
never a passing qualification.

## Packed completion checkpoint

The new nonuniform regression reproduced the reviewed issue: a three-element
U8 result shrank its admitted capacity from four to three. Unsigned completion
now transfers its Vec through `UnsignedMatrixBuffer`, preserving the allocation
and capacity. Both growth preflight and completion price the capacity that the
result retains. Uniform fill retains its separate exact-capacity path. Draft
collection behavior and the existing completion coexistence envelope remain.

Focused `canonical-native-program` checks on nightly-2026-03-03 pass: 17
collection tests, five uniform-construction tests, and one owner test covering
pointer identity, capacity, and exact/one-short managed admission at lengths
three and five for all four unsigned widths. Regressions also cover exact copy
and output caps, unchanged prior publication, failure after completed collection
construction, input aliases, retry, and zero managed bytes/nodes after cleanup.
Commands, complete output, and unsuccessful development attempts are preserved
in [the hardening receipts](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/).

An initial test used narrow addition unnecessarily. Its decoded artifact lacks
a lowering witness (`MissingSemanticWitness`); the collection-runtime changes
cannot affect that earlier compiler stage. The nonuniform ownership test uses
varying input words directly so it tests collection finalization independently.
The narrow-addition reproducer is retained with the receipts as an inherited,
separate compiler limitation. Test-authoring errors (wrong accessor, omitted
source-read copy debit, and assuming an unrequested binding was published)
were corrected; their failed attempts remain visible. `python` is unavailable
in this environment; receipt commands use `python3`.

## Definition-table allowance checkpoint

The reviewed implementation is a conservative **per-table estimate**, not a
total analysis-memory budget. The public compilation field is now
`max_definition_table_bytes`; engine lowering exposes
`lower_native_program_artifact_with_definition_table_limit`, and rejection is
`DefinitionTableLimit { stage, required, limit }`. The 64 MiB default, estimate,
check order, semantic plan image, and diagnostic unrestricted mode are unchanged.
The estimate covers one dense table's row owners and packed definitions; graph
inventories, worklists, cloned rows, and simultaneously live nested tables are
explicitly outside its scope. Admission precedes definition-set and scope-table
construction. This work does not claim to impose a phase-wide memory bound.

The exact and one-byte-short nested-scope test passes for original/decoded
artifacts through engine lowering and JIT compilation. Both proof-storage
tests pass, including 321 composed recursive calls and failure/retry; the
explicit unrestricted-mode regression also passes. Receipts are in the same
hardening evidence directory. Existing engine proof tests retain the malformed
oversized-table preflight and arithmetic-overflow cases with the precise name.

## Shared liveness checkpoint

`NativeProgramFacts` now owns the common graph walk for an old-value read before
a redefining write. Actions read before writes; a write kills the previous
owner before the terminator. The frontier terminator, branches, and backedges
remain visible. Invalid graph metadata rejects the optimization.

Frame eligibility still maps exact loop seed/latch/return reads to their
existing positions and declines opaque uses. Selected unsigned-buffer calls
still reject collection captures, loop roles, publication and effects at their
independent ownership boundaries. Neither traversal sharing nor a static proof
replaces runtime account/exclusivity checks. The two query unit tests and all
seven existing private-frame regressions pass, including independent image
rederivation, aliases, current Booleans, nested calls, finite denial and retry.
The 14 selected-buffer capacity regressions also pass after the shared walk.

## Shared dimension relations checkpoint

Schema-owned dimension relations now provide an explicit exact/free-Turn-axis
policy and a bijective parameter map. Origin freshening is a separate policy;
computed-axis generalization is a separate operation with an expression map.
The source annotation, function-argument and selected-result owners call these
mechanics while retaining their own selection boundaries. Fresh embedding still
assigns per-use IDs. Unsigned matrix eligibility, fixed and repeated axes,
element kinds, and the constrained/persistent conversion path remain intact.

Unchanged I64/F64 scalar siblings no longer prevent an otherwise harmless
parameter renaming. Three core relation tests, 53 selected frontend tests
(including the existing 12 relation-policy tests), and a new original/decoded
resident/JIT mixed-record test pass. The latter changes both buffer lengths and
retains I64 minimum, signed zero, NaN bits, and input aliases. No compressor
component, allocation site, source fixture or expected output changed.

## Prepared selected-call checkpoint

Each selected library site now has one immutable prepared strategy: private
frame, unsigned buffer, scalar record field, unsigned length/element, or the
ordinary canonical path. The original frame-first priority remains. Live loan,
owner, extent, account and exclusivity checks still run inside the selected
consumer at their original positions. A shared private-owner inventory replaces
five identical exclusions without extending any consumer's eligibility.

The strategy is derived again from independently validated plans and actual
factories during image restoration; it is absent from serialized authority.
One new test covers all six routes for original/decoded artifacts and restored
images. Existing runtime regressions pass: seven frame, five word-read, three
record-word, three unsigned-length, 14 capacity, and two caller-inventory tests.
These functional runs were concurrent; their incidental timings are not a
performance comparison. No per-operation allocation or interpreted dispatch
was introduced, and direct scalar emission is unchanged.

The published checkpoint is `39442acb64ed47500ec26d5c4037cc37d0015ebe`.
Its requested review completed with no findings or unresolved threads. CI run
40635783224 was cancelled by request; it is not passing CI evidence.

## Frame measurement checkpoint

The probe and one-shot comparison driver are in `hosts/gpu/examples/native_frame_transport.rs`
and `benchmarks/native-frame-transport/`. Before changing frame storage, an
allocation diagnostic linked the current debug runtime libraries with an
optimized wrapper. Its source, commands, first type-inference error/fix and all
warmup/retained observations are preserved. The task compiler was paused while
measuring and resumed afterwards. This is not the optimized acceptance cohort.

Exclusive-buffer visits add exactly 67 requests and 3966 requested bytes each;
aliased-buffer visits add 70 requests and 4234 bytes each. At 64 visits, warmed
counts are 4578/289771 bytes and 4750/305254 bytes respectively; measured debug
times are about 3.4 ms. The scalar-only loop uses 164 requests/24048 bytes at
both zero and 64 visits, with a generated scalar loop and no private frame.

The primary comparison is now frozen at 64 visits. Its committed protocol adds
original/decoded forms, direct Bool/U64/I64/F64 controls, and exact scalar
allocation-slope checks. It retains every sample, balances execution order,
requires the same artifact hash across compilers, and refuses concurrent local
compilation. No primary observations have started. The optimized baseline build
must finish before its executable is saved and frame implementation changes
are wired into the maintained pipeline.

The maintained compressor assembly is now shared under the fixture owner.
Its 25 components plus entry are byte-identical to the saved source: 160992
bytes, SHA-256 `345324ce551aaa2485dcd0fe4262da96e660d6b31f5f772bff89b034e9b1997f`.
The pin has an ordinary Rust regression as well as the recorded byte comparison.
Public finite-policy execution remains pending.

The earlier [native profiles](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/profiling/REPORT.md)
are retained separately from fresh compiler qualification and the declared
frame comparison. They establish generic indexed-copy and budget-traversal
costs without assigning all complete-compressor cost to frames.

The optimized baseline executable is now saved independently of Cargo output:
SHA-256 `9528eafd738edf78e46d78f926b67dd2ef62e06894039415bf2f0d4b1ce33cda`.
Its production sources match `39442acb64ed47500ec26d5c4037cc37d0015ebe`; the
probe source and complete production-source manifest accompany its receipts.
The full dependency rebuild took about 50 minutes, including a deliberate
diagnostic pause. Probe-only hash formatting failed with sha2 0.11's output
type, then passed after portable byte formatting (6.212-second rebuild).
This was build time, not Mech execution time. All failed attempts remain.

Optimized diagnostic runs of all seven cases in original/decoded form pass
verification and cleanup. At 64 visits, mixed frames retain the same request
and byte counts above and take about 0.31 ms; the scalar loop takes about 9 µs.
Direct Bool/U64/I64/F64 controls take about 1.06–1.09 ms per 1000 invocations
(36 requests and 4804 requested bytes per invocation, including their existing
public entry setup). These are diagnostic observations, not the balanced
acceptance comparison. The immutable baseline executable is preserved before
wiring any frame-storage change.
