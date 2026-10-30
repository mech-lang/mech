# Parent/child numeric composition: design gate

Status: **reproducer and proposed execution contract, without a compiler change**.
The production compiler remains `915f039cfd7e2647cc3a1d349ab1070c3f292d19`.
The retained compression result remains round 32: **16.937986667× the reference**
on complete eight-byte compression. This diagnostic establishes no new speedup.

## Real operation and minimal witness

The reached complete-program parent is iteration 190/header 17352,
`huff-assign-lengths`; its child is iteration 191/header 17390,
`huff-assign-symbols`. The existing callback census attributes 848 admissions
and 610 helpers to the parent. The child already has native execution; its
remaining helpers are first-write boundaries. These are retained census counts,
not a fresh timing or a prediction that all those calls will disappear.

For each width, the parent reads a count and invokes the child. The child writes
that many code lengths and returns **both the updated buffer and next symbol
index**. The parent must use both current results on the next width. This is
ordinary typed state and explicit results in the existing language.

`huffman.mec` extracts eight unchanged helper/recurrence definitions from the
maintained `deflate-huffman-level12.mec` fixture and adds one invocation. It is
27 lines including imports and blank lines. It does not replace or modify the
compressor. `witness.rs` uses the existing component harness's frontend, bytecode
codec, input builder, resident oracle, JIT and snapshot comparisons.

Observed on the retained compiler, for both original and decoded artifacts:

| Minimal program region | Result |
| --- | --- |
| Parent iteration 0/header 4 | Not selected: `not a leaf: child loop or activation boundary` |
| Child iteration 1/header 42 | Selected numeric write region |
| Child successful native backedge | Zero helpers |
| Child first-change boundary | Site 83 |
| Child proved compute per traversal | 40,184 |

One executed Rust diagnostic passes, covering twelve semantic observations:

- Three nonempty widths: symbols `[2,0,3,1]` produce lengths `[3,1,3,2]`.
- Changed start/counts: lengths become `[2,1,9,2]`, detecting stale index/buffer reuse.
- Zero width skips the child, even with empty symbol/count buffers.
- Zero counts permit repeated empty child calls and preserve the buffer.
- A short output rejects at index 4/upper bound 2 after one valid write.
- Retry on the same JIT and memory account produces the expected output.

Success agrees with both independent literals and the resident executor. Failure
agrees with resident error classification, preserves the prior published snapshot
and input aliases, restores retained bytes/nodes, and permits recovery. Final
cleanup is zero. No compression timing was performed for this design witness.

Raw receipts are under
`results/2026-10-30-native-hardening/parent-child-design/`. Attempts 01 and 02
are introduced harness build errors (imports and omitted build environment).
Attempt 03 successfully builds but its runner uses the wrong output path.
Attempt 04 reuses that exact successful binary and executes **one test, zero
failures**. All failed attempts remain archived; no compiler defect is inferred
from these runner errors. This standalone diagnostic is not counted among the
561 scoped maintained regression tests or claimed as scheduled hosted coverage.

## Existing restrictions are necessary

`prepare/readonly_numeric.rs` currently requires a leaf body, and
`prepare/tail.rs` constructs one acyclic per-traversal envelope. Runtime
`invocation/tail.rs` permits one `closed_tail` and reconciles compute as an exact
multiple of that region's envelope. `buffer_frames.rs` permits one private
numeric-owner set. The child is legal today because its parent is ordinary and
suspended. Removing the leaf check would violate these assumptions.

## Proposed first shape

Support a parent with **one statically identified child call edge** per traversal,
with an already-qualified numeric child and flat typed carriers/results. Begin
with fixed-size unsigned arrays, scalar control, and replace-only writes. The
real witness has U32 symbols, U64 counts, a U8 result buffer, and a child result
`([u8],u64)`.

Eligibility must depend on checked types, CFG edges, frame mappings and the
child's proof, never function names, block numbers or this input. Keep ordinary
execution for multiple child edges, deeper composition, activation changes,
maps, append/capacity changes, escaped carriers or unproved result forwarding.

### 1. Preparation and canonical reconstruction

Derive a composition descriptor from the validated parent CFG and selected child
proof. Record the child initializer/exit, exact successor continuation,
argument/result field mappings, and parent segments before/after the child.
Contract the child subgraph only after proving its entry, exit and dominance
relationships. Reuse the scalar/aggregate/frame operations, SSA adapter, emitter
and cost validation already maintained by the compiler.

