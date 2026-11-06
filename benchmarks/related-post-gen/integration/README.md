# Current related-post integration targeting #973

The current compiler integrates #973 through **369714775615e4adb85d67467b9e7409a69f92b4** and repairs its inherited exhausted-transition reduction miscompile. Compiler **fc61145beddd6a9777e40e0469e224801bfe47c8** is **IR231 / work115 / ABI41 / image9**. [PR990](https://github.com/mech-lang/mech/pull/990) targets `codex/level12-compression-consolidated` (#973).

The regression first reproduces the unfixed result `(9,119,1)` instead of `(0,119,1)` with the reduction selected. The fix retains the active tag and resumes the original exhausted source transition after the candidate reduction, preserving arbitrary final payload updates. Prepayment still charges the exact original visits. The new tests cover three payload updates, scalar/vector thresholds and tails, zero/exhausted entries, ties, original/decoded artifacts, direct/restored execution, aliases, finite publication, cleanup and retry. The optimized reduction stays selected.

Current local qualification: **841 GPU passes with exactly the two inherited failures**, **128 scalar and five compression rows exactly equal to IR228 in every non-timing field**, 434 loader fixtures, 52 original/decoded bundle files, 48 finite denials, 12 finite small/full successes, 16 forged-frontier rejections, and two fresh genuine same-ABI/work-policy old-Object checks (IR228/230). Runtime-only full5000, scalar-only compilation and the unmodified generated application's success/error/denial checks pass. Earlier old-Object checks remain separately labeled. This is a draft; no green hosted-CI claim is made.

All four related-post complete native text sections and their **212 numeric functions are byte-identical to measured IR228**. The incoming affine-address optimization and this reduction correctness fix do not select the related query trees. **Fresh IR231 parent measurements are now available in the Boolean experiment screen below.** Earlier timing/resource/profile evidence remains separately pinned to IR228. No new native-kernel speedup is claimed for IR231. IR208 stays selected; Rust parity remains unmet.

See `ir231-review-status.json`, `ir231-local-archive-index.json` and `evidence/ir231-*` for the failed-before/passed-after evidence, complete source/build inventory and current qualification. Raw binaries and Objects are indexed locally, not remotely uploaded. The fresh-traversal IR229 probe is also retained as negative evidence: it identifies 464 query words but emits unchanged native text.

The supplied performance review examines historical IR223. Reinspection confirms direct producer-proven Boolean literal lowering is still missing in the current source emitter; the later backend Boolean cleanup is a separate pass. The direct-Boolean experiments are now completed and retained below. Next are ordered subset keys and mask reuse, immutable one-shot call/restore metadata, producer-derived ranges and native-tree funding. The fixture independently confirms 5000 ordered unique rows, 22538 tag IDs, 35228 triples and 15152 quartets. These are experiment targets, not demonstrated speedups.

## Fresh randomized Boolean experiments (unselected)

Source patches and complete measurements are in `evidence/boolean-experiments/`. These patches are preserved for review and combination with later compiler work; the active compiler in this branch remains qualified IR231. The harness now supports seeded randomized order while retaining alternating order by default. It saves all round orders before the first process. All ten existing timer checks and five new schedule checks pass.

The latest31-round screen has nine entries, keeping standard Rust as output oracle, all three leading matched Rust controls, the latest full-process Rust winner, selected IR208, current IR231 and both candidates. Every invocation computes all5000 posts. All279 outputs are exact. Its separate seven-round resource cohort retains70 exact outputs. The earlier eight-entry31-round screen and63 resource invocations are also retained, including every initial/adverse sample.

| Original source,4 workers | Continuous post-JSON | × fastest matched Rust | Fresh launch through exit | × fastest whole-process Rust |
|---|---:|---:|---:|---:|
| Current IR231 parent |2.961916 ms|1.811938×|38.391708 ms|1.551089×|
| Direct canonical Boolean branches, IR232 |2.962959 ms|1.812577×|37.269792 ms|1.505761×|
| Eight-bit condition, IR233 |2.954208 ms|1.807223×|37.479541 ms|1.514236×|

The fastest matched control is Rust U8/U32 at1.634667 ms; the fastest full-process control is Rust U32 at24.751458 ms. Those different references stay explicit. IR232 wins only14/31 processing pairs against IR231; IR233 wins17/31. Their whole-process results win21/31 and23/31 pairs respectively, but the cause is not isolated by these comparisons. Neither establishes a clear processing improvement over the current parent. IR231 itself wins only14/31 whole-process pairs against selected IR208; IR208 remains selected.

