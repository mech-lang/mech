# Round 06: consuming scalar indexed stores

Starting compiler: `d9c2b9a3c62a7e29177918bd045a81023d9d8a01`.
Starting measured executable: `8ad35661bca41e0a3ab1c79bbe006c94b0474f8782d222b27050e5d4835ec33c`.
The eight-byte complete case is 1631.52x Roc against a 0.000285 s reference.

The real compressor operation is `core/assign/indexed-axis`, reached by
huff-set8/32/64, bt-descend-left/right, tree hash updates, bt-clear/attach and
bt-put-entry. The pinned algorithm repeatedly updates U16/U32 arrays. The
maintained packed constructor copies the entire buffer and publication compares
snapshots. Eight U32 stores still request 49000016 bytes, including eight cache
payloads. Current consuming-buffer facts cover sequence leaves, not indexed
replacement. This is a concrete compiler/runtime path, not an application rewrite.

Implement the narrow plain U8/U16/U32/U64 scalar replacement through existing
private selected-factory evidence, native word operands, prepared-call routes,
canonical owners and same-account exclusive transfer. Validate exact kind,
current shape and one U64 linear index, including column-major selection over
row-major storage. Derive changed from the selected old/new words. An unchanged
write must not copy or scan the whole buffer. Admit honest resources before
mutation; keep shared aliases immutable and use the existing publication/failure
boundary. No new intrinsic, executor, public ownership authority or source-name
recognition is permitted. Shared JIT/Object preparation and independent image
restoration must derive the same strategy from actual selected factories.

Require original/decoded widths, rectangular/current shapes, high bits, no-op,
empty/invalid selectors, exact/one-short resource admission, previous publication,
aliases, cleanup and retry. Preserve existing expectations and allowances.
Measure 1/8 stores on 288 and real 262144-U16 / 1500513-U32 lengths. Private chains
must avoid repeated payload copies; an external alias permits the initial
necessary copy, then private reuse. Prove the actual recursive tree/cache
carrier pattern benefits. Extend loop/field ownership only if a reached witness
shows the existing proof or retirement is insufficient; retain escape/capture,
external alias, fresh-definition and backedge checks.

Keep all 25 compressor components, algorithm, tables, source allocations, limits,
buffers, six initializers, artifact and evaluate_into timing boundary unchanged.
Use the same selected build and allocator, fresh 20000-observation reference,
excluded warmup/extra, three primary eight-byte calls, exact raw bytes and
independent inflation, capacity/alias/cleanup checks, and a separate two-second
profile. New physical ownership accounting can reduce work; report the exact
delta rather than preserving obsolete clone charges or increasing allowances.
Retain protected original/decoded scalar allocation and 1.05 timing gates and
zero per-visit loop allocation slope. Archive every observation and failure,
commit after timing, publish to draft #973, request review and cancel CI. No
larger timing or cancelled long campaign runs. Application rewrites will have
separate source revisions and measured cohorts.
