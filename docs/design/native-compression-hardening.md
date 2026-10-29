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

This section preserves the pre-change checkpoint; the completed cohort and its
outcome are recorded under “Typed frame result” below.

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

## Typed frame result

The targeted implementation is `2b4adb7289cb81ffd0c37456ce1f968c9ed3d43d`.
Its optimized executable is SHA-256
`3c34e2ff5a72f8981c7c23d57a04c13ca185cd63d73a14e91a72262c18cfb457`.
The exact probe source remains unchanged from the saved baseline. The
predeclared balanced comparison passed every gate on its first cohort. No
observation was discarded and no failed cohort was rerun. The complete
[observations and executable/source manifests](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/frame-comparison/)
are retained.

At 64 visits, original/decoded exclusive frames drop from 4578 requests and
289771 requested bytes to 1428 requests and 61192 bytes; paired median elapsed
ratios are 0.4654 and 0.4640. Aliased frames drop from 4750 requests and 305254
bytes to 1600 requests and 74010 bytes, with ratios 0.4682 and 0.4678. This is
66–69% fewer requests, 76–79% fewer requested bytes, and about 53–54% less time
for this focused workload. It is not a whole-compressor speedup estimate.

All four primitive scalar controls and the scalar loop retain exactly the same
allocation requests and requested bytes. Their timing ratios remain within the
predeclared 1.05 threshold. Zero and 64 scalar-loop visits both request 164
allocations/24048 bytes. Across three rounds the cohort retains 840 primary
observations, 84 explicit warmups and 16 scalar-slope diagnostic observations.
The measured runtime runs alone; compilation had completed and no old
compression job was active.

Frame facts now prepare exact schema/kind/lane maps. Scalars occupy native
word lanes with current-visit validity, while only managed fields retain Value
owners. Copy and pack validate all fields before replacing owners; retirement
invalidates words and releases managed references. Setup admission includes the
real word and owner arenas. Selected record projection stays in word lanes
until a real canonical consumer/publication needs a Value. Prepared frame
geometry caches only immutable normalization bounds and a conservative flat
shape recipe; each call revalidates current extents, fixed/repeated relations,
scalar widths and account membership. Unsupported relations retain the core
constructor/binder. Image restoration derives the recipe and lane map again.

Nine frame integration tests, two recipe tests and the existing linked
mixed-record regression pass. Coverage includes all seven scalar kinds, exact
narrow maxima, signed limits, signed zero/NaN bits, changing shapes, invalid
bits and successful retry. The first minimal engine-only unit command lacked
the maintained semantic-compiler test features and failed in existing test
imports; it also found a new test's missing Arc conversion, now corrected.
The supported combined engine/GPU feature profile passes without suppressing
warnings. Failed commands and their original output remain in the receipts.

## Public finite-policy and current-state checkpoint

`c24984b402a1228cccf934a478965143b539f611` adds explicit finite compile limits
to the public managed and hosted application APIs and public CLI. Existing
default entry points delegate with unchanged defaults. Strict compilation JSON
and strict runtime launch JSON have separate owners and cannot grant each
other's authority. The managed shell retains its numeric argument interface.
The shared complete source, normal-main example, finite policy and commands are
in [public-application](../../benchmarks/mech-deflate/public-application/README.md).

Both original and decoded source-to-Object generations succeed on the normal
process stack. Objects, descriptors and identities match byte for byte. Actual
runtime-only Cargo artifact inventories exclude the parser, compiler and JIT.
Both linked executables run the eight-literal input with exact bytes,
independent decompression and complete managed cleanup under finite launch
policy. This qualifies one deployed small case per form; it does not qualify
the other eight small cases or final-head block/window/full-corpus execution.
The [public-path report](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/public-application/REPORT.md)
retains build/run commands, hashes, source-generation times, successful
executions and failed attempts. No compiler stack fix was necessary for the
tested public entry points.

The [current-state document](native-compression-status.md) supplies the
component/capability/owner map, recursion boundary, corrected narrow-operation
scope and outstanding qualification gates. Historical receipts are preserved.

