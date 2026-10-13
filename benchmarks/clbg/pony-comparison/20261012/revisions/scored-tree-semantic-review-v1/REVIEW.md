# Pony original scored tree semantic review

Source-only independent review, 2026-10-13. This new note does not change the
failed work-v5 report, the six completed trace records, the recovery results,
the original sources/binaries, or any timing/chart data. No hash, import,
fixture, compiler, disassembler or application execution was performed by this
reviewer. Artifact digests below are copied from the actual closed result and
terminal, not calculated by the reviewer.

## Conclusion and exact meaning

Both original tree programs implement a fresh, complete recursive Node graph,
with separate leaves and child subgraphs, and retain the long-lived graph
through every worker join and its final check. The four pending semantic fields
can be proposed as true for both exact profiles after root/peer acceptance and
an additive metadata seal:

| Field | Proposed verdict | Evidence and scope |
| --- | --- | --- |
| `leaf_path_allocates_fresh_node` | true | Every recursive depth-zero Node is allocated by its parent before the depth-zero constructor stores two None children. The final scored ARM preserves those allocations. This is not a claim that the depth-zero constructor itself contains an allocator call. |
| `interior_path_allocates_fresh_node` | true | The authored root and each interior child are separate new Nodes. Final ARM preserves allocated recursive descendants and a separately allocated long root. The compiler promotes the nonescaping stretch/worker root into stack storage and registers; this field means fresh authored object semantics with compiler optimization permitted, not that every interior Node remains heap allocated. |
| `no_static_or_shared_tree_graph` | true | Each live recursive child comes from a separate allocator result and is constructed at depth minus one before being stored in its own child field; the two recursive calls do not borrow another graph. Worker iterations construct both subgraphs again. Shared Node type descriptors and the None sentinel are not shared Node graphs. |
| `long_root_storage_retained_through_worker_joins` | true | Main stores the independently allocated long root at offset320/0x140, its GC trace visits that root and each Node trace visits both children, and final checking occurs only after the pending-worker counter reaches zero at the last depth. Workers hold a traced Main reference. |

The current authored policy in frozen `revisions/work-full-v5/static_review.py`
requires every interior node and leaf to be a separate Node, a fresh transient
tree per bounded behavior, and Main's long root retained until final output.
`full_prerequisite.py` requires the four named fields plus annotated scored ARM
and source-matched IR; it does not require a heap allocation for every optimized
temporary root. Root explicitly confirmed the distinction between fresh object
semantics and compiler-driven stack promotion during this review. A stronger
claim that every scored interior/root object is heap allocated is false and
must not be emitted by the metadata seal or described as a measured allocation
count.

## Closed input identity

All paths in this note are relative to `benchmarks/clbg/pony-comparison/20261012/`
unless prefixed with `continuation/`, which is relative to `benchmarks/clbg/`.

Root's actual physical closure is
`continuation/jobs/pony-runtime-target-closed-review-v1/receipt.json`, digest
`ef0f4a313d76473bc52dad938be6f6a8245ef0c16b2a592cdd50fd5d5cfe0f0b`.
It records 10,198 verified pins, six normal native evidence commands, no
application/benchmark/fixture commands, two original profiles, and reuse of the
successful compact compile and entry decode.

The actual result is
`revisions/runtime-target-continuation-v1/jobs/remaining-scored-evidence-v1/result.json`,
digest `c0895574ca921e74941b012def48e0396746447b5cf92320c75b45cc35158f12`.
Its actual body terminal digest is
`b5dc136bbef864d67d350a925a7bd96d96db6fd1cb1909f799618cf137bdc450`,
and its closed durable terminal digest is
`04b15344fe06675b0066df55acf4173f05a17b5cce05636ed922a9bd1b2e14df`.
Those records continue to say that semantic review is pending; this note never
relabels them.

| Profile | Original source SHA256 | Original scored binary SHA256 |
| --- | --- | --- |
| `pony-binarytrees-compact-precedence-v4` | `9fe1b9d288f21559aaf6d40706cd6fe3c969bfdddcbbaa0900fa3d97cc670a20` | `32e8e965c377dfe9b872adbd0777bcaf8d34686b75606d4038c8cf0d0e0e7e30` |
| `pony-binarytrees-optimized-precedence-v4` | `351177026d084edcdabdb79651db63fffcb68edaee223b908f31e55f606f295a` | `d2e3f363a13d4d7b7f56ffc9dc10f31f58d89f28358ee79e1a0de518ba2aa033` |

Sources are the full files at
`qualification/private-v6/source-snapshot/revisions/precedence-v4/sources/binarytrees/{compact,optimized}/main.pony`.
Original binaries are `.build/private-v6/bin/PROFILE/PROFILE`; their separate
production receipts are `qualification/private-v6/builds/PROFILE/build.json`.
Both complete authored sources were read. Their only source difference is
Main's `_pending = 1` versus `_pending = _iterations.min(8)`.

Both exact production commands, compiler/source/library identities, diagnostic
commands and owned receipt closures remain in the actual result and terminal.
The diagnostic command changes only the exclusive output and adds `--pass
bitcode`; the ARM text is produced from the unchanged original scored binary.