Post-JSON instructions are57,115,570 for IR231,57,672,075 for IR232 (+0.97%), and59,008,497 for IR233 (+3.31%), versus matched Rust19,226,397. Allocation requests stay3858 versus matched Rust58, requested bytes8,044,006 versus6,825,713, and peak live requested heap about13.62 MB versus7.66 MB. IR232 shrinks the query28448→28316 bytes but grows its stack528→544; IR233 restores528-byte stack while growing query code29712 bytes. Total generated bytes are908464 /911812 /928848. Smaller frames or fewer static instructions alone are not accepted as speedups.

IR232 passes843 GPU tests with exactly the same two inherited failures and all133 scalar/compression rows equal to IR231 in every non-timing field. Its same-compiler ablation preserves values, exact work/errors, aliases, publication and retry. Raw inputs and merges with an unknown producer keep their literal checks. One initial regression had an incorrect expectation that a mixed raw/proven merge should retain a canonical copy; that expectation is corrected, and the original failed receipt is retained. IR233 has targeted Boolean/finite/error/reduction/funding checks and31 linked-Object fixtures; full GPU/compression qualification was not repeated, and it is not an adoption candidate.

A fresh IR232 profile validates1000 exact invocations and4148 post-JSON records against the actual linked native bytes. The query remains the leading application-code sampled location. UTF-8 preparation, data-element access, allocation and snapshot work remain. Restoration retires roughly136million instructions; call/restore metadata and repeated source key work remain next targets. Samples are positions, not CPU-time shares, and no IR232 PC mapping is applied to changed IR233 code.

## Retained IR228 measurements and qualification

The current compiler merges #973 through **0bca6a54cdc2d69df71ca2763a6f36993448fe02** and the related-post compiler series. Compiler commit **29838b0e08b5ba66bbad5b7f7565fc354a4e9490** is **IR228 / work115 / ABI41 / image9**. [PR990](https://github.com/mech-lang/mech/pull/990) targets `codex/level12-compression-consolidated` (#973).

The new DEFLATE nonempty scalar-reduction optimization is integrated and its boundary cases pass. All four related-post complete native text sections, containing **212 numeric functions**, are **byte-identical to IR227**. This pass does not select the related query trees; its inclusion is not a new query-kernel speedup.

The latest15-round balanced comparison uses the same complete continuous post-JSON boundary and fresh launch-through-exit boundary:

| Mech variant | Post-JSON | × fastest matched Rust | Whole process | × fastest whole-process Rust |
|---|---:|---:|---:|---:|
| Original,4 native workers |2.948625 ms|1.801879×|42.281416 ms|1.637997×|
| Compact U8/U32,4 workers |3.020833 ms|1.846004×|41.481833 ms|1.607021×|
| Original,8 workers |3.147916 ms|1.923664×|37.962750 ms|1.470691×|
| Compact U8/U32,8 workers |3.235208 ms|1.977007×|40.692459 ms|1.576440×|

Fastest matched Rust is the single-threaded streaming control at **1.636417 ms**. Fastest whole-process Rust is `rust-preinitialized`, **25.812875 ms**: this parent timer includes all setup before the implementation's narrower internal timer. That narrower timer does not qualify as a matched post-JSON reference. All variants compute all5000 posts and produce exact standard-Rust output.

Four-worker post-JSON is effectively unchanged from same-cohort IR227 **2.947792 ms**. Its smaller multiple than the prior31-round result reflects the changed fastest Rust reference. **Whole-process four-worker time is adverse** versus IR22739.918583 ms and selected IR20839.827916 ms; it wins only3/15 whole-process pairs against each. Original-eight has faster whole-process results but slower post-JSON results. Those intervals must not be combined into one Mech result. The cause of the whole-process variation is not established by the post-JSON profile. The integration remains **unselected**; IR208 stays selected and both Rust parity targets remain unmet.

The separate resource cohort records **57,049,727 post-JSON instructions versus matched Rust19,654,219**, **3,858 allocation requests versus57**, requested bytes8,044,006 versus6,803,175, and peak live requested heap13,616,163 versus7,640,671. Per-stage restore/JSON counters, residency, file/text sizes and all raw resource samples are in the packet. Native program bytes remain908,464 with a528-byte query frame. Native instructions are only one part of the process; the profile also retains host snapshot, data-element and preparation work.