Small complete-compression timing is recorded in
[`af77b28409414b2d61e30ace29c98a37f152ff48`](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-01/REPORT.md).
With Roc = 1×, that matched-allocator, uninstrumented baseline is 5756×.
The corrected matching-System/instrumentation before/after control in
`3465e13cf3263a047b0c158eab7a9d3d15c99ebb` is 8659× → 6599×. Allocator and
instrumentation changes are separate from the compiler comparison. All 25
components remain byte-identical, and no long campaign was restarted.

## Packed assignment planning result

Protocol `e0b6b18c60ed6b244263227f97b6cbe0c2a9eeba` precedes implementation
`335de1c16743343fa6745d7ed2e638f6f1626fa9`; timing and profile were committed
as `0c2ed38dd037bd23aed8fc2f5d78a42b7d3f562b`. The
[round 02 report](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-02/REPORT.md)
retains exact executable/source/allocator hashes and all observations.

A proven single-destination, single-source packed unsigned assignment can
prepay its unchanged planning charge. The existing meter previews fit; tight
budgets retain the incremental traversal, exact first denial and spent prefix.
Copy admission/construction, alias ownership and publication remain unchanged.
No source component, artifact or compiler architecture boundary changes.

Fresh matched-allocator eight-byte medians improve **5496× → 2494× Roc**,
1.676382750 s → 0.760682250 s against a 0.000305 s reference. Reported work
remains 939091080723, output matches exactly, independent inflation succeeds,
and managed cleanup is complete. The real-buffer witness preserves allocation
counts/bytes; its faster stores still make physical copies. Only small inputs
were measured and no cancelled campaign was restarted.

The qualification follow-up records 58 distinct passing tests and all failed
attempts. The newly introduced matrix-valued source fixture rejected before
runtime; correcting it to the actual scalar-store operation passes for original
and decoded artifacts, finite denials, aliases and recovery. Engine units still
cover scalar and one-cell packed data. Four unsupported caller forms are saved
as separate reproducers without claiming clean-parent validation. A flat,
byte-identical engine test-file relocation fixes the module-layout guard; all
architecture/catalog/routing/inventory/warning/boundary/format checks pass.

The post-change profile no longer has the full destination planning walk as its
dominant visible stack. Schema-size and ownership accounting, call setup,
resource checks and managed transport remain visible costs. Follow-up symbol
resolution identified repeated schema allocation-bound traversal; the next
recorded round addresses that mechanism. The 1× target remains unmet and #973
remains draft.

## Schema summary and selected-operation build

Runtime commit `ef0e5d37a47c547a7c4a28f1bfae508f4d977cb8` derives a private
checked size summary for each immutable schema entry arena. Clones retain it;
new arenas derive it. The no-callback query uses it while the ordered precharge
visitor remains unchanged. Its real 16-byte inline cost is included in memory
and logical-work accounting; canonical identity and wire bytes do not change.

Build commit `ac96b275d23c1219bb3c8c8067f4e05d19a44b47` adds an ordinary
consumer using existing standard-library feature gates. Only five arithmetic
operations, six comparisons, and/or and inclusive ranges are selected. It
avoids GPU example dev-dependencies that select the full math library, while
retaining the compiler/runtime feature configuration and scalar controls.
The first selected build completes in 990.50 seconds; the full build was
stopped unfinished at 1376.43 seconds. Neither is a clean-build comparison.

The [round 03 report](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-03/REPORT.md)
records **2753× → 1904× Roc**, 0.757143375 s → 0.523613000 s on the unchanged
eight-byte input and matched allocator. This comparison includes both changes;
it does not isolate a schema-summary effect. Exact source/artifact identity,
177 distinct Rust tests, protected scalar timing/allocation gates, aliases and
cleanup pass. Full GPU component/linked and generated runtime-only application
qualification has not been rerun for this change. Only small timing ran; the
full corpus remains stopped.

## Owned turn templates and unsigned read ordering

`1e0f5ea48aa4fc0fdff456cf8b113f4a065ef8cb` removes the second whole call-plan
copy from current turn derivation. The already-owned local template is reused;
its limits are restored before returning success or error. The core planner,
current arguments, region/publication checks and logical charges are unchanged.
The [round 04 packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-04/REPORT.md)
records 120 passing scoped Rust tests and **1796× → 1702× Roc**, or 5.23% less
elapsed time on the unchanged eight-byte case. Real-buffer stores eliminate
1080 allocation requests and 124608 requested bytes per eight stores.