## Annotated IR and final scored ARM

Compact authored IR:
`revisions/native-evidence-recovery-v2/jobs/scored-bitcode-v2/pony-binarytrees-compact-precedence-v4/authored-entry.ll`,
digest `190ab8b1a59c0b98abb33193e13c8e255ba11ac88808eeb0fe1e940312d96d61`.
Optimized authored IR:
`revisions/runtime-target-continuation-v1/jobs/remaining-scored-evidence-v1/pony-binarytrees-optimized-precedence-v4/authored-entry.ll`,
digest `af1f49f2a2c6fdbef66a0a4235c477b2fe14bbf70aec1b3b6c90a1f278daec7c`.
These are the separately identified authored entry modules; neither overwritten
old `.main.ll` nor separate builtin IR is substituted.

In both IR files, `Node_ref_create_pZo` at line975 branches on depth zero.
Its nonzero branch calls `pony_alloc_small` separately for left and right,
stores `Node_Desc`, recursively constructs each at depth minus one, and stores
the resulting pointers at Node+8 and Node+16. Its depth-zero branch writes the
None sentinel to those two fields of the already allocated Node. `Node_Alloc`
at line1011 returns a separate allocator result with a noalias return contract.
The source-matched IR Worker body (compact2405, optimized2406) allocates and
constructs each root when remaining is nonzero, checks both child subgraphs,
adds the root's one to the sum, decrements remaining, and schedules its next
behavior. No precomputed check substitutes for recursive construction.

Both IR Main constructors at line1188 separately construct the stretch tree and
then the long tree; the latter result is stored at Main+320. `Main_Trace` at
line578 traces that exact field with `Node_Desc`, and `Node_Trace` at line513
traces both children. `Main_Desc`/`Node_Desc` at lines76/75 bind those actual
trace functions. `Worker_Trace` at line672 traces its Main reference. Both
`Main_tag_done_pWo` bodies at line1029 decrement Main+344 and return while it
is nonzero; only at zero does the program advance depth or load Main+320 for
the final recursive long-tree check. `_start` at line1325 passes Main to every
worker and records the batch's exact pending count.

Compact final ARM:
`work-qualification/private-v8-work-v5/scored-tree-allocation/pony-binarytrees-compact-precedence-v4/arm/scored-arm.stdout`,
digest `3053732417af086c0754d2c82a7fc1c1cb92a4046f0d40b9ef9dec400e5fe289`.
Optimized final ARM:
`revisions/runtime-target-continuation-v1/jobs/remaining-scored-evidence-v1/pony-binarytrees-optimized-precedence-v4/arm/scored-arm.stdout`,
digest `e271b7fa46a50bbd78447af190656cdbdb5dddd3b5adda75611dadb5edf13f96`.

| Function / path | Compact address witnesses | Optimized address witnesses |
| --- | --- | --- |
| Recursive constructor | Entry `0x100004858`, depth-test `cbz` at `0x10000486c`; allocator calls `0x100004884`, `0x1000048ac`; recursive calls `0x10000489c`, `0x1000048bc`; child stores `0x1000048a0`, `0x1000048c0`; leaf child sentinel pair `0x1000048e0`. | Entry `0x1000048b4`, depth-test `cbz` at `0x1000048c8`; allocator calls `0x1000048e0`, `0x100004908`; recursive calls `0x1000048f8`, `0x100004918`; child stores `0x1000048fc`, `0x10000491c`; leaf child sentinel pair `0x10000493c`. |
| Worker nonzero-depth tree | Root descriptor at aligned stack slot `0x100005910`; separate child allocations `0x100005924`, `0x100005940`; constructors `0x100005934`, `0x100005950`; actual child checks `0x100005a00`, `0x100005a0c`; root one added `0x100005a14`. | Root descriptor at aligned stack slot `0x10000596c`; separate child allocations `0x100005980`, `0x10000599c`; constructors `0x100005990`, `0x1000059ac`; actual child checks `0x100005a5c`, `0x100005a68`; root one added `0x100005a70`. |
| Long root allocation/storage | Main constructor allocator `0x100000df0`, constructor `0x100000e00`, store at Main+0x140 `0x100000e04`. | Same original addresses in optimized Main constructor. |
| Worker-join gate / final long check | Done's decrement/store `0x100000948`/`94c`, zero-only branch `950`; load long root at `0x100000a38`; recursive child checks `0x100000b74`, `b80`. | Same original addresses in optimized Done. |
| Long root GC traversal | Main trace loads Main+0x140 at `0x1000045a8` and calls/tail-calls `pony_traceknown`; Node trace at `0x10000085c` loads/traces both Node children. | Main trace loads Main+0x140 at `0x100004604` and calls/tail-calls `pony_traceknown`; Node trace has the same original address/body. |

