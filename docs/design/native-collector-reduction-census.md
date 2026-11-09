# R239: measured collector writes and short reduction completion

This is an aggregate diagnostic checkpoint, not a new performance optimization or completion of the whole brief. R234 is the main large-input control; retained R237 stays available separately and R236 tradeoffs remain historical evidence. No new primary timing or instruction gain is claimed. Ordinary R215 and standalone omitted-contract scope are unchanged.

## What the counts establish

Random32 executes221,976 root indexed replacements:78,218 equal and143,758 changing, with only5 physical backing clones. All149,801 U16 tree replacements arrive with a unique consumed owner;72,504 change a value and77,297 do not. Its51,506 empty-head child clears all leave the value unchanged. The leading opportunity is to retain private-view knowledge across operations, not to predict large new allocation savings. Unconditionally storing equal clears would add traffic and must be measured explicitly.

Natural32 completes383,735 optimized ordered reductions and then enters ordinary terminal dispatch. Its1,130,575 candidate lengths fall into235,600 single-length intervals,47,285 two-length,26,085 three-length,63,965 four-to-fifteen and10,800 sixteen-plus. One-to-three lengths account for80.5165% of reductions. Direct completion therefore has a concrete high-frequency witness; no elapsed saving is established yet.

Independent biased-U16/rebased and absolute-position input censuses agree at every active position on all25 fixtures. They reproduce the review exactly: random32 has25,753 empty/expired heads out of32,765 (78.5991%), distribution16 has14,260/16,380, natural32 has5,700/32,765 and held-out random65 has45,681/65,533. Logical replacement attempts are not physical stores; native counters distinguish them.

| Input | Root indexed attempts | Equal | Changing | Actual clones | Optimized reductions replaying terminal dispatch | Candidate lengths | One-to-three interval share |
|---|---:|---:|---:|---:|---:|---:|---:|
| empty | 0 | 0 | 0 | 0 | 0 | 0 | — |
| one-stored | 0 | 0 | 0 | 0 | 0 | 0 | — |
| seven-stored-prefix | 0 | 0 | 0 | 0 | 0 | 0 | — |
| eight-literals | 331 | 20 | 311 | 3 | 0 | 0 | — |
| eight-overlap | 156 | 10 | 146 | 3 | 8 | 16 | 100.000% |
| 320-overlap | 1,463 | 7 | 1,456 | 3 | 16 | 1,256 | 50.000% |
| eight-literals-prefix | 331 | 20 | 311 | 3 | 0 | 0 | — |
| 128-biased-dynamic | 1,122 | 357 | 765 | 3 | 57 | 57 | 100.000% |
| 32-high-entropy-stored | 1,635 | 344 | 1,291 | 3 | 0 | 0 | — |
| 16384-distribution-change | 120,367 | 40,470 | 79,897 | 13 | 25,228 | 25,363 | 100.000% |
| 33027-periodic-window | 133,344 | 152 | 133,192 | 3 | 512 | 65,460 | 50.000% |
| random-64 | 2,407 | 309 | 2,098 | 3 | 0 | 0 | — |
| random-256 | 6,275 | 1,057 | 5,218 | 3 | 0 | 0 | — |
| random-1024 | 11,828 | 3,100 | 8,728 | 3 | 0 | 0 | — |
| repeated-1024 | 4,279 | 7 | 4,272 | 3 | 32 | 4,056 | 50.000% |
| random-32769 | 221,976 | 78,218 | 143,758 | 5 | 63 | 63 | 100.000% |
| repeated-32769 | 131,586 | 12 | 131,574 | 3 | 508 | 65,024 | 50.000% |
| reymont-prefix-1024 | 9,128 | 1,757 | 7,371 | 6 | 4,348 | 22,836 | 70.009% |
| reymont-prefix-16384 | 103,134 | 11,163 | 91,971 | 20 | 189,825 | 611,218 | 79.214% |
| reymont-prefix-32769 | 191,846 | 15,123 | 176,723 | 18 | 383,735 | 1,130,575 | 80.517% |
| heldout-window-32767 | 191,839 | 15,123 | 176,716 | 18 | 383,730 | 1,130,565 | 80.516% |
| heldout-window-32768 | 191,844 | 15,122 | 176,722 | 18 | 383,730 | 1,130,570 | 80.516% |
| heldout-slides-65537 | 373,319 | 21,703 | 351,616 | 27 | 748,780 | 2,257,021 | 79.865% |
| heldout-random-65537 | 424,539 | 130,043 | 294,496 | 4 | 231 | 231 | 100.000% |
| heldout-mixed-131073 | 751,839 | 123,361 | 628,478 | 28 | 388,178 | 1,331,149 | 80.444% |

## Instrumentation and qualification

A separate opt-in compiler image records aggregate counts, never iteration traces. It observes equal/changing writes, current incoming and post-retirement owner counts, actual backing changes, root source visits, numeric child selection and reduction entry/completion/span bins. No observer changes a reference, grants mutation permission or retains an owner. The counter distinguishes borrowed, unpublished, unique and shared states; raw allocation starts unpublished with refcount0.

The nine-file diagnostic patch changes no compressor source or algorithm. Its observer is imported only for the explicit observed image. The normal no-general-call numeric invariant still rejects every other call; this diagnostic alone recognizes its exact observer FuncRef and makes no finite-accounting claim. A direct phase verifier runs before cleanup. With instrumentation off, the entire emitted Object is byte-identical to R234, including all numeric/map/root code. Full metadata identity is not claimed: only pre-cleanup/dead-instruction diagnostic counts differ, while semantic and post-cleanup geometry agree.