Review then exposed an existing multi-row unsigned read defect. Test commit
`57abc1daf9c2c1223e6aed15f21f22094ec195a2` corrects the U64 expectation against
the resident oracle and adds all-width current-shape/recovery coverage. Two
tests fail before production changes. `d19532830206dc51df584c1f39b5d42724305c34`
reuses the numeric dimension owner and translates the selected ordinal into
canonical storage order. All 80 final scoped Rust tests pass, including the
69-test component cohort. The [repair packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/matrix-read-order/REPORT.md)
records **1695× → 1693× Roc** (0.16% elapsed difference, not a speedup claim).
Both cohorts retain exact bytes, work, aliases, capacity and cleanup; protected
scalar controls pass. Each iteration reuses all machine libraries and rebuilds
in about 7.5 minutes. Broader linked/application and full-input qualification
remain unrefreshed; #973 remains draft.

## Packed payload length and the next ownership target

Protocol `7ffb113e5239e24fd34822d65161443df042919a` precedes implementation
`ced2c71c36d002a044b40f1f5cfb8ae293e92572`. The existing private sink protocol
lets the length sink count fixed-width packed data from length and encoded
width. Other sinks keep the original encoder, and variable/composite data keep
their traversal. Saturation, canonical bytes and ordered work/admission remain.

The [round 05 packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-05/REPORT.md)
records **1727× → 1632× Roc**, 0.492213292 s to 0.464984292 s against a fresh
0.000285 s reference, or 5.53% less elapsed time. It retains the unchanged source,
artifact, allocator, eight-byte boundary and logical work. Binary inspection
confirms numeric length calculation no longer loops over elements. All 248
scoped Rust tests, no_std, static checks and protected scalar gates pass; the
initial new-fixture borrow error is retained. The core change rebuilds the
machine libraries and takes 1007.38 seconds. Broader linked/application and
full-input qualification remain unrefreshed.

The small gain does not close the performance gap. Scalar indexed replacement
still copies the entire array and compares snapshots. The main next change is
a consuming selected update with native scalar arguments and a direct changed
bit, followed through the real recursive tree/cache carriers. Require private
stores to avoid repeated payload copies, external aliases to remain immutable,
and success/failure/retry to preserve publication and cleanup. Keep application
rewrites as separate source revisions and measurements. Do not substitute
further small metadata patches for this ownership work.

## Consuming scalar indexed replacement

Protocol/regression `dfaa03115656ba0e2ed4bf84db024dffa2cd8e5a` and its corrected
fixture `151d5a0b5507b57b0f7cad0aee8784788a9b9bfe` establish the old recursive
eight-payload allocation slope before production changes. Runtime implementation
`5929c0bff0a7e1b4b14aec2f684b0b8c634975ad` adds the narrow prepared unsigned
store route; `5bd4cb352ba01e0dfb34982dc98d035b3473fffb` checks exact and every
one-short resource dimension. The constructor-private original factory, shared
preparation/image derivation, existing owner transfer, scalar transport, index
mapping and publication boundary remain. No new opcode or source rewrite is used.

The [round 06 packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-06/REPORT.md)
records **1626× → 574× Roc**, 0.465164708 s to 0.164188708 s against a fresh
0.000286 s reference: **2.833× faster** than the paired Mech baseline. All
bytes, inflation, aliases, capacity and cleanup checks pass. Work changes to
888390537256 logical units; the report discloses both increasing and decreasing
component charges rather than confusing them with instructions or time.

All 48 recursive real-buffer observations now have exactly one payload-size
allocation slope, including retained external aliases and internal initialization.
The existing frame retirement suffices for that witness; no capture/loop exception
was added. Actual search/skip collection also benefits: six-position traffic falls
from 79457091 to 35045914 bytes and 25682 to 8362 requests with unchanged golden
state and finite policy. It still has additional copies. The unchanged straight-
line eight-store witness falls from 8833 to 274 allocation requests.