For the writable child array, use the child's proved result mapping: tuple field
zero returns its current lens array, and field one returns its current index.
This establishes a **current returned owner**, not equality to the parent's
original backing. Reject an ambiguous result, swapped array or unknown writer.
Reconstruct all derived mappings and costs independently on image load; wire
input cannot grant them. No new bytecode opcode, executor or runtime owner.

### 2. Parent suspension and child entry

Use nonoverlapping native segments around the existing child execution. At the
call boundary, spill current scalar fields through existing checked frame lanes,
settle the parent's active segment, and release its views/private owners before
entering the child's existing initializer. A bounded continuation identifies the
composition, exact call edge and permitted parent successor; it conveys no
array mutation authority.

The child receives current arguments from those lanes, including the latest
buffer and index. Existing current-invocation, predecessor, account, liveness,
retirement and first-write rules remain authoritative. Only one closed native
segment and one private-owner set may be active. Do not carry an earlier array
pointer across a helper mutation or add a new raw-access loan.

### 3. Current results and parent resumption

Let the child settle through its existing result and ownership protocol. Install
the returned tuple's current array and index in the parent's checked successor
lanes. Validate the continuation and exact completed child invocation before
resumption. Acquire any required views afresh from the current owners.

The parent then decrements width and recurs using those results. The next child
must observe every previous child's write. Zero width executes no child edge;
zero child count still completes the actual child invocation. Do not invent
new values, replay the initializer, resume at an earlier header, retain a second
private owner, or substitute the parent's entry-time array.

### 4. Finite funding: separate segments on the same account

Do **not** insert child spending into the existing whole-parent equation
`consumed = traversals × parent_body_envelope`.

Instead, independently certify the acyclic parent prefix and continuation using
the common instruction-cost validation and existing prices. Settle each reached
segment before transferring control. The child keeps its existing per-traversal
funding and reconciliation. Every segment/child uses the same invocation account
and post-settlement remaining amount; no nested account or refreshed allowance.

The ledger must establish, with checked arithmetic:

`spent = reached parent segment envelopes + actual child charges + boundary work`

The parent's segment inventory excludes the child body. A child traversal is
therefore charged once, and an unselected parent continuation is not charged as
if executed. Each generated fragment is reconciled against its own certified
entry/exit, rather than dividing spending across a child by one parent cost.
Keep the child's current envelope and all memory/node/header/work prices.
Any additional boundary bookkeeping must be admitted explicitly.

At a fragment entry, insufficient funding declines native entry before that
fragment executes. Ordinary fallback begins at the exact unexecuted source site
on the same remaining account. A settled fragment is never replayed or charged
again. Child fallback follows the existing child path; it does not rewind the
parent. This segment design avoids a partly prepaid parent body surviving across
arbitrary child/helper spending or fallback.

### 5. Failure and cleanup

An assertion, arithmetic error, invalid access or resource rejection retains its
source position and first-error order. Do not manufacture an early failure at
composition entry for an operation on an unselected path. Existing bounds and
first-write exits materialize the selected operation through their maintained
path. Failure clears the pending continuation, active view and private-owner
state through existing invocation cleanup; prior published outputs and external
aliases remain intact. Spent work is not refunded. Retry starts with fresh
invocation-local state and current arguments.

## Acceptance before production retention

First register minimal original/decoded tests through the maintained component
harness. Preserve the current witness's values and add positive composition
selection and successful-parent-backedge evidence. Exercise changing counts,
array contents and start positions across nested/repeated calls; zero/unselected
children; first-write and later-child rejection; parent/child assertions and
arithmetic in both orders; exact/one-short compute, comparison, memory, nodes and
transport; prior publication/aliases; cleanup and retry; and independently loaded
images with corrupted mappings/continuations/costs rejected.

Verify the ledger equation at each boundary using actual consumption. Prove no
renewed fuel, stale parent baseline, overlapping owners, replayed prefix or
double charge on ordinary fallback. Check both native and declined entry. Retain
all unsupported shapes on the ordinary route and keep scalar controls unchanged.

Then rerun the existing component/linked and architecture qualification. Confirm
the real reached parent changes selection/callback traffic without changing any
compressor source or allocations. Compare generated code size and profile the
new binary. Only then perform the same protected complete eight-byte paired
timing, using a fresh common reference, pinned allocator and unchanged boundary,
and commit every raw cohort immediately. No larger run or performance claim is
authorized by this diagnostic result.

The next implementation step is the descriptor/proof for these parent segments
and exact child result mapping, with a failing maintained selection test first.
The runtime suspension/funding contract above must be implemented and tested
before enabling composition; merely suppressing today's rejection is invalid.
