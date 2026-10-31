# Round16 baseline: ordinary record projection

This is an investigation checkpoint, not a new production optimization or protected speedup result. Production/source head `ebc559b2bdd30c0d1417e765c7ea87fab8d5491a` is clean-reviewed in [receipt6489919438](https://github.com/mech-lang/mech/pull/973#issuecomment-6489919438), with no unresolved inline threads. Its measured compiler is `5cc14c28ccf786bceeb3ea0db7448530caed537c`. Stack parent remains `eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b`. Branch/worktree: `codex/level12-compression-consolidated`, `/private/tmp/mech-deflate-linear-stack`.

Protected baseline remains **142.091837× Roc=1×**,41.064541ms for the complete eight-byte case. No new protected timing is claimed here. Pinned caller source/bytecode, all25 components, tables/limits/real buffers/allocation sites/timing boundaries and primary allocator remain unchanged. Diagnostics below do not change the production policy or compressor. The full1× goal remains active.

## Evidence that changes the next action

Reanalysis of the retained1354-sample primary profile isolates561 exclusive weights under other invocation helpers. Of those,425 enter `CallCacheLane::invoke`;424 enter `invoke_current_cached`. Within that exact ancestry422 take `invoke_validated_current` (the uncached fallback), with192 under `bind_values` and225 under ordinary maintained invocation, plus5 retirement weights. These are disjoint subdivisions, not additive inclusive sums. Cached ordinary calls represent425/1354≈31.4% of sampled ancestry; this is not a promise of removable elapsed time.

The cache's initial headroom check computes `max_temporary_bytes.checked_add(native_scope_metadata_bytes())`. NoLimits supplies u64::MAX, so the sum is unrepresentable and the optional binding cache declines. `cache-witness.rs` executes the real public prepared-call API for a source-derived repeated String-length call with three fresh values. NoLimits retains0 metadata bytes on every call; a diagnostic finite-temporary control retains3761808 metadata bytes. Results3,2,4 and complete cleanup pass on both sides. Work per NoLimits call is7981463; the finite control shows the normal reuse path. That control changes policy only in the diagnostic, provides no primary speedup and is not an acceptable substitute for the unchanged benchmark policy. The conservative guard also protects tight global-memory headroom; bypassing it or reducing primary limits is not the proposed fix.

The next operation was chosen by actual execution, not static candidate counts. A temporary test-build counter ran **one complete unchanged compiled eight-byte compression**, obtaining100 generic calls across60 sites. Exact ten-byte output, work303735923752,213929 ordinary visits and final zero memory/nodes agree. Counts:

| Generic operation | Calls |
| --- | ---: |
| RecordFieldAccess |53|
| CompositePack(3) |38 (33 with fully fixed geometry) |
| CompositePack(6) |4|
| CompositePack(2) |2|
| CompositePack(4) |1|
| CompositePack(7) |1|
| MatrixHorizontal(1) |1|

A second diagnostic adds per-call clocks to the same test. Its100-call counts agree exactly. **Record-field reads consume15.370920ms of20.547795ms inside generic calls**; all38 three-field constructors consume2.077083ms (the33 fully fixed ones account for1.524542ms). These are System-allocator/test-build observations with instrumentation, not primary mimalloc timings or exact primary time shares. GPU code is opt1 and Engine dependencies are the exact opt3 release libraries; output/work/visit evidence agrees. No cross-allocator percentages are multiplied into a claimed speedup.

Reached selectors include litlen-lens(4),offset-lens(4),freqs-litlen(3),freqs-offset(3),literal(8),length(8),offset-slot(8),nodes(2),costs(4),saved(4),selected(1),tallied(1),seqs(1),litlen-codewords(1),offset-codewords(1). The names/counts derive from the same selected site IDs and constant Id selectors. The32 most repeated fixed record constructions are the real optimal-matches results; they are a smaller target than managed record projection in this case.

## Exact source requirement and existing owners

`hosts/gpu/tests/fixtures/native/deflate-compress-controller.mec:17–21` reads planned cost/state records and selects buffers for `optimal-plan-block` and `block-flush`. The unchanged Huffman source at243/247/248 selects generated/sorted lens and words. Optimal planning repeatedly selects `.literal`, `.length`, `.offset-slot`, `.costs`, `.saved`, `.selected`, `.tallied` and `.nodes`. These reads must return current matrices/nested records with their current shapes; scalar-only field transport is already specialized.

The maintained path is Native `LibraryCallRoute::Canonical` → `CallCacheLane::invoke` → `NativePreparedLibraryCall::invoke_current_cached` → uncached `bind_values`/`bind_current` → ordinary `SnapshotAccessPlan`/`snapshot_access_from_source`. It rebuilds the complete bound-call metadata and resolves the parent component geometry. Existing `numeric::aggregate_access::{closed_source,current_shape,selected_component}` and the Core constructors remain authoritative.

Importantly, ordinary aggregate access already keeps a selected shallow unsigned matrix packed and uses `packed_word_matrix::copy` after admission (`numeric/mod.rs` around17920/18060). A proposal to add that packed-copy optimization again would be duplicate work. Whole-enum tuple projections are another conservative miss, but this actual input makes0 generic EnumPack calls. Static inventories contain156 repeated canonical candidates, including24 EnumPack sites; they are not equally hot. Flat scalar-record eligibility is also narrower than the measured record-buffer reads.

## Next bounded implementation and acceptance

Target **prepared metadata for ordinary record-field projection**, particularly scalar/packed-matrix fields and their current parent→child shape mapping. Reuse the existing selected factory, aggregate-access mechanics, footprint/accounting owner and typed constructors. Prepare immutable schema/key/field/shape facts once and evaluate current source/selector/shape/account/output facts each call. Do not introduce a program executor, fabricated turn plan, cached successful admission/current value, source-name selector, ownership bypass or changed benchmark policy. Preserve actual packed copies and original allocation behavior for this slice; borrowing/subvalue ownership is a later separate change.

Before widening code, establish a small source-derived external record-input projection witness (so private-frame unboxing does not hide the ordinary path), then repeat records with changing owners/extents and competing field/type/resource failures. Acceptance covers all four unsigned widths, zero/rectangular axes and independent parameter arenas; missing/wrong/ambiguous selectors; source and prior-output aliases; exact/one-short compute/copy/temporary/node/output limits; first error/spent prefix; original/decoded agreement; unchanged publication, cleanup and recovery. Keep the existing scalar/array/private-frame controls and allowances. Use the real reached controller/Huffman operations as the permanent qualification case. Run and commit the unchanged paired eight-byte primary/profile immediately after a correct candidate; retain unsuccessful results too.

## Diagnostic mechanics and failures

The host rejected an LLDB process launch: non-interactive session could not obtain debug permission. Its symbol probe also resolved0 locations. No debugged process/compression ran; logs are retained. This was a host/tool limitation, not automatic approval rejection or a compiler failure.

Exact counts instead use a cfg(test)-only entry/Drop hook in the actual GPU library test root. Five unrelated unit modules needing the optional mech-runtime dev graph were temporarily excluded; their names, original hashes and reasons are in `trace-test-exclusions.json`. All production execution modules remained included. The count and timed variants each ran1 test with399 filtered, preserving exact result/work/visits/cleanup. These are two executions of one diagnostic test, not two unique product regressions. The original manual test-root build exposed missing optional dev dependencies and one dependent excluded module; failed build receipts are retained. An initial Python receipt-script brace typo and an incorrect AOT cfg pattern were corrected before the successful build. No production failure was repaired or hidden.

All six temporary source edits were restored byte-for-byte from verified backups, and a clean worktree was confirmed before creating this packet. The full instrumentation patches and test source are retained for reproduction; no tracing hook, test exclusion or policy change is published in production source. New primary/scalar/large-input campaigns and a full qualification rerun were unnecessary during this evidence-only checkpoint; the332-test round15 packet remains prior evidence for unchanged production code.

Inherited unresolved scalar observation remains round10 decoded-U64 ratio1.0526036>1.05. Review/CI policy remains draft, fresh review requests and cancelled CI at the user's direction. This checkpoint leaves the full parity goal active.