All 84 scoped Rust tests, selected checking/build, static checks and protected
scalar gates pass. Machine libraries are reused and release rebuild takes
454.37 seconds. The corrected Rust fixture macro failure and expected red
allocation failure are retained separately. Broader linked/generated application,
route unit-suite and full-input coverage remain explicit gaps; CI is cancelled
and #973 remains draft. The new profile makes repeated budget/accounting and
runtime visits the next measured target. Parity remains unmet.

## Successful native metadata precharge

Protocol `0a575edfce86deeedcd11e9b5ee5e2ec5b93c1a8` reproduces the missing native
scope preview; `51eeb3f5fc5390b7b1b5213870b60ed3c577d182` enables pure preview
and batches the existing immutable schema charge when the entire debit fits.
Tight/overflow/foreign scopes and worker loans retain the incremental path.
Independent metadata oracles verify identical errors, spent prefixes and cleanup.
No account authority or resource allowance changes. Separate commit
`03a6f6518d15e480e30270c1acbe9fa5cffb9e89` fixes an unused mapper import introduced
by round 06 in a native-artifact profile without sequence_buffers.

The [round 07 packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-07/REPORT.md)
records **574× → 495× Roc**, 0.162907833 s to 0.140640417 s against a fresh
0.000284 s reference, or 13.67% less elapsed time. Source, artifact, allocations,
complete logical work, bytes, aliases, capacity and cleanup remain unchanged.
All 104 scoped Rust tests, alternate feature checking, twelve static checks and
protected scalar gates pass. Release rebuild takes 447.89 seconds and reuses the
machine libraries. The zero-test setup run, introduced import error, expected
preview failure and invalid filter command are retained and classified. Broader
linked/application and full-input coverage stay unrefreshed; CI is cancelled.
The profile removes most ordered schema precharge but still shows repeated
runtime visits, memory admission and object transport. The remaining 495× gap
requires larger reductions in real numeric-loop transactions. #973 stays draft.

## Exact mixed scalar state and closed failure exits

Protocol/test commits `37ba94f2812ea23f51401cbf7ba4a0346455e1c4` and
`8d1792af42e412a25441d35c6773b9a96d9826ae` reproduce the I64-pair and U32/U64-enum
eligibility miss independently. `7692a4453e1b67eba6609eb2f5295deaa565d69c` extends
the existing flat native layout using shared exact scalar kinds and bits; whole-
component escape, tags, 32-field bound and canonical public outputs remain.
`803e2e83bc76d31d4d2a5d5dad7b28ba04ab9f8b` keeps restoration tests in the existing
audited image harness. Actual source-derived private selection is 91 values /
310 words, including eight optimal-path Tail carriers. All 20 measured mixed-state
observations have zero allocation growth across zero to 64 visits.

The first expanded run passes 94 tests and fails checked-state error handling.
`c4681888b9d5e9a315842e5ec9a7d57cb81d1e9d` adds an all-U64 reproduction;
`c465f1712fed87e116771a9785207054258ce9ba` routes source assertion/rejection exits
through existing loan reconciliation, and `4af4662097dd570f0821fc105a1c4d9de4eb5d89`
fixes a status namespace typo found by compilation. A retained hash-checked old
executable proves the error-masking bug is inherited. The first parent attempt
used reused Cargo paths and is explicitly not parent evidence. Every setup and
qualification failure remains in the packet with its disposition.

The [round 08 packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-08/REPORT.md)
records **490× → 455× Roc**, 0.139637125 s to 0.129644084 s against a fresh
0.000285 s reference, or 7.16% less elapsed time. This is not the larger win being
sought. The pinned source, artifact, tables, algorithm, allocation sites and timing
boundary stay fixed. Complete logical work changes to 875610515844. All 109
scoped Rust tests and protected scalar gates pass, including exact bits, fresh
Boolean visits, failure precedence, prior publication, cleanup and retry. Static
checks pass; release builds take 458.00 s (Engine) and 72.28 s (host-only repair),
with all machine libraries reused. Broader linked/application and full-input
coverage remains unrefreshed; #973 stays draft and CI is cancelled.