Current qualification passes **839 GPU tests**, with the same two inherited failures; all **128 scalar and5 compression rows match IR227 in every non-timing field**, including allocations. There are **434 loader fixtures**,52 byte-identical original/decoded bundle files,48 exact finite denials,12 finite small/full successes,16 forged-frontier rejections and **26 genuine old-Object checks**, including same-ABI IR227. Scalar-only compilation, runtime-only full5000 output and unmodified Managed success/error/denial checks pass. Unchanged Core/Engine sources and libraries retain the prior391/289 checks. The current cohorts contain **1,437 exact outputs**, and fresh profiles execute **2,000 exact invocations**.

See `evidence/ir228-screen-report.json`, compressed complete cohort/profile/size reports, current source/build inventory, `native-text-equivalence.json` and `latest-local-archive-index.json`. The current local archive verifies 2282 files and1,473 completed output copies. Raw archives are retained locally, not remotely uploaded. The IR227 control-summary correction and funding ablation below remain explicitly labeled historical evidence.

## Retained IR227 screen and audit

The production compiler builds with the pinned optimized dependency closure. Original and compact sources with four and eight workers each retain **53 native numeric functions** and pass **31 exact-output fixtures per variant (124 total)**. The original-four native code decreases from **930,772 to 908,464 bytes**. Seven additional unsigned guards are proved per variant. The measured comparisons below use this exact compiler.

The latest combined source passes **839 GPU tests**, with exactly the two inherited matrix-catalog and missing-stats-catalog failures, plus **289 Engine tests**, scalar-only compiler and runtime-only package builds, exact full-5000 runtime output and the unmodified generated application's success/error/denial checks. Focused child funding, nested-phase, continuation, work-price, retirement and argument-bank checks pass.

**Child-funding counter audit passes.** All128 scalar controls match IR225 in every non-timing field. All five compression rows match the original IR226 compiler, its fresh repeat, IR227 and a production IR227 build with child funding disabled, including allocations and every report field. A same-compiler enabled/disabled test also preserves exact outputs and all counters, with **4,700 actual funded activations versus zero**. The75-byte output independently inflates, input aliases remain unchanged and cleanup is exact. Diagnostic patches, source inventories, build/run commands and raw receipts are retained in `evidence/funding-audit-*` and `evidence/funding-disabled-*`.

**Reporting correction:** the earlier claim that IR226 matched all133 IR225 controls was wrong. The original IR226 comparison already recorded three different compression measurement rows, but its script printed a misleading success message without asserting that flag. Those original raw receipts and their false comparison flag are now included. Compression's work850,632,784,230 →30,944,255,711, visits30,907 →1,647 and steady allocation requests23,347 →3,376 changed with the IR226 native integration; they do not change with child funding. Requested bytes1,431,695,700 →23,977,116 and all other differences remain in the before/after receipt. Work counters are not hardware instructions. No comparison fields were excluded to pass this audit; an assertion of cross-compiler IR225 equality remains false. The audit passes; performance and selection are documented below.

The feature-boundary repair also keeps the efficient scalar-only compiler available. Runtime-only, Managed generation, warnings-as-errors, and the current full GPU checks pass. The additional loader, finite-budget and old-Object qualification below now passes.

The retained IR22731-round balanced comparison measures the complete continuous post-JSON interval, including input admission, setup and worker creation/run/join, and fresh launch through exit separately. Every variant computes all5000 posts and its output exactly matches standard Rust. The fastest matched Rust control is single-threaded.

| Variant | Post-JSON | × fastest matched Rust | Whole process | × fastest whole process |
|---|---:|---:|---:|---:|
| Mech original,4 native workers |2.942167 ms|1.846066×|38.678625 ms|1.520997×|
| Mech compact U8/U32,4 workers |2.989125 ms|1.875529×|39.778458 ms|1.564246×|
| Mech original,8 workers |3.182208 ms|1.996680×|38.651750 ms|1.519940×|
| Mech compact U8/U32,8 workers |3.194000 ms|2.004078×|39.289750 ms|1.545028×|

Matched Rust is1.593750 ms (`rust-mech-control-u8-ids-u32`); the independently fastest whole-process Rust is25.429792 ms (`rust-mech-control-u32-casts`). Their exact sources and flags remain in the packet. The earlier15-round confirmation gave2.950416 ms /1.844298× and39.109333 ms /1.485306× against its contemporaneous references; its raw samples are retained separately. Do not compare ratios from different cohorts as if their baselines were identical.

