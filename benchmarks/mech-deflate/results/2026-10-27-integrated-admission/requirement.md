# Integrated native program admission

The final level-12 block selector requires 21,289 native blocks and 31,848,344 bytes of current-visit analysis storage (11,292 values, 11,669 analysis cells). Its maintained source witness is `hosts/gpu/tests/support/canonical_program_selection.rs` and its pinned package revision is `671077457ff7ef3a3d44caf715a3fc595addf379`.

The reviewed parent is `6132d35b1cfa74a5a37d79f9764619d4f7de90cd`. Its shared 16 MiB analysis guard rejects this computation before native execution. The compiler patch keeps a finite, checked 64 MiB guard shared by normal lowering and linked revalidation. It retains the default 4,096-block native compilation profile. Only the new integration qualification requests 32,768 blocks.

After the proof passes, the map write-use pass rejects the same map-free computation at its map-specific 16,384-block graph bound. The focused `large_mapless_program_has_no_map_write_uses` test reproduces this denial with a small valid metadata graph before the fix. The pass now validates the plan and semantic actions, then returns no write-use facts if there is no selected map insertion. Programs containing an insertion retain the existing graph bound and alias analysis. No storage or mutation authority is granted by this early return.

The new selector needs 2,505,256 bytes of execution frame transport. Its new qualification explicitly admits 4 MiB; the first 2 MiB attempt is preserved as a pre-execution resource rejection. Existing qualification limits and expected values are unchanged.

## Execution evidence

`selection-real-execution` passes one Rust test across three pinned cases in both original and decoded artifact forms. All nodes, models, frequencies, codewords, sequence fields and selection flags agree with the reference. Real buffer sizes are retained: 1,500,513 U32 cache words and a 305,257-entry U64 node capacity.

Original-artifact case measurements in the unoptimized Rust test profile:

| Case | Completed positions | Seconds | Allocation requests | Cumulative requested bytes |
|---|---:|---:|---:|---:|
| eight-literals | 8 | 6.048302125 | 2,897,229 | 234,285,208 |
| eight-overlap | 8 | 5.965383750 | 2,860,685 | 233,101,371 |
| 320-overlap | 320 | 10.245481208 | 4,382,449 | 1,850,388,876 |

Requested bytes are cumulative allocation traffic, not peak live memory. These are component qualification measurements, not complete compressor performance. Match collection is qualified separately and is not substituted by a foreign compressor here; the selector witness supplies the previously qualified cache records so its own decision can be checked independently.

## Validation

The attached JSON receipts preserve exact commands, feature configurations, source HEAD, worktree differences, elapsed time and outcomes; compressed logs preserve detailed failures and results. `validated-sources.json` identifies the exact compiler file contents measured.

- `admitted-plan-proof-final`: 13 current-visit proof tests pass, including malformed metadata, stale values, finite admission rejection and the large map-free reproducer.
- `selection-real-execution`: 1 integration test passes, three cases in original/decoded forms.
- `admitted-plan-map-regressions`: all 11 existing native map-entry, write-use and owner-region tests pass under `mech-stdlib --no-default-features --features full_compiler,native-artifact`. They retain all seven widths, old aliases, direct/aggregate reads, failure atomicity and retry checks. The cold feature build takes 8m34s; the tests themselves take 13.09 seconds.
- `admitted-plan-static-final`: resident routing, source catalog, native host catalog, build-analysis architecture, CI inventory, warnings-preserving formatting and diff checks all pass (16.25 seconds).

The complete compressor source is written but has not executed successfully. Its initial default test-thread run overflows the Rust stack before native evaluation. The identical binary with `RUST_MIN_STACK=33554432` completes source checking and reports `InvalidControl { node: NodeId(3), reason: "control graph admission limit" }` from canonical artifact finalization after 451.8 seconds. The shared control-count validator is unchanged from the parent; this is a separate inherited canonical-artifact admission bound, not a selector runtime failure. Its precise count still needs diagnosis. A stack sample records a 6.8 GiB peak compiler footprint. No runtime-memory or complete compressor performance claim follows from those compiler measurements. These remaining full-program findings are outside the selector acceptance result and are not claimed as fixed by this patch.