The permanent default cost-table case still has about 44000 allocation requests
for 544 independent updates. The next experiment will construct independent
results directly, as a separate source revision/measurement. Preserve this
compiler-only pinned-source result and do not combine its gain with a source
rewrite or claim parity. No cancelled full-input campaign is restarted.

## Optional independent cost-table source experiment

The [round 09 packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-09/REPORT.md)
records a separate source variant, using the unchanged compiler executable:
**448.75× → 421.53× Roc**, 0.130136250 s to 0.122244917 s, fresh 0.000290 s
reference. This 6.06% gain does not meet the requested larger reduction. Six
focused tests cover original/decoded reference costs and vector boundary/error/
retry behavior with unchanged allowances. The default assembly and all source
pins remain. The optional variant increases bytecode 8.32%, has an extra fallback
row copy, and is not qualified for arbitrary multidimensional matrices. Setup
schema-join, Rust-fixture and source-definition-filter failures are retained.
The same executable/libraries do not require new scalar timing gates; no fresh
linked/application or large-input coverage is claimed. Twelve static checks pass.

Inclusive stack evidence places ordinary argument-frame handling in 600/1370
pinned-profile samples (43.8%), still 656/1492 with the source experiment. The
next assigned change is a general read-only unsigned-array leaf recurrence with
scalar state, including ordinary suspended parents, native typed loads, finite
fuel, owner lifetime and independently rederived image proofs. Selection must
come from validated plans and original factories, never compressor identifiers.
The existing true-cost loops and a generic weighted reduction are the acceptance
witnesses. This next capability is not delivered by the source experiment.

## Reuse exactly validated managed-projection keys

Protocol/regression `618d60eaa99802b842618d27f258ac3bc1958ec5` and implementation
`89a48f961a0905a83d81c3acd97a9e93fa40b691` reuse the existing prepared type-slot
key. Both preparation and independent restoration establish exact index/schema/
full-definition/table-key alignment. This fixes an inherited acceptance of two
swapped, individually valid slots. Current owners, kinds, extents and finite
admission stay fresh; no new cache or Engine type field is introduced.

The [round 10 packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-10/REPORT.md)
records **448.70× → 404.86× Roc**, or **9.77% less elapsed time**, on the unchanged
pinned eight-byte compressor. Exact bytes, inflation, aliases, reserve13 capacity,
cleanup and logical work 875610515844 agree. There are 41 scoped Rust passes and
twelve passing static checks. Seven of eight direct scalar timing gates pass;
decoded U64 is 1.0526× and fails the unchanged 1.05 gate, with exact allocation
counts/bytes. The cause is unresolved and the cohort is not rerun for acceptance.
The Path-expression diagnostic setup failure is corrected and retained separately.
Native-loop selection, broader linked/deployed tests and larger inputs remain
pending; the optional source experiment stays separate. The next assignment
continues the ordinary read-only numeric leaf capability, with private writes
and growth excluded from the first slice. #973 remains draft; CI is cancelled.


## Native read-only numeric leaf completion

Compiler `06885e0d9a1466ba477d642ee9fd9fd8510b06f2` completes the bounded read-only
slice. `3a16ffadb96d4c246b4eba86969902ccf3575e9d` adds all-edge invariant array
provenance, invocation-current packed views, native scalar lanes/loads/branches,
ordinary suspended-parent support and independent image proof/price validation.
`88ee934f882e4fc404abcd28dff0df6e0189e1c8` audits actual emitted successful hot
CFGs for calls. `e57c6278eed2a3f3d2becf4da3bcab18ae65e174` corrects the first
run's pre-entry account snapshot. IR116/work policy91 identifies the new work;
canonical bytecode revision14 and source remain unchanged.

The [round 11 packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-11/REPORT.md)
records **408.21× → 346.11× Roc**, or **15.21% less elapsed**, 0.117155000 →
0.099332833 s against a fresh 0.000287 s reference. Exact bytes, inflation,
capacity13, aliases and zero terminal bytes/nodes agree. The caller clears and
rebinds its prior result, but `same_backing` is false for every measured call on
both compilers. Physical output copying at the immutable public boundary remains
a timed cost. Earlier references to output reuse describe caller/capacity
retention, not demonstrated physical backing identity.