Against the prior selected IR208 in the31-round cohort, original-four wins31/31 post-JSON pairs and19/31 whole-process pairs, with median paired deltas−0.263583 ms and−0.796959 ms. Its whole median is lower, but it does not meet the recorded21/31 paired-win criterion in both intervals, so **IR208 stays selected**. The current integration is a qualified review candidate; parity remains unmet. Four workers consistently beat eight on the post-JSON interval; whole-process medians for original-four/eight are effectively tied in this cohort.

The separate seven-sample resource cohort records **57,087,085 post-JSON instructions** versus parent63,689,244 (−10.37%) and fastest matched Rust19,222,098 (**2.970× Rust**). Allocation requests remain **3,858 versus Rust58**; requested bytes8,044,006 versus6,825,713; peak live requested heap13,616,157 versus7,663,209. The generated program is908,464 bytes and its hot query frame528 bytes, down from930,772 and672. The complete executable grows16,390,576 →16,805,680 bytes and host/runtime text6,749,060 →6,973,240 bytes. Generated-program size, whole executable size, allocator requests and residency are distinct measurements. Resource reference selection is pinned to the15-round cohort; all resource variants, including the later31-round winner, remain in the raw report.

Additional qualification passes **434 loader fixtures**, **52 byte-identical original/decoded bundle files**, **48 exact finite denials across three loaders and two source families**,12 finite first10/full5000 output checks,16 forged-frontier rejections, and **25 genuine old-Object checks**, including same-ABI/work-policy IR226. The complete comparison cohorts contain **2,832 exact outputs**, plus **2,000 exact profile invocations**. All53 numeric functions per variant retain zero general helpers on successful native backedges.

The fresh profile maps only the actual current native binaries. The query tree remains the leading application-code sampled position; stack loads/stores, comparisons and moves remain observed alongside useful rank arithmetic. Host `collections::data::element`, snapshot measurement and output preparation also remain. These are sampled positions, not CPU-time percentages; waiting positions are explicitly separated. The phase-only funding pass does not select the query tree. Next compiler work should extend proved bounds/funding to native trees and reduce constructor/admission/materialization work without weakening source errors or accounting.

One earlier31-round attempt exhausted disk during **untimed duplicate-output retention** after1344 captured invocations. Its partial report and1357 recoverable output copies are preserved separately and excluded from headline results. The complete rerun uses verified immutable content-addressed copies after each timed process exits; producer paths cannot modify retained evidence. All original commands, generated code, source and timers are unchanged. The indexed local archive verifies **2,281 files**,2,880 completed-cohort output copies and1,357 failed-cohort copies. Raw archives were not remotely uploaded.

See `evidence/current-screen-report.json`, the compressed complete latency/resource/profile reports, qualification receipts and `local-archive-index.json`. `verify-review.py` checks their hashes and the current compiler source independently from the historical IR223 timing evidence.

Both compiler paths lower source work to native instructions. Recursive frames and immutable captured arrays retain their must-current tree proof. Bounded and partial phases retain #973's separate coverage proof, then convert to the shared checked native representation. The combined emitter includes local-phi normalization, range cleanup, completed-child read windows, child settlement and prepayment. Those phase proofs do not yet cover the related-post current-funded query trees; their presence is not a claim that every pass optimizes this benchmark.

The integration also preserves independently proved child entries and parent pieces when an outer phase declines. A live paid route must still match its exact primary, and overlapping paid source work still declines admission. Helper tags, serialized metadata, current scalar-bank imports, recursive owner projections, cold exports and image validation use the combined ABI and work policy.

`verify-review.py` verifies the retained timing evidence against its historical compiler Git tree and verifies the current integration's Rust/manifest inventory separately. The timings in the original review guide still belong to **IR223**, not this integration. The IR225 screen in `evidence/` is also a separate, unselected experiment: **3.133083 ms /1.960679× matched Rust**, **38.782792 ms /1.490356× complete Rust**, with 0.70% fewer instructions but a 0.64% processing regression against IR224. IR208 remains the selected speed checkpoint; Rust parity is unmet.

Build plans, receipts, and archive references retain the original local paths and pinned dependency hashes. This is not a portable one-command build or a remotely hosted raw-archive claim.