Fifty full compression calls complete, two per case, with byte-identical repeated aggregates, exact compressed bytes, independent inflate, held input/output aliases, original capacities and zero final ownership. All25 allocation/requested-byte/initialized-copy/zero/peak-payload fields also agree with the retained R234 receipt, proving these counters did not change the measured buffer mechanism. The observed image has134 numeric routines with exactly the same header selection as R234. Its added observer code/stack are diagnostic costs, not optimization results.

Five focused tests pass with warnings-as-errors: real cache/tree/path allocation behavior, shared alias copying, clear retaining capacity, rejected allocation/recovery, and aggregate bins/current-owner states/reset. The exact six-feature optimized compiler slice is rebuilt; math/Engine/ordinary runtime/dependency libraries are frozen. No fresh703-test suite, hardware-counter cohort or new performance profile is claimed for this diagnostic-only checkpoint. R238 contains the completed broad qualification and profile.

Instrumentation setup failures are preserved and corrected: a wrong wrapper path, missing Debug formatting, observer rejection by the normal call invariant, and one observer SSA dominance error. A unit expectation incorrectly assumed raw allocations already had a live reference; the repaired test now checks unpublished0→unique1→shared2 without observer mutation. The final verifier, focused tests and50 calls pass. No unresolved introduced program failure remains; inherited experimental module-layout debt and omitted ordinary contracts remain separately assigned.

## Next bounded experiment and unfinished work

The proposed first private region covers a single-entry linear chain of consumed non-growing indexed replacements. A nonallocating current sole-owner guard selects it; shared or unproved paths keep the exact original fallback. Reuse current indexing, packed stores, arithmetic, source failures and owner settlement. Empty prechange-retirement lists are required initially, so removing equality checks cannot silently change equality-dependent cleanup. No global permission cache or long-lived per-array flag vector is proposed.

The real consecutive head stores1038→1040 execute32,765 times each on random32, both unique with no clones and no prechange retirements. A clean-parent source witness is compiled in original/decoded forms before functionality is added.3,000 native checks agree with an independent scalar oracle:240 successes and1,260 ordered rejections per form, with live aliases, equal/repeated writes, bounds/overflow order, unchanged failed result ports and zero cleanup. The first fixture drafts violated existing exact branch-schema rules; matching row-vector arms repair the fixture without compiler semantic work. Invalid raw Boolean, stronger explicit retry and general geometry/refusal controls remain acceptance work.

Collector private-view continuity, direct reduction completion, whole-phase append reserve and comparison-result reuse are not yet implemented. The remaining census covers initial/vector mismatches and descent rereads, fully attributed zero-trip state transport, source pass/recompute attribution and hot stack-slot origins/dynamic uses. The useful write/span evidence is complete for the first two targets; do not report the entire brief complete.

For subsequent optimization rounds, predeclare all11 inputs of at least16 KiB as primary, natural/repetitive/random families separately, fixed confirmation and all25 unchanged1.05 regression gates. A tiny-case gain cannot compensate for a reproducible large-case regression. R234 is the main control, current retainedR237 is reported separately, and historical R236 remains explicit.

Retained standalone timings remain the R237 measurements: natural32 2.136610× Roc, random32 2.831896×, periodic1.724336×, repeated1.633949× and eight literals0.504507×. This census adds no new timing claim or denominator. The ordinary exact resource/admission/accounting/publication/recovery contract remains omitted in this standalone entry.

## Pins

- base: `f1a271c92fa7f1e6772f7ea5739110a3e2cf652d`
- main_compiler_control: `eab2b19251c9ac6a221b862f1354bb5ec45a4d34`
- retained_compiler: `f92415a31dbf0398ae641e66c419f435a3e3cb7c`
- ordinary_compiler: `42c95cfd79e0dc3773d03aeb9f553d32ff3922c5`
- source_sha256: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- bytecode_sha256: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- compiler_source_manifest_sha256: `ecea5d3628c53af8a4836b28cd0e18e1927ba12f2e2cce5c9d3df74bd11522a0`
- compiler_patch_sha256: `addf61111392d4e068464ef51c53988f8a3561b3fff9bc81bc504db7083bc628`
- diagnostic_worker_sha256: `59c1384b72f11d3d8dbd9421ef069e9956bd74d402f39df1a50698c240573891`
- diagnostic_object_sha256: `49f1da9318c636f547a29208a83f3f1e83f40eb910c2fd35aaf2e0ba5bd71b24`
- disabled_object_sha256: `1561d3a0d98b1db3e8176fb23d5725f543e9accf0b3b383d765c0d80855573fc`

The external packet contains the full instrumented source snapshot/patch, aggregate native observations, source-site attribution, independent input census, setup failures, current proof/acceptance plan and numerical replay. No native binaries, Objects, bytecode, full executable metadata/descriptors, raw traces or raw stack samples are included.

## Public evidence

[Complete census evidence](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r239-collector-census-ecb8c8ca0262.tar.gz) (`sha256:ecb8c8ca02623a31a48336652b3cc0598b2b96ee6d4824678582c8db2910d8c5`, 6380283 bytes). All 936 downloaded hashes, server digest and byte-identical numerical replay verify.