All 128 scoped Rust tests and twelve static contracts pass, including a real
restored linked program and actual finite-fuel carrier observations. Ten cold
exits restore current fields after 61/53/37/21/5 native traversals, preserving
publication through denial and retry. All fresh scalar timing and exact
allocation gates pass; the historical round-10 1.0526× observation remains
unresolved. The post-key weighted diagnostic loses its 7 allocation/128-byte/
57-visit per-element slope. Both forms stay at 245 requests, 40715 bytes and six
ordinary visits from zero through 256 elements. The full Object selects 45
parented native read-only regions, with 72 typed load sites and zero hot calls.

Focused linked/unit qualification uses explicit nondefault features and records
its actual Rust consumers. Default-profile compilation is restored for protected
timing; no build or test overlaps it. Failed source-fixture, profile-setup,
account-snapshot and observer attempts are retained with their dispositions.
The latest inclusive profile has frame handling on 482/1347 ancestries, frame
admission on 333 and ordinary visits on 245; these overlapping counts are not
additive wall-time buckets. Private indexed-write recurrences are the next
measured compiler slice. Larger inputs, general producers, growth and SIMD remain
separate. #973 stays draft, review is requested and CI is cancelled by request.

## Native private unsigned indexed-write completion

The [round 12 packet](../../benchmarks/mech-deflate/results/2026-10-28-native-hardening/compression-round-12/REPORT.md)
records final production compiler `d105f884bc79d464a205b3022da4abdc00da41aa`,
based on `efa2ff81741362394553d5c7eae7d9dfc39766c4`. The unchanged eight-byte
compression pair is **345.68× → 251.38× Roc**, 0.099901542 → 0.072649542 seconds
against a fresh 0.000289-second reference: **27.28% less elapsed**. Earlier
provisional and corrected-handoff cohorts remain separate, with failures and
corrections retained. Source, bytecode, tables, real buffer sizes, allocation
sites, mimalloc object and timing boundaries are unchanged. Physical output
backing still changes on both sides despite retained capacity 13.

Core pre-admits a future immutable owner for exclusive fixed-shape unsigned
updates. The numeric-leaf proof derives current functional versions on every
path; arrays retain owners separately from scalar SSA fields. At the first
changed write, the original consuming factory decides exclusivity. Private
initial backing is retained and shared backing copies once. The unchanged
allocation contract caught an initial carrier-pin copy, corrected by
`c0bef396bbc88cdd8dd94037053866b182b7a68d`. The final compiler also preserves
selected output-element ceilings for no-op stores at their source position.
IR117/work policy92 records current-version proof, prepaid publication and
same-account cold resumption; canonical bytecode14 and ABI35 are unchanged.

All 231 scoped Rust tests pass: 87 Core snapshot, 129 maintained component,
8 scalar, 3 bits, 3 actual runtime units and 1 generated linked application.
Sixteen actual write-fuel exits freeze current cells and arguments without
renewing admission; ten read-only exits retain the earlier guarantee. Adversarial
publication tests cover arena/generation/type/account/bound mismatches. The linked
application executes 3 → 0 → 2, preserving assertion-before-division order,
prior publication, cleanup and successful recovery after a private write.
Qualification receipts and the test-only typed-work-error correction are
`e7767697e1b1bdc6e8013a499a1cecbb80319a22`.

All twelve static contracts and twelve fresh scalar pairs pass, with exact
allocation equality and maximum direct ratio 1.0180374 against the unchanged
1.05 gate. The historical round-10 1.0526036 failure remains unresolved. The full
Object selects 91 write regions (88 parented), 117 typed stores and 315 loads;
successful already-private CFGs have zero calls. These are static sites. The
separate final profile still has frame handling on 446/1356 ancestries and frame
admission on 318; overlapping ancestry is not additive time attribution.
Mixed array/scalar results in the real cumulative Huffman loop are the next
bounded candidate. No larger timing or new optimization is included here.
#973 remains draft; the previous code review is clean, fresh review is requested
for closure and CI is cancelled by request.
