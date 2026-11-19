# Historical literal reuse: source, work and runtime study

**The selected recipe remains 63,692 B with 81/81 Roc and 62/81 libdeflate in both complete orders.** This study revisits the older literal-read-once gain using generic compiler proofs on unchanged Mech source. Its smaller 63,484 B experiment has one near focused C loss and is being repaired; it is not selected and has no inherited full-suite win count.

The [19-mechanism history review](../../history-and-profiles-20261119/README.md) already led to retained spill pairing and unsigned-guard/layout combinations. Literal reuse is the next bounded retest. Current exact-worker samples put 72.8% of 1 MiB encoder samples in matching and 54.0% of natural 4 KiB samples in scoring. The former exposes a shared path across many larger losses; the latter remains the closest next-win check. Sample fractions are not elapsed phase timers or forecasts. [Current profiles and build baseline](../../sealed-base-phase-fusion/README.md).

## Two real reads, then a backend tradeoff

The original collector reads the same immutable input byte for observation sampling and the cache header. On 1 MiB, those sites execute 1,048,545 and 1,044,385 times. Source-pinned CLIF identifies the same current index through four copy-phi nodes with one external seed. The compiler reuses the existing strong defaulted-read matcher: exact pointer/length packet words, unsigned bound, physical width, wrapping address and zero default. Only the exact private fixed-view loan supplies immutable authority. Writable/appending lanes and cloned proofs are excluded; unknown inputs retain their original safety semantics.

A bounded all-incoming copy-phi proof groups equivalent current indices at a common dominator. Actual updates, unavailable operands, unknown seeds, closed seedless cycles and exhausted budgets refuse sharing. The caller retains read-only owners, writable backings pass the existing unique-owner gate, and borrowed-copy analysis excludes phase boundaries. Original source, canonical bytecode, error ordering, resource checks and scalar native emission remain unchanged.

| Source-emitted experiment | Whole encoder text | Matcher body | Matcher frame |
| --- | ---: | ---: | ---: |
| Selected reference |63,692 B|6,092 B|384 B|
| Existing safe-address fold, all 8 reads |63,708 B|6,100 B|400 B|
| Immutable flags, before explicit reuse |63,724 B|6,120 B|400 B|
| Current-index reuse, backend may rematerialize |**63,484 B**|5,880 B|384 B|
| Current-index reuse, retain load placement |63,692 B|6,092 B|400 B|

The last two rows have freshly reconstructed matcher layouts. The other two retained phase profiles have exact ordered-graph equality proofs. Normal and allocation-checked builds reproduce identical objects, full metadata and bytecode. The disabled pass reproduces the selected native artifacts exactly.

Cranelift 0.131.3 treats a narrow `Load` as pure when all of `readonly`, `notrap`, `can_move` are set. One shared expression is then rematerialized at both distant uses. Removing `can_move` keeps the manually proved load placement, but its longer live range adds spills. The diagnostic CLIF is not a shipping artifact; enabling it also retains additional general bodies. No whole-debug-object identity or elapsed-time claim is made.

## Actual collector work after fresh layouts

The counters execute original instructions in diagnostic clones, verify outputs/inflation/aliases, and are never timed.

|1 MiB random-prefix trace|Selected|Rematerialized reuse|Retained load placement|
| --- | ---: | ---: | ---: |
|Original instructions|233,154,478|247,693,058|242,615,383|
|Branch instructions|41,017,489|36,790,982|36,790,982|
|Byte-load instructions|4,002,154|4,002,154|**2,957,769**|
|Stack-load instructions|22,267,325|22,127,094|23,296,565|
|Stack-store instructions|8,653,030|**6,534,095**|9,701,645|

Thus the 208 B saving does not remove dynamic byte loads. Pinning the load removes 1,044,385 byte loads but gives back the size gain and adds stack traffic. Both alternatives add overall instructions. The natural 4 KiB trace similarly favors rematerialization for spills:106,294 loads/30,598 stores versus the selected 131,115/38,744. These observations justify measuring both alternatives; neither proves a runtime gain.

The source-emitted zero data changes Mach-O section order. The diagnostic counter adapter canonicalizes section coordinates while preserving every text/data byte, symbol-relative offset and relocation target. Its linked executable is byte-identical to the corresponding original worker. Original objects and workers are used for runtime timing. The normalization and worker bindings are archived.

## Measured focused orders and next repair

Both changed workers pass 162 exact output/inflation/alias observations, six checked failures,12 successful recoveries and the 4,848 B peak active stack check, below the 5,280 B ceiling. Eight optimized native tests pass across defaulted loads and private-phase contracts, including original/decoded codec proofs, null/default/wrapping inputs, changed loop indices and divergent paths. The initial broad test build failed because its include still named the removed synthetic fixture; it was corrected to use the unchanged actual codec. The earlier failed fixture and logs remain archived.

- Retained placement preserves all 15 focused Roc wins and all 13 contemporaneous focused C wins in both orders. It has no whole-text saving. Complete runtime qualification and matched complete builds are not claimed.
- The 208 B-smaller alternative preserves all 17 focused Roc wins. It keeps 12/13 contemporaneous C wins forward and 13/13 reverse. `320-overlap` is **1.001763× C** forward and **0.994634× C** reverse. The single near loss prevents selection. Large-random 1 MiB and 256 KiB also slow approximately 2.7% versus their paired control in the first focused order. Every raw order and bootstrap result remains included.

**Repair the scope before giving up the gain.** The new pass currently also applies the ordinary safe-address fold to six mutable-buffer reads; only two reads have immutable certificates. Those other transformations add address/select/GOT work on the main matcher path. The next source variant leaves all uncertified reads' original branches in place and shares only certified immutable reads. Its executable tests pass; compiler emission, new counters/layout, actual-worker checks and both full benchmark orders remain pending. The standalone ordinary safe-address pass stays independent.

The complete source-through-final-link baseline remains 2.205340 s from the prior balanced cohort. This study's compiler bootstrap/preflights occurred under diagnostic load and are not replacement build timings. Fresh matched complete builds are required before selecting any changed recipe. The 1× libdeflate text/build goal and the larger early-region construction work remain unfinished.

## Archive and checks

Run `python 3 verify.py` to verify source/artifact bindings, tested workers, counters and paired focused observations. `as-run/` contains the exact acquisition scripts and original machine paths; source snapshots are compressed to keep historical Rust out of architecture scans. These are provenance records, not a portable replacement compiler build. This package preserves initial/adverse/repaired evidence without promoting an unqualified worker.
