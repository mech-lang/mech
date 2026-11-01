# Native compression: current status

Raw observations, diagnostic drivers, full censuses and rejected experiments stay
outside source. The maintained harness and [compact evidence index](../../benchmarks/mech-deflate/experiments.json)
retain the reproduction path. [PR #973](https://github.com/mech-lang/mech/pull/973)
stays draft pending green CI and clean review. Publication CI is cancelled by
instruction; no green hosted-CI claim is made.

## R102: inline paired budget debit; retain measured gain

Compiler **`fa1ffe75e78c7ace538d7d14aa5f3ebc74783166`**, previous published head
`c489e5fc30a4e9cea549cc1de66e2c94ab2f9eb9`, stack base
`eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. One `#[inline(always)]` keeps the
shared paired-debit checks in their hot callers. No checks, fee, limit, debit
order, worker atomicity or benchmark source change. R101's ordinary `#[inline]`
was ineffective, measured adverse, archived, independently replayed and reverted.

Fresh matched complete eight-byte compression **8.477859×→8.156141× Roc's time**:
2.4331455→2.3408125ms against a0.287ms reference, **3.794800% less elapsed time**.
All four matched pairs and64 descriptive blocks improve. All3968 primary/3984
total observations remain; blocks are not independent experiments. Historical
ratios with different reference clocks are not gain estimates.

| Metric | Fresh retained R100 | R102 |
|---|---:|---:|
| Instructions, separate cohort |27,930,744.5|26,988,708 (−942,036.5; −3.372758%)|
| Cycles, same hardware cohort |7,594,187|7,435,497 (−2.089625%)|
| Requests / reallocations |5,077 /162|unchanged|
| Requested bytes / incremental live peak |95,895,278 /17,782,744|unchanged|
| Generated text / entry stack |14,604,696B /135,728B|byte-identical /unchanged|
| Whole linked text, including runtime |63,674,544B|63,675,184B (+640B)|

Actual host disassembly removes the separate520-byte helper. The library caller
512→736B and observer428→628B now contain the checks directly. The complete
Object, descriptor and metadata stay byte-identical. This is a controlled
R100-to-R102 result; it does not assign all historical R99 instruction growth or
all Result transport to outlining. Fresh isolated profiles cover695 calls per
subject; samples remain separate from retired instructions.

Passes: actual Engine budget108, runtime356, work/memory33, guarded native CI65,
image1, unchanged components236/connected8/path1, linked40, scalar12 (same1.05
gate; max1.016923, pure-scalar allocation equality, loop slope0), frame8,
workflow132, architecture15 and current runtime-only GPU cfg. Five affected
crates rebuild coherently, Core/math frozen, warnings-as-errors unchanged. A
Git metadata permission denial occurred only when recording sixteen already
passing linked cases; the same sealed packet was committed with authorized
access. No program rerun, receipt rewrite or source/allowance change was needed.

Logical work42,410,605,541 and IR/work/ABI/bytecode143/99/38/14 are unchanged.
Algorithm, tables, buffers, allocator, input aliases, cap13 output clear/rebind
and evaluate_into timing stay pinned. Exact bytes, independent inflate and
final zero managed bytes/nodes pass. The existing expensive knucleotide-body
exclusion remains; no full math/all-features/minimal graph, larger-input timing
or green hosted-CI claim. Public output backing still changes.

The new complete allocation trace independently reconciles5077 requests,
95,895,278 bytes and162 reallocations. It finds336 eight-byte replacement
requests (immediate callee deduplicated; exact ancestry/return offsets retained)
and33632-byte final shape owners. The672-request/13,440-byte shape-forwarding
candidate is confirmed as a current target, not an achieved saving. Next: shape
owner forwarding, then prepared schema headers. The independent instruction
track remains prepared frame transfers, known-zero arguments and entry-local
checked-source reuse.

## Previous retained checkpoint: R100 — move ordinary binding metadata; no elapsed win established

Compiler **`bebe5a245d312796c630de76fc558fbcd2c2f2ff`**, published parent
`c84e307aa64e136bd0d3f6d2fc2bbf869b4e3b25`. The no-witness binding branch now moves
the original BoundCall into its sole kernel consumer. Certified scalar kernels
still retain independent executor/seal metadata; all builder invalidation,
accounting, current-value validation and admission order stay unchanged.

Fresh matched complete eight-byte compression is **8.268488×→8.281428× Roc's
time**, 2.422667→2.4264585ms against a 0.293ms reference: **+0.156501%** pooled
median elapsed. Two pairs improve and two worsen; 25 descriptive blocks improve
and 39 worsen. All observations remain: 3,968 primary and 3,984 total. No elapsed
improvement is established. R99's historical 8.363107× used a different reference
clock and must not be subtracted from this ratio to claim a gain.

| Metric | Fresh R99 baseline | R100 |
|---|---:|---:|
| Instructions, separate cohort |27,950,216.5|27,933,245 (−16,971.5; −0.060720%)|
| Cycles, same hardware cohort |7,697,293.5|7,622,510.5|
| Allocation requests |5,149|5,077 (−72; −1.398330%)|
| Requested bytes |95,900,682|95,895,278 (−5,404B)|
| Incremental peak live bytes |17,782,744|unchanged|
| Reallocations |162|unchanged|
| Object/image/metadata |R99 artifacts|all byte-identical|
| Generated text / entry stack |14,604,696B / 135,728B|unchanged|

Retain this as a small allocation/instruction cleanup, with no elapsed-win or
peak-live-heap claim. Price formulas, logical work 42,410,605,541 and versions
IR/work/private ABI/MBC143/99/38/14 stay unchanged. Source, tables, search limits,
real scratch capacities, allocator, output capacity13 clear/rebind and evaluate_into
timing remain pinned. Mech computes the exact raw bytes and independent inflate
passes; input aliases and final zero managed bytes/nodes remain verified.

Passing current selections: nine actual Engine semantic-seal unit tests, runtime356,
work/memory33, guarded native CI65, image1, unchanged components236/connected8/path1,
linked40, scalar12, frame8, workflow132 and architecture15. Scalar gate remains1.05;
the isolated rerun's maximum ratio is1.010357, pure scalar requests/bytes stay
equal, and loop allocation slope remains zero. The initial scalar batch also
passed but is excluded because a descriptor read overlapped its start. All its
observations are preserved. An auxiliary Engine test driver initially used
metadata-only extern paths and an incompatible filename flag; switching to the
same frozen rlibs fixes the driver without compiler or allowance changes.

Five Engine-dependent crates rebuild coherently with the retained optimized
operation profile and warnings-as-errors; Core/math dependencies remain frozen.
The additional actual Engine unit root uses opt-level1 and the same selected
features. No new Core, all-features, full-math or larger-input qualification is
claimed. The existing expensive knucleotide-body runtime exclusion remains.
Fresh serial before/after profiles cover695 complete calls per subject.

### New instruction finding: runtime budget helper outlining

A current calibrated R97/R99 audit finds exactly9,801 callbacks in both versions.
R99 adds only eight ownership-candidate checks (980→988) across200 matrix updates;
57 buffer visits and260 buffer checks stay equal. Broad additional retirement
scanning is not established as the instruction regression's cause. Four fresh
hardware pairs reproduce about2.05% more instructions in R99, with lower cycles.
Instrumented windows remain estimates, separate from uninstrumented counters.

The budget source file is byte-identical across R97 and R99 (Git blob
`bbcc2b4c97ac2a61ebe6b0a912349c8a85ef33cb`). Actual disassembly finds a732-byte
library charge routine with its checks inline in R97. R99 instead has a512-byte
caller plus a separate520-byte charge_control_account_at_limits routine, with
intermediate Result transport. R100 still has this split. This establishes a
binary/code-generation difference; its exact contribution to the regression
remains unproved. The next bounded experiment explicitly requests inlining of
that helper and measures it separately, retaining every finite check and error
order. Prepared flat frame transfers and per-entry source reuse follow.

Seventeen immutable evidence packets verify1,249 file occurrences across538
paths and29 preserved multi-version paths. Fresh retrieval reproduces all four
reports byte-identically. The first retrieval exposed a missing linked metadata
input; a new replay-input packet fixes packaging without changing compiler source
or measured data. Commands, all raw cohorts, profiles, assembly excerpts and
diagnostic readers remain archived. Native binaries remain local. Local
validation is complete; hosted CI is cancelled by instruction and the PR stays
draft pending green CI and clean review.

## Previous retained checkpoint: R99

### Retire finished ordinary collection sources

Complete eight-byte level-12 compression improves from **8.470875× to 8.363107×
Roc's time**, **1.272220% less elapsed time** against the fresh matched baseline.
All four pairs improve; 61 descriptive blocks improve and three worsen. All
adverse observations remain. There are 3,968 primary and 3,984 total observations.
Blocks within a pair are descriptive, not independent experiments. The reference
is 0.289ms; Mech is 2.448083→2.416938ms. The target is 1×. Larger-input throughput
remains unestablished. R97's historical 8.378794× used a different reference clock;
it is not the baseline of this comparison.

| Metric | Fresh retained R97 baseline | R99 |
|---|---:|---:|
| Instructions, separate 32-call cohort |27,389,660.5|27,947,877 (+2.038056%)|
| Cycles, same hardware cohort |7,732,271.5|7,622,493 (-1.419745%)|
| Allocation requests including reallocations |5,153|5,149|
| Requested allocator bytes |97,997,834|95,900,682 (−2,097,152B)|
| Incremental peak live allocator bytes |17,782,744|unchanged|
| Reallocations |162|unchanged|
| Generated compressor text |14,604,696B|byte-identical|
| Machine entry stack |135,728B|unchanged|

The independent reference hardware cohort retires 1,364,372 instructions and
921,002 cycles. Reference allocation measurements retain 95 requests,
9,021,923 requested bytes and 9,257,328 incremental peak live bytes. These are
observed implementation results, not semantic minima. Hardware, heap, samples
and primary elapsed are separate cohorts. Requested capacity is not initialized
copy volume or live memory. Peak live heap does not improve. Hardware-cohort RSS
is 319,963,136→319,700,992B and footprint is 228,968,128→228,919,040B; these include
prepared code, libraries and harness state, rather than just compressor buffers.

## What changed

The Engine's old-value graph query previously treated CollectionSource as opaque,
so an ordinary completed comprehension could keep its input alive indefinitely.
The query now places ordinary materialized source reads at initialization,
header, body, latch, exit and collection finish. Finish matters: shape-preserving
finalization reads the arena source after the generator releases its source pin.
The same graph traversal preserves backedges, fresh definitions and read/write
order. Unsupported consumers remain conservative. Private-frame eligibility and
Core's current same-account/exclusive ownership check remain in place.

The real tree trace finds aliases 882 and 908 before each update. Previously 882
remained after retirement and forced a copy. After the fix neither remains; all
four 262,144-entry U16 updates reuse their 524,288-byte backing. Four complete
calls verify all 16 updates. This establishes exactly **2,097,152 initialized
bytes no longer copied per complete call**. The trace's arena scan is diagnostic
only; no production owner scan, source-field specialization or algorithm change
is introduced. The selected site's static inventory grows 45→47, adding 882 and
1857. Later readers and externally retained aliases still prevent unsafe reuse.

An independent Mach-O section audit finds the entire generated text byte-identical:
SHA-256 `65099c20101e43aaa1e355c0758a40316fc7ed6aa3ad358b1e0b5c8ba07020d4`.
Object/image identity and ownership metadata change. IR143 reconstructs the
new proof independently on image restoration; rejected R98's IR142 is not reused.
IR/work/private ABI/MBC are **143/99/38/14**, scalar ABI1. Strict prior-image
rejection remains. Price formulas are unchanged; actual logical work falls
42,449,879,781→42,410,605,541 as already-accounted copying/ownership paths change.
Logical work is not a hardware instruction count.

## Instruction tradeoff and fresh profiles

Whole-call instructions increase by 558,216.5 despite lower elapsed time and
cycles. This adverse result remains explicit; R99 is retained for its elapsed
and physical-copy reduction, without an instruction-reduction claim.

Separate nonnested windows around the four tree helpers observe raw instruction
totals 473,722→87,110.5 and cycle totals 209,935→32,007.5 per full call, over 32 calls
per subject. Empty read-pair calibration averages 5,006 instructions; raw window
totals include counter overhead and are not subtracted from the uninstrumented
whole-call cohort. No complete per-function instruction ledger is established.
The whole-call increase remains unattributed outside that bounded diagnostic.

Fresh serial profiles cover 695 complete calls per subject and 1,302/1,293
exclusive sample weights. Copy/move ancestry falls 41/1,302→15/1,293 weights;
frame-action ancestry is 131/1,302→113/1,293. These are overlapping ancestry unions
of CPU samples, not retired instructions or exact timer fractions. Runtime
helper, admission, native-segment and cleanup costs remain prominent.

## Revisions and qualification

Published parent: `64705927265fa331735ef39cdb3185471015054e`.
Retained comparison compiler: `20f002f4ff5ff2186888b7efc8517261756b425e`.
Regression/CI inventory: `bdf3df43a2c0d5dc9483ae3ade9793f85de2a0f7`.
Compiler fix: **`5c219ed220879b05ee147bdf5d3f85680311eae2`**.
Test-only final-root publication correction: `348101721a2870ab2f503508491e7d3395c94b32`.
Stack base: `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`.

The new regression fails against the retained Engine at missing source eligibility,
then passes after the fix. Original/decoded artifacts and original/restored images
cover selected/unselected mapping, current arguments, a later published reader,
external input aliases, unused failing results, assertion/index error order,
unchanged prior publication and retained accounting through failure and retry.
Final managed bytes/nodes remain zero.

Passing current selections: runtime356, work/memory33, native CI65, image1,
unchanged components236, connected8, path1, linked16+12+12, scalar12, frame8,
workflow132 and architecture/type/memory15. CI's guarded inventory is updated
64→65 and actually lists and executes all65. Scalar allowance stays 1.05; the
maximum ratio is 1.018146. Pure scalar requests/bytes stay equal and loop allocation
slope remains zero. Frame request/byte/work counts do not increase. Component
expected values and allowances are unchanged.

Engine, Stdlib, Compute, Runtime and GPU are coherently rebuilt with the same
selected operations, optimized flags and warnings-as-errors. Core, Syntax,
Bytecode, math libraries and external dependencies stay frozen. No fresh Core,
full-math, minimal-dependency or all-features graph qualification is claimed.
The existing expensive knucleotide-body runtime exclusion remains explicit.
Formatting and architecture/catalog/type/memory checks pass.

Preserved preparation failures: an initial diagnostic reporting prefix of16 was
insufficient for45 static candidates and was replaced by a complete inventory;
this was no production resource denial. An initial narrow-arithmetic witness was
outside the retained lowerable subset and was corrected to the compressor's wide
arithmetic/cast pattern. The new test's initial publication expectation was
corrected to the language's final-root rule. No existing component golden changes.
No current correctness/test failure remains; the instruction increase remains
an adverse performance result.

## Fidelity, evidence and next bounded work

Source SHA-256:
`331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`.
Bytecode SHA-256:
`a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`.
Level12 retains the 262144-entry U16 tree, 1500513-word U32 cache, 305257-entry U64
path capacity, depth300, nice258 and original near-EOF rules. Search and skip stay
separate. Algorithm, tables, source allocation sites, allocator and evaluate_into
timing stay pinned. Exact raw output is `[75,76,74,78,73,77,75,207,0,0]`, checked by
independent inflate. Input aliases and output capacity13 clear/rebind remain.
Physical public-output backing still changes; physical output reuse is not proved.

Every measured cohort is sealed and committed. Fourteen uploaded evidence
archives verify 1,032 file occurrences, 463 paths and two preserved multi-version
paths. Fresh independent retrieval reproduces all four numerical reports
byte-identically without compilation or compression. The initial automatic upload
rejection was resolved by explicit authorization of evidence uploads. Native
Objects and executables stay local; descriptor/excerpt/hash retrieval does not
establish native-Object retrieval. No larger-input run is performed.

Next, explain the remaining instruction cost using exact callback/ownership
visits, remove the no-witness BoundCall clone, then stage immutable constructor
binding/planning and prepare flat frame-transfer geometry. Current funding,
values, error order and publication remain dynamic. The rejected R98 direct child
seed experiment stays reverted; its 112 executed handoffs were slower and are
not proposed again as unexplored work.