The stretch root is optimized like the worker root: Main writes a descriptor to
its aligned stack slot and keeps child pointers in registers, allocating both
subtrees before checking them. Final ARM therefore does not materialize the
temporary root as a complete 24-byte heap object. It still constructs both
recursive subgraphs and counts the root exactly once. Valid worker depths begin
at4 and Main's maximum is at least6; the worker's generic zero-depth branch is
not a scored root invocation. Recursive leaf invocations occur below allocated
depth-one parents and preserve their allocated object storage.

The final scored `Node_box_check_W` reads both child fields, calls itself for
one child, follows the other with a tail-loop, and accumulates one per Node.
This positively demonstrates traversal of the constructed graph. Recursive
child identity is supported by the separate allocator calls and pointer stores,
not by searching for an allocator name or the absence of global variables.

The actual runtime decode is
`revisions/runtime-target-continuation-v1/jobs/remaining-scored-evidence-v1/runtime.ll`,
digest `581aea4adb0ef1396e2ca90f744953a74233b4748b39cc3b475c1ec4362f30ce`.
Its `pony_alloc_small` at line1530 follows the current actor's heap and calls
the noalias `ponyint_heap_alloc_small` at line13727. That body selects and
clears a free slot in its bitmap, unlinks exhausted chunks, or obtains a new
pool chunk; it does not return a fixed Node. The same positive bitmap clearing
and chunk/pool path is present in final scored ARM (`_ponyint_heap_alloc_small`
compact `0x100005a1c`, optimized `0x100005a78`). Reuse of released allocator
storage across sequential graphs is allowed; sharing live graph objects is not.

## Six retained actual traces

The result retains exactly the original six source-bound trace records, with
their distinct observer source/binary identities. The source-only capture
worker `revisions/scored-allocation-recovery-v1/capture_existing_v1.py` binds
each to the original qualification and manifest source SHA, complete output
match, native owner/terminal pins and raw trace SHA, and retains all original
failures. Its existing actual pin closure was consumed by the normal evidence
continuation and root's 10,198-pin closed review. No trace was rerun here.

| Pair | Retained actual evidence |
| --- | --- |
| Pi compact/optimized | n97; 416 complete candidate states and 832 separate numerator divisions; 97 output digits. |
| Nucleotide compact/optimized | n1000; 70,108 exact width-windows; all seven complete maps and matching parent maps; entry counts4,16,64,256,3705,10010,10004; 118,240 non-thread records per profile. |
| Trees compact/optimized | n6; 4,398 observer Nodes/checks, 2,240 leaves; distinct 127-Node long graph;64 depth4 and16 depth6 churn roots; exact worker done/join pairs2 compact and16 optimized. |

The tree identity validator `revisions/work-full-v5/check_traces.py` was fully
read along with its independent count checker and the retained compact observer
source. It requires every recursive node identity to be unique/live within a
graph, full `(2**(depth+1))-1` cardinality, two zero child sentinels only at leaf
depth, disjoint concurrent graphs, every node checked once, long graph alive
at each done/join, and the same long identity after all joins. It permits reuse
only after an identity is finalized. Original missing-record and excess-thread
mutation rejections remain recorded.

Raw retained trace lifecycle witnesses were read directly. Compact has
`ROOT_LONG_NEW` line1023 identity5125914624, done/join depth4 at9473/9475,
done/join depth6 at17670/17672, final `ROOT_LONG_CHECK` at17801 with the same
identity, then its `FINAL` at17802. Optimized has `ROOT_LONG_NEW` line1023
identity4597432320, eight done/join pairs per depth, final depth6 join17728,
`ROOT_LONG_CHECK`17857 with that identity, then `FINAL`17858. These are actual
observer lifecycle records, never asserted identities for the scored binary.

The observers add identity/finalizer/trace state, which can change escape and
allocation optimization. Their count4,398 must not be reported as the scored
binary's heap allocation count. Scored storage conclusions instead use the
original ARM above. The four retained Pi/map traces keep their earlier actual
proofs; this review adds no new scored machine-code proof for those programs.

## Minimal next step

After independent peer/root reading of this exact note, root can capture its
digest under common and author a small additive metadata seal. That seal should
consume the actual result/body/durable/root-review digests above, every actual
recorded pin and original six records, and the accepted review digest. It should
emit a new annotated allocation receipt with both exact profile identities,
the four proposed fields, the reviewed symbols/leaf control flow, explicit
temporary-root stack/register storage, and actual requested-versus-runtime
targets. It must never edit or mark the failed top-level work-v5 report passed.

The original `full_prerequisite.py` additionally expects a complete work
receipt with status `native-work-v2-passed-awaiting-scored-binary-allocation-review`
and the exact six qualified attempts. A new metadata-only projection can
establish that compatible receipt by reusing the six completed original actual
attempts and their closures, preserving four-retained/two-new historical
provenance and the failed report. Reexecuting successful traces, allocation
captures, fixtures or version/image queries is unnecessary.

Only after those new metadata receipts are physically closed and independently
accepted may the original full-performance prerequisites be satisfied. The
full20 program timing suite remains pending, with zero new timings or chart
admission from this review. Requested macOS14.4 versus actual runtime/main
macOS26 evidence stays explicit; no minimum-OS deployment compatibility claim
is made.
