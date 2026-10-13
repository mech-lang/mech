# Additive Crystal LOC overlay candidate

This is an unexecuted source candidate. The previous `seal_overlay_v1.py` draft and LOC18 source proposals remain intact and are not admitted. V2 consumes only the separately queued `crystal-brace-repair-v2/jobs/fixtures-replay-v1/source-proposals.json`, with its actual digest supplied by the reviewer. It does not normalize sources, alter timings, or admit a parser result on its own.

Before an overlay can be sealed, the corrected replay must retain the immutable LOC18 prerequisite and pass the exact 36 discovered/started fixture identities, with zero failures, errors, skips, expected failures, or unexpected successes. Its twenty source comparisons must match the same exact source hashes, leave nineteen counts unchanged, and remove only the optimized Fannkuch tuple's header and closing callback events at offsets 1310–1311 and 1328–1329. The corrected count is established by that actual replay, rather than an estimated count inserted in a publication row.

The separate semantic review JSON must have `status: passed-exact-source-semantic-LOC-review`, `language: crystal`, a nonempty `review_identity`, the corrected `replay_sha256`, `complete_authored_source_bundles: true`, and `unresolved: []`. Its exact twenty-profile mapping supplies each profile's `status: passed`, `review_identity`, `source_sha256`, `counted_source`, `physical_loc`, `canonical_loc`, `unresolved: []`, `ownership_disposition`, and `compiled_input_roles: [application]`. The reviewer must assess each source's actual expansions; the old all-source brace review is useful evidence but does not automatically certify the new replay.

The frozen measured source and binary identities come from the approved full-v1 index. Independently retained compiler receipts establish each authored input inventory: fifteen actual v2 build commands are matched to their preserved reuse receipt, and five fresh v6 build commands are matched to their preparation records. Each command names exactly the counted application source and actual output binary. The two Pi mains require the ordinary `big` standard library; local requires, macro declarations, or local foreign support invalidate this reviewed one-application inventory. Vendor toolchain and runtime inputs remain pinned separately.

All twenty source, review, compiler, fixture, and count rows are validated before any overlay output is created. The new exclusive revision then writes separately pinned compiler-input evidence and the importer-compatible overlay: per-profile physical LOC and expansions, per-file physical/canonical LOC, complete authored bundles, independent `compiler_bound_input_inventory`, and the actual grammar/replay/semantic receipts. Original producer LOC stays null. The importer must still validate the full overlay cohort before publication; this seal changes no Site or chart.

After actual corrected replay and semantic review, the proposed launch shape is:

```text
python3 benchmarks/clbg/crystal-comparison/20261012/source-counts/seal_overlay_v2.py --replay-file benchmarks/clbg/loc-normalization/20261012/crystal-brace-repair-v2/jobs/fixtures-replay-v1/source-proposals.json --replay-sha256 ACTUAL_REVIEWED_REPLAY_SHA256 --review-file benchmarks/clbg/crystal-comparison/20261012/source-counts/ACTUAL_SEMANTIC_REVIEW.json --review-sha256 ACTUAL_REVIEWED_SEMANTIC_SHA256 --revision corrected-qualified-overlay-v2
```

The sealer holds the common timing mutex for every hash and evidence write. No launch is authorized by this document alone.
