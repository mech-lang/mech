# PR #995 current status

**Working selection: `m1-narrow-packet-words`, 63,804 B, 81/81 Roc and 63/81 libdeflate in both complete 96-block orders.** Every required named point win is retained, and all required confidence checks also pass in both directions. This saves 48 B versus the preceding 63,852 B working build and 64 B versus the original eight-entry 63-win build.

Natural 4 KiB measures **0.981214× / 0.992852× C**. The point-win requirement remains the selection rule; confidence is reported alongside it. The one narrow discovery loss did not discard this candidate; complete 96-block evaluation established all 63 named C wins.

Source through final executable link on Apple M1 takes **2.222473s**, paired **0.742624× Roc / 4.538331× C**, in the existing 18-trial matched cohort. Roc takes3.005122s and C0.490298s. Stripped executable **276,072B**; peak stack **4,736B**. Encoder text **1.106× Roc / 1.721× C**. No material compile-time gain is established. All compilers and fixed support are prebuilt equally; final linking is included and stripping/bootstrap are separate.

`rebuild.py` defaults to this recipe on Apple AArch64. Explicit selection: `--variant m1-narrow-packet-words`. Actual named-recipe normal/checked emission reproduces the measured object, full metadata and canonical bytecode exactly. [Current source, both full orders, every named win, confidence bounds and reconstruction](evidence/encoder-size-gap/packed-word-packet-selection/README.md). [Prior twelve-recipe study, tests, gates, counters and build evidence](evidence/encoder-size-gap/packed-word-repair-study/README.md).

Original 2,719-line Mech source and canonical bytecode remain unchanged; no LLVM dependency is added. The next milestone is **64/81 C**, preserving all 63 existing C names and 81/81 Roc; the full goal remains **1× libdeflate across all 81 cases**. The next major diagnostic is a current Mech/C phase-and-work comparison across the remaining losses, as requested in the new brief. Existing unsafe audit still has the unrelated missing archived allocator blocker.

Current attribution now includes **45 fresh owner captures, exact nine-case Mech/C parsing-work alignment, C search counters and 212 real Huffman vectors**, plus current matcher instruction and stack counts. Both Mech scorers match C exactly on passes, node updates, no-match positions, match records and candidate lengths. The opt-in carried-cost candidate takes **2.42–2.51% less time on random 1 MiB** in its two 21-case discovery orders. All observed wins survive, but no 64th win or full 81-case qualification is claimed. The selected 63-win recipe is unchanged. Comparable total matcher/setup/output work, Mech frequency capture, phase replay and aligned coarse phase times remain outstanding. [Current evidence, all raw observations and portable verification](evidence/current-phase-work-study/README.md).

Everything below is historical: references to a selected/default recipe apply to that checkpoint.

## Previous confidence-qualified selection and experiments

**Qualified throughput recipe: `m1-bounded-byte-histogram`, 63,612 B, retaining every prior individual 81/81 Roc and 62/81 libdeflate win.** The smaller `m1-word-complement-current-reads` remains available at 63,388 B. No 63rd C win is yet confidence-qualified; the goal remains 1× libdeflate performance.

The guarded histogram takes **2.44–2.73% less time on random 1 MiB** than its same-run compact reference, at a cost of 224 B. It measures **1.325151× / 1.324508× C** in the original full orders. Natural 4 KiB improves 0.30–0.60% and remains **1.027657× / 1.031202× C**. [Source, raw observations, qualification and reproduction](evidence/encoder-size-gap/bounded-byte-histogram-study/README.md).

Qualification uses 81 cases, forward96 and pooled reverse288 blocks per case. The original reverse96 had two uncertain Roc comparisons during large timing swings; the complete reverse192 repeat independently supports all 81/62 wins, and all 288 reverse blocks remain included in the pooled checks. Every prior named win is checked explicitly. All 162 output/inflation/alias observations, six failures, twelve injected failures, twelve recoveries and 4,864 B peak stack pass. Normal and checked allocation emission match exactly.

Latest complete-build cohort on the **Apple M1** reproduces the qualified encoder exactly and measures **2.213505 s Mech, 2.986912 s Roc and 0.482204 s C**, ten balanced fresh-process trials including final executable linking with equal prebuilt compilers/support. Stripped executables remain 276,072 B / 191,912 B / 155,752 B. Bootstrap and strip are separate; no compilation-time gain is established. [Current build evidence](evidence/encoder-size-gap/scalar-pair-revisit-study/README.md).

The histogram source is committed at **7ea6c80d8db874e10392bdb2a3d38e6205ca876d**. Its new entry guard removes four per-byte bounds checks and retains the U32 increment check; a guard miss executes the original checked code. The two relevant repository tests pass 11,760 native calls. The original 2,719-line Mech source and canonical bytecode remain unchanged, with no LLVM dependency added. Rebuild with `--variant m1-bounded-byte-histogram`.

Both earlier mismatch finalists and all adverse observations remain available in the [review follow-up](evidence/review-followup-20261119/README.md). The first-word probe remains experimental. The initial admitted-index-range pass removes no checks from this codec: the existing 262,144-entry floor covers an already optimized history table; the observation counters need a separate extent proof. The source utility's tests and further work remain separate from the qualified histogram gain.

The entire GPU suite is not claimed. Its relevant histogram and earlier wide-equality tests are recorded separately, and the existing missing archived allocator continues to block the repository unsafe audit. [Review and continuation handoff](REVIEW.md).

## Current paired scalar revisit

Compiler source **`84628d979e`** restores the historical paired scalar continuation as an opt-in path. It passes **20 optimized tests in four configurations**, including **22,528 new native calls** per configuration, and all **33 older recipes reproduce exactly**. Both main candidates and four register-setting workers pass output/error/stack/recovery gates and portable normal/checked replay. The qualified **63,612 B / 81 Roc / 62 C** default remains selected; the preceding eight-entry candidate still retains 63 C point wins in both full96 orders, with reverse confidence crossing parity.

Pairing increases natural 4 KiB stack loads **17,310 → 105,720** and instructions **3,266,626 → 3,589,068**, despite removing 3,881 branches. Its original head variant costs **1.96–2.44% more natural 4 KiB time** and **4.21–4.86% more natural 8 KiB time** than its held control, losing the old 8 KiB C win in both orders. Existing register settings leave the scorer body and counts unchanged; their four additional orders are preserved. Keep the source and repair the cost/best/item lifetimes. A separate eleven-case census also identifies the literal-only DP route behind random-input losses. [All eight orders, checks, counts, builds and replay](evidence/encoder-size-gap/scalar-pair-revisit-study/README.md).

Head text: **64,108 B**; complete build **2.219618 s**, paired **0.743139× Roc / 4.606172× C**; stripped executable **276,072 B**; peak stack **4,736 B**. No compile-time gain is shown. Next, test sequential updates inside the unrolled pair and optimize the literal-only route separately. These follow-ups are not yet implemented.

## Preceding guarded eight-entry experiment

Compiler source **`60e942f462`** adds guarded eight-candidate entry to the paired packed scorer. Head candidate: **63,868 B**, **4,736 B** stack. It removes **66,937 natural-4-KiB scorer instructions and 36,501 branches** versus the sixteen-entry head worker, with unchanged stack traffic. Every original named 81 Roc / 62 C win remains precision-supported in both full96 orders. The new reverse natural-4-KiB interval still crosses parity, so the 63rd C win is not yet qualified. The qualified 63,612 B histogram remains selected until a separate promotion. [All eight runtime orders, confidence results, source, tests, counts, builds and portable replay](evidence/encoder-size-gap/packed-eight-entry-study/README.md).

Forward96: **81 Roc / 63 C point wins**, natural 4 KiB **0.993901× C**, 90% interval **-1.144% to -0.096%**. Reverse96: **81 Roc / 63 C point wins**, natural 4 KiB **0.997675× C**, 90% interval **-0.818% to +0.388%**.

Head complete build: **2.215637 s**, paired **0.742654× Roc / 4.604948× C**, stripped executable **276,072 B**. Nineteen optimized tests pass in three configurations; 31 older recipes and both portable normal/checked artifacts reproduce exactly. No compilation-time gain is established.

## Preceding paired scorer margin experiments

Compiler source **`2b6ed311d`** keeps the paired scorer and tests head-group processing plus wider item decoding. The head candidate is **63,836 B**, saving **16 B** and **10,308 executed natural-4-KiB scorer instructions**. It measures **0.998497× / 1.000382× C**, with **1.000019× / 0.998029×** time versus the exact paired-carried control. All prior wins present in eight complete 21-case discovery orders survive. No new full score or qualified 63rd win is claimed; keep the saving and continue the paired scorer. The qualified default remains **63,612 B / 81 Roc / 62 C**.

Wide-only and head+wide are exact artifact no-ops for this codec. Combining width changes with the older deferred-item option removes 2,955 instructions, but the deferred family does not establish a latency gain. All six workers pass output/error/stack/recovery gates and portable normal/checked replay. Nineteen optimized tests pass in four configurations; 25 older recipes reproduce all artifacts exactly. Head build is **2.222766 s**, paired **0.744078× Roc / 4.610974× C**, with **276,072 B** stripped executable and **4,736 B** peak stack. No compile-time gain is shown. [Source, all eight distinct-code orders, negative results, counts, checks and complete builds](evidence/encoder-size-gap/packed-margin-repair-study/README.md).

Next, enter at eight only under the packed bounds and retain the original short wide-cost scalar path. This revisits the earlier short-vector work with the new paired scorer; it is not implemented in the linked source epoch.

## Preceding paired packed-accumulator experiments

Preceding compiler source **`31910aff6`** adds two independent packed accumulators over eight candidates. The **63,852 B** carried probe takes **1.50% / 1.59% less natural-4-KiB time** than its matching one-accumulator control. Both full81/24-block orders win **81 Roc / 63 C**. Full96 then retains every original **81 Roc / 62 C** named win with confidence support, but natural 4 KiB is **0.984695× C forward / 1.000362× C reverse**. Its reverse 90% interval spans parity (−0.429% to +0.528%). **Keep the candidate and repair this one margin; no qualified 63rd win is claimed.** The smaller qualified 63,612 B histogram remains selected.

Complete carried-probe build is **2.215453 s**, paired **0.742850× Roc / 4.606302× C**; encoder text is **1.107× Roc / 1.722× C**, stripped executable **276,072 B** and peak stack **4,736 B**. All prior named wins, output/error/recovery/stack gates, three configurations of eighteen optimized tests, 23 exact older recipes and both portable normal/checked emissions pass. No compile-time gain is shown. [Source, all eight runtime orders, both confidence reports, counts, complete builds and replay](evidence/encoder-size-gap/paired-packed-accumulator-study/README.md).

Next, test optional-head peeling and a widened packed ordinal separately and together. This targets redundant loop-bound setup and item-index reconstruction while retaining this exact paired candidate. These repairs are not yet implemented.

## Preceding bounded packed-key experiments

Preceding compiler source **`f3d74d948`** integrates bounded packed cost/ordinal keys with the original optimized vector fallback. The packed probe takes **0.89% / 0.88% less natural-4-KiB time** than its matching control, reaching **1.005062× / 1.008491× libdeflate**. No 63rd win is established. Carrying indices removes more instructions; a broader profile repairs its distribution-change slowdown to **0.39% / 0.57% less time** than its matching revised-profile control. All prior peer wins observed in fourteen complete 21-case orders survive; the qualified **63,612 B / 81 Roc / 62 C** recipe remains selected.

The repaired probe is **63,660 B**, **1.103× Roc / 1.717× C encoder text**, with **4,736 B** peak stack. Its complete executable build is **2.221298 s**, paired **0.743629× Roc / 4.605955× C**; the same-cohort qualified build is **2.218823 s**. No compile-time gain is shown. Eighteen optimized tests pass in all three feature configurations; sixteen old recipes and all seven portable normal/checked emissions reproduce exactly. All seven workers pass output/error/stack/recovery gates. [Source, all fourteen raw orders, counts, build profiles, verifier and reproduction](evidence/encoder-size-gap/packed-cost-ordinal-study/README.md).

Next, test two independent packed accumulators over eight candidates to reduce the key/minimum dependency and loop branches. Preserve the existing vector fallback, exact tails, all named peer wins and every adverse observation. This is the next experiment, not an implemented result.

## Preceding conditional winner experiments

Preceding source **`46dc3230f`** completes the conditional winner experiment. The combined word is **62,796 B** and saves about 1% on distribution change against its 62,844 B control, while natural 4 KiB stays near **1.025× / 1.035× C**. The index-only guard is slower. Moving item reconstruction to the same strict-winner edge recovers its 32 B growth and removes 19,427 natural-4-KiB scorer instructions, but still adds 3,550 branches and establishes no latency win. All prior peer wins observed in twenty complete twenty-one-case orders survive; the qualified **63,612 B / 81 Roc / 62 C** result remains selected.

The mapped word is **62,796 B**, or **1.088× Roc / 1.694× C encoder text**, and builds through final link in **2.215644 s**, paired **0.741685× Roc / 4.605130× C**. All ten variants pass output/error/stack/recovery and checked replay. Sixteen/seventeen optimized tests and 8,192/16,384 native fixture calls pass. A separate 8,192-call packed-key prototype emits one horizontal minimum instead of two under explicit U16 bounds; it is not codec-integrated or a speed claim. Next, admit its bounds and retain the old vector path on a miss. [All twenty raw orders, source, counts, complete builds, prototype and reproduction](evidence/encoder-size-gap/conditional-winner-item-study/README.md).

## Preceding packet phase-selection experiments

Source **`3fe63cb1d`** allows exact phase selection for the existing packet-load qualifier. Selecting nine complete routine families recovers **320 B** of the original 416 B saving: word **62,844 B**, probe **63,116 B**, both with 4,736 B peak stack. All prior peer wins observed in fourteen complete twenty-one-case discovery orders survive; no full score or win #63 is established. The current all-phase probe still costs 8.80–9.26% on distribution change, while the narrower selection costs 0.11–0.45% in the probe orders and 0.20–0.63% in the word orders. The smaller 62,796 B non-scorer candidate retains its 5.86–6.20% penalty for repair.

The selected-routine word builds through final link in **2.213414 s**, paired **0.741357× Roc / 4.606720× C**. All six new candidates pass outputs/errors, twelve failures/recoveries, stack gates and normal/checked artifact replay. The qualified 63,612 B / 81 Roc / 62 C result is unchanged. Next, combine the word scorer-only improvement with the nine-family saving and isolate the four extra families responsible for the remaining 48 B tradeoff. [All source, fourteen raw orders, exact-mode checks, full build cohort and reproduction](evidence/encoder-size-gap/selected-packet-phase-study/README.md).

## Preceding fixed-read window experiments

Source **`2dcf8babd`** reuses the selected `window >= 5` edge to admit a four-byte read at `position + 1`, without a new runtime guard or cloned body. Word is **63,164 B**, probe **63,436 B**, and packet-qualified **63,020 B**, all with 4,736 B stack. The probe takes 0.13–0.38% less natural-4-KiB time and 0.44–0.50% less random-1-MiB time than its matching clamped-window base. Natural 4 KiB remains **1.010619× / 1.017073× C**. Four historical carry/load/hash combinations retain the panel wins but earn no additional win; hash narrowing slows random 1 MiB by about 4.1%. All fourteen complete twenty-one-case discovery orders remain available. Full 81-case confidence qualification is outstanding.

The probe builds through final link in **2.217276 s**, paired **0.741816× Roc / 4.587254× C**, in its twelve-trial cohort. Twenty-five relevant repository tests pass, including 72,752 native calls; all seven candidates pass output/error/stack/recovery gates and checked artifact replay. The qualified 81 Roc / 62 C selection is unchanged. Next, isolate packet-load qualification by phase to recover its 416 B saving without the earlier hot-scorer slowdown. [Source, all observations, dynamic counts, complete builds and reproduction](evidence/encoder-size-gap/fixed-read-window-study/README.md).

## Preceding clamped-window experiments

Source **`330416904`** proves the collector's clamped window fits the admitted input and consumes the facts with late constant-branch cleanup. Word is **63,180 B**, probe **63,484 B**, and packet-qualified **63,068 B**; all use 4,736 B peak stack. The probe takes **1.04–1.53% less time on natural 4 KiB** and **1.61–1.62% less time on random 1 MiB** than its matching input-word/probe base. Natural 4 KiB remains **1.011199× / 1.009384× C**. All previously required peer wins present in all six twenty-one-case discovery orders remain point wins. There is no new full score or 63rd C win.

The probe builds through final executable link in **2.213320 s**, paired **0.742111× Roc / 4.602746× C**. Twenty-three relevant tests pass, including 47,552 native calls; all three candidates pass outputs/errors, twelve failures/recoveries, checked emission and stack gates. The first comparison-only implementation left constant branches and is preserved with its adverse counts. Next, reuse `window >= 5` for the remaining four-byte read. [Source, all data, machine counts, complete-build profiles and reproduction](evidence/encoder-size-gap/clamped-input-window-study/README.md).

## Preceding input-word and packet experiments

Source **`baeab0027`** adds an admitted eight-byte input floor. The word candidate is **63,292 B / 4,736 B stack**, with a **2.213294 s** complete build, paired **0.742082× Roc / 4.611124× C**. The combined word-probe candidate is **63,756 B** and measures **1.009970× / 1.022278× C** on natural 4 KiB against its matching held-feature control. These are twenty-one-case, twenty-four-block discovery orders, not a new full score. Every previously required peer win observed in the panel is retained. The smaller word's reverse `320-overlap` win is fragile at 0.999809× C.

Revisited packet-load qualification reduces the probe to **63,340 B**, but slows natural and distribution-change inputs. All five packet alternatives and all eighteen discovery orders are retained for repair. Twenty-one relevant tests pass, including 34,440 span/histogram native calls. All nine timed variants pass output/error/stack gates; word and probe pass twelve failures/recoveries. Source **`6062e2e65`** adds actual optimized IR inspection, with twelve artifact-identical plain/inspected/checked replays. The actual collector's clamped input-window checks are the next proof target. The qualified 81 Roc / 62 C selection remains unchanged. [All source, raw data, checks, complete-build profiles and portable reproduction](evidence/encoder-size-gap/admitted-input-word-study/README.md).

## Preceding input-span and first-word experiments

Compiler source **`b55805f7f`** proves `begin <= end <= current input length` for the private unit-stride collector, after readonly-load conversion. The word candidate is **63,292 B / 4,832 B stack**, and builds through final link in **2.217011 s**, paired **0.742764× Roc / 4.595761× C**. It now retains every prior named win in both full 81-case, 24-block point orders: **81 Roc / 62 C**. It remains unpromoted pending full 96-block confidence qualification. Other current variants still have nineteen-case panels.

Combining the span proof with the first-word probe takes **1.27–1.30% less time** than its held-feature vector/complement control on natural 4 KiB, in both orders. It remains **1.024347× / 1.039711× C**. All previous wins in the nineteen-case subset are retained; this is not a new 81-case score. The earlier bitwise word candidate's reverse `320-overlap` loss and every adverse observation remain recorded.

Fifteen relevant repository tests pass, including 23,856 span/histogram native calls and the equality probe tests. All five current alternatives pass 162 outputs, six failures and the stack cap. Word, vector/complement and the combined probe also pass twelve failures/recoveries and normal/checked artifact replay. Public profiles reproduce word and the probe exactly. [Source, all alternatives, raw data, build profiles and portable reproduction](evidence/encoder-size-gap/admitted-input-span-study/README.md).

## Earlier experimental compact candidate

The guarded-index word recipe is **63,388 B** with **4,720 B** peak stack. Word and vector/complement retain all prior named wins in both complete 81-case, 24-block point orders: **81 Roc / 62 C**. All 162 outputs, six failures, twelve recoveries and checked emission pass. Full 96-block individual confidence qualification remains pending, so the qualified histogram selection above is unchanged. No 63rd C win is claimed.

The minimal word recipe builds a final executable in **2.215846 s**, paired **0.741457× Roc / 4.583660× C**, with 276,072 B stripped deployment. Ten balanced fresh-process trials include final linking and equal prebuilt support. Rebased verified region hints avoid the optional compiler feature's measured 3.9% overhead. Both the slower intermediate modes and raw counter regressions are retained for repair. [Source, full point screens, build cohorts and portable reproduction](evidence/encoder-size-gap/bounded-index-range-study/README.md).

## Previous source-unit cohort

Compiler source **`44fc9f05304111c6e75cac54a3dc5ec5c4f443bf`** adds source-relative unit extraction and caller-owned boundary storage. The selected recipe remains **`m1-immutable-current-reads`, 63,532 B, 81/81 Roc and 62/81 libdeflate wins in both complete orders**. Current minimal and experimental compiler configurations reproduce its exact artifacts in normal and allocation-checked modes; fixed-support linking reproduces the same qualified `834c7a7e…` worker. No 63rd C win is claimed.

| Latest Apple M1 build | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Earlier compiler, selected encoder | 63,532 B | 2.241802 s | 2.252804 s | 276,072 B |
| Current minimal compiler, selected encoder | **63,532 B** | **2.245045 s** | **2.256023 s** | **276,072 B** |
| Experimental compiler, split disabled | 63,532 B | 2.215508 s | 2.226393 s | 276,072 B |
| Repaired source unit, unselected | 65,948 B | 2.282298 s | 2.293045 s | 276,072 B |
| Roc | 57,696 B | 2.993610 s | 3.004575 s | 191,912 B |
| libdeflate | 37,080 B | 0.491877 s | 0.502805 s | 155,752 B |

Eighteen balanced fresh-process orders per actor include final linking, with compilers and fixed support prebuilt for all. Strip is separate. Paired ratios for the current selected build are **0.739423× Roc / 4.540212× C**. Whole-encoder ratios remain **1.101151× Roc / 1.713376× C**, leaving 26,452 B above C.

The current minimal compiler is a paired **1.058 ms slower**, 90% interval **−8.222 to +19.025 ms**, than the earlier compiler producing the same encoder: no established change. The experimental compiler configuration also has no established advantage. The repaired unit costs a paired **66.899 ms**, interval **63.081–71.099 ms**, against its own compiler with the split disabled. Separate cohorts' medians do not establish causal gains.

Current phase medians: parsing **706.538 ms**, semantics **225.121 ms**, artifact preparation **6.921 ms**, encoding **2.667 ms**, native pipeline **1,226.643 ms**, final link **35.812 ms**. Catalog work overlaps parsing; separate medians do not add. Source peak RSS is **392.2 MB**.

Source-relative extraction, shared packets, owner homes, borrowed private output scratch and root-scoped register preference reduce the early experiment from 69,556 to **65,948 B**, with actual active stack **5,280 B**. All 162 output/alias/inflation observations, six failures and 12 recoveries pass. It remains 2,416 B larger than the selection and loses the existing `320-overlap` C win in both 17-fixture focused orders (**1.007444× / 1.009468× C**). Its full runtime score is unmeasured. The source machinery and adverse evidence are retained for repair; the experiment is not selected.

Four boundary tests, five production Words tests, five production phase-storage tests and nine scalar success/error probes pass. Normal/checked replay and portable published replay both reproduce all three artifacts. Four compiler captures identify full continuation-liveness computation as added early-unit planning work. The corrected identical-body inventory finds one 296 B candidate, without proving safe folding. Formatting, module layout and warning policy pass; the existing missing archived allocator path still blocks the unsafe audit.

**Immediate work targets a runtime win on natural 4 KiB**, whose selected result needs about 6.45% less time in the worse order. Combine bounded scorer repairs with earlier gains, keep every existing peer win and the 81/62 floors, and price size and complete build time before selection. The 1× libdeflate goal remains unfinished. [Source, all alternatives, raw timings, checks and portable reproduction](evidence/early-source-unit-repair/README.md).

## Previous standard-module integration cohort

The final compiler source is **`2cd17fca3c9d58a5804a18c904fe1daf65598b78`**, including standard native/parser module layouts and a separate allowlisted JIT test boundary. Its normal and allocation-checked builds reproduce the same **63,532 B** object, full metadata and canonical bytecode. Relinking with the original fixed support reproduces qualified worker **`834c7a7e…`** exactly, so its **81 Roc / 62 C point and precision wins in both complete orders**, output/error/recovery/stack checks and profiles remain applicable.

| Latest Apple M1 build | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Earlier compiler, same qualified encoder | 63,532 B | 2.204676 s | 2.215713 s | 276,072 B |
| Final source/compiler configuration | **63,532 B** | **2.208667 s** | **2.219695 s** | 276,072 B |
| Roc | 57,696 B | 2.980663 s | 2.991847 s | 191,912 B |
| libdeflate | 37,080 B | 0.480168 s | 0.490875 s | 155,752 B |

Eight balanced fresh-process builds per actor use prebuilt compilers and fixed support for every actor. Paired ratios are **0.741248× Roc / 4.602361× C**. The final compiler/configuration costs a paired **3.360 ms** versus the earlier compiler generating the same encoder, 90% interval **0.941–5.050 ms**, 1/8 faster. This comparison includes the mechanical source/build changes; it does not isolate the immutable-read pass. The earlier sixteen-trial same-compiler study below found no established effect from that pass. Do not add or compare separate cohorts as causal gains.

Latest phase medians are **702.055 ms parsing**, **214.546 ms semantics**, **6.834 ms artifact**, **2.532 ms encoding**, **1,221.209 ms native pipeline**, and **35.044 ms final link**. Catalog work overlaps parsing; separate medians do not add. Source peak RSS is **392.9 MB**.

The published named rebuild command also reproduces the encoder and passes **162 outputs, six failures and 12 recoveries**. It rebuilds host support and has a different worker hash (`01ace001…`), so that worker has no inherited runtime-win claim. The matched timings use fixed prebuilt support; runtime qualification belongs to the exact `834c7a7e…` worker. Its fixed-support reproduction is explicitly verified.

Fresh **536 default-feature syntax tests** and four load tests pass after the module moves; the prior 539 full-feature syntax tests are archived separately. Workspace formatting, module layout and warning policy pass. The unsafe audit remains blocked by the existing missing `benchmarks/mech-deflate/comparison/rust/src/pinned_allocator.rs`; no CI-green claim is made.

## Algorithm qualification and original pass-isolation cohort

The selected M1 recipe is **`m1-immutable-current-reads`**: **63,532 B**, saving **160 B**, with **81/81 Roc and 62/81 libdeflate wins in both complete orders**. Every paired control win and all 81/62 precision-supported wins are retained. Original Mech source and canonical bytecode are unchanged; no LLVM dependency is added. Compiler source is integrated at `5ccf097dc02ceb3abb17bccb85cd11a3ae144503`.

The source-through-final-link build is **2.211926 s**, versus **2.213335 s** with the pass disabled in the same compiler. The paired difference is **-0.139 ms**, 90% interval **-3.628 to +3.598 ms**, 8/16 faster: **no established compile-time change**. Sixteen balanced trials per actor include final linking, with support prebuilt for every actor. The complete stripped executable remains **276,072 B**. Runtime reports **13 established small slowdowns above 0.5% and 15 uncertain comparisons**; no individual peer win is lost. Large random 1 MiB costs 0.54% / 0.77% versus control. [Source, all controls, full orders, profiles and complete builds](evidence/encoder-size-gap/immutable-current-read-study/README.md).

| Actor | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Same compiler, pass disabled | 63,692 B | 2.213335 s | 2.224324 s | 276,072 B |
| Selected immutable-read reuse | **63,532 B** | **2.211926 s** | **2.223045 s** | 276,072 B |
| Roc | 57,696 B | 2.988797 s | 2.999631 s | 191,912 B |
| libdeflate | 37,080 B | 0.491145 s | 0.502016 s | 155,752 B |

Paired build ratios are **0.739157× Roc / 4.505969× C**. Encoder ratios are **1.101151× Roc / 1.713376× C**, leaving **26,452 B** above C. Compiler bootstrap is excluded for all actors; strip is separate. Historical cohort medians are not causal comparisons.

Current phase medians: parse **704.371 ms**, semantics **217.767 ms**, native pipeline **1,218.926 ms**. Catalog work overlaps parsing and independent medians do not add. The post-encode sample finds dominance, SSA construction, validation and guard inference among the leading active functions; it is not an exclusive phase timer.

The historical literal-reuse lead is now a generic immutable-loan/current-index proof. A 63,484 B alternative lost C on `320-overlap` by 0.18% in one focused order. Restricting the pass to the two certified reads and allowing backend rematerialization keeps 160 B of that saving and restores the win in both full orders. The original branches remain for all uncertified reads. All adverse variants and source snapshots are retained.

The selected worker is `834c7a7e423a69c46483a3dae74d9450949fdf142b5ad2dede894e0da3ca8693`. Normal and allocation-checked source builds reproduce identical object, full metadata and bytecode. It passes 162 output/inflation/alias observations, six failures, 12 recoveries and **4,848 B** peak stack under the 5,280 B ceiling. The root remains **46,788 B**, with a **3,296 B** frame. Four executable load tests cover both placement choices; four unchanged optimized private-contract tests cover original/decoded loans and refusal paths. Prior full-feature syntax qualification remains 539 tests; the final source-identical parser module move has its own fresh 536-test default-feature check.

Fresh actual-worker samples put **72.3%** of 1 MiB encoder work in matching and **14.4%** in scoring. Natural 4 KiB is **52.3% scoring / 24.5% matching**. Large random still has the largest absolute gap, about **7.344 ms/call**; natural 4 KiB is the closest next-win check at **1.063757× / 1.068946× C**, needing about **6.45%** less time in the worse order. Sample shares are not elapsed phase timers or forecasts.

The 1× libdeflate code/build goal remains active. The source-unit repair above is still unselected. Immediate work targets the natural 4 KiB scorer while preserving every existing peer win and both 81/62 floors; target 63 C wins next. [Review handoff](REVIEW.md).

```sh
python3 rebuild.py /tmp/mech-immutable --variant m1-immutable-current-reads \
  --mimalloc-object /path/to/mimalloc.o
```

## Previous sealed-parser cohort (63,692 B)

**The compiler combines deterministic sealed-input parser phases and checks the work allowance before removing a pending frame.** The then-selected encoder remained **63,692 B**, with **81/81 Roc and 62/81 libdeflate wins in both full orders**. The new compiler reproduces its native object, complete metadata, canonical bytecode and fixed-support worker byte for byte. No new runtime campaign or 63rd C win is claimed. Original Mech source is unchanged.

| Actor | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Before, same selected encoder | 63,692 B | 2.212686 s | 2.223608 s | 276,072 B |
| Sealed parser combination | 63,692 B | 2.205340 s | 2.216274 s | 276,072 B |
| Roc | 57,696 B | 2.988178 s | 2.999173 s | 191,912 B |
| libdeflate | 37,080 B | 0.480449 s | 0.491357 s | 155,752 B |

Eight balanced fresh-process trials per actor include final linking, with fixed support prebuilt for every actor. The paired complete-build saving is **8.203 ms**, 90% interval **4.951–11.409 ms saved**, 8/8 faster (**0.371%**). Paired complete-build ratios are **0.738040× Roc / 4.589692× C**. Compiler bootstrap is excluded for all actors; its diagnostic log is retained separately. Strip is reported separately. Do not infer causal gains from separate cohorts.

This is parser maintenance, not the large code-size or build-time reduction still needed for 1× libdeflate. Encoder text stays **1.103924× Roc / 1.717691× C**; the C text gap is still **26,612 B**. Peak active encoder stack remains **4,848 B**, under the 5,280 B ceiling, and deployment size remains 276,072 B.

Current phase medians: parse **0.701823 s**, semantics **0.217373 s**, native **1.214269 s**. Catalog work overlaps parsing. The identical worker reuses its existing full runtime/stack/failure/recovery qualification. **539 syntax tests pass**. [All evidence and source variants](evidence/sealed-base-phase-fusion/README.md).

Fresh profiles of this exact selected worker place **72.8%** of 1 MiB random encoder samples in matching, versus **14.2%** in scoring. Natural 4 KiB is **54.0%** scoring and **23.6%** matching. The qualified median excess is about **7.13 ms** per large-random call versus **21.8 µs** for natural 4 KiB. These are sampled work shares and separately measured full-call gaps, not phase timers or savings forecasts. Prioritize the shared random collector and retain natural 4 KiB as the next-win check.

## Previous matcher guard/layout qualification cohort

The selected recipe is **`m1-matcher-guard-facts`**: **63,692 B encoder text**, **81/81 Roc and 62/81 libdeflate wins in both complete orders**, with every previous winning fixture and all 81/62 precision-supported wins held. It combines allocator spill pairing with two generic impossible-guard proofs and a fresh matcher layout. It saves **64 B** from the qualified paired recipe and **112 B** from the earlier 63,804 B reference. Original Mech source and canonical bytecode are unchanged.

The actual fixed-support worker is `88321c097d29b7219f639abbc5fcce727f0602de2d05ab24e5e2b47f2996c3b4`. The checked root stays **46,788 B**, frame **3,296 B**, peak active stack **4,848 B** under the unchanged 5,280 B cap. Stripped deployment stays **276,072 B**. All 162 output/inflation/alias observations, six failures, 12 recoveries and ten unsigned-guard tests pass. Normal and allocation-checked compilers reproduce identical artifacts. No LLVM dependency is added.

| Actor | Encoder text | Source through final link | Through strip | Stripped executable |
| --- | ---: | ---: | ---: | ---: |
| Prior selected, same compiler | 63,756 B | 2.211203 s | 2.222270 s | 276,072 B |
| Current matcher facts + layout | 63,692 B | 2.214612 s | 2.225813 s | 276,072 B |
| Roc | 57,696 B | 2.983165 s | 2.994082 s | 191,912 B |
| libdeflate | 37,080 B | 0.480880 s | 0.492001 s | 155,752 B |

Eight balanced fresh-process builds per actor include final linking, with fixed support prebuilt for every actor. Mech complete-build time is **0.74294× Roc / 4.61093× C**. It is **1.000656× the same-compiler reference**: a paired **1.453 ms cost**, 90% interval **-0.561–5.621 ms**, 3/8 faster. This is not a demonstrated build-time gain. Do not compare separate cohort medians as a causal improvement.

The encoder is **1.717691× C / 1.103924× Roc**, still **26,612 B** above C. The broader 1× libdeflate code and build-time goal remains unfinished. There is **no 63rd C win**.

Current phase medians: parse **0.707648 s**, semantics **0.214311 s**, artifact **0.006829 s**, encode **0.002540 s**, native **1.218448 s**; final link **0.036247 s**. Catalog work overlaps parsing; independent medians need not add.

Random 4/8 KiB improves roughly 3–4%, and 1 MiB about 2%, against the immediate qualified paired worker. The prior two 1 KiB slowdowns are recovered. There are no established slowdowns and three uncertain comparisons. The current 1 MiB matcher executes **233.15 million instructions**, including **41.02 million branches**, **8.65 million stack stores** and **22.27 million stack loads**.

[Current source recipe, full qualification, complete builds, adverse data and replay](evidence/encoder-size-gap/current-matcher-guard-facts/README.md)

## Previous 63,756 B qualified recipe

`m1-register-spill-pairs` remains available with its own 81/62 full qualification.
Its earlier same-session complete build was 2.219958 s; its current-cohort
reference is 2.211203 s. Those medians belong to different cohorts.
[Previous pairing qualification and evidence](evidence/encoder-size-gap/current-register-spill-pairs/README.md)

## Previous 63,804 B reference and compiler cohorts

The selected compiler recipe is **`m1-sparse-guard-rounds`**, producing the same
qualified **63,804 B** encoder as `m1-preserved-root-registers`: **1.106× Roc**
and **1.721× libdeflate**. The checked entry remains **46,788 B**, peak active
stack **4,848 B**, and stripped deployment **276,072 B**. The maintained
frame-copies control remains 71,192 B.

The newly linked fixed-support benchmark worker is byte-for-byte identical to
`5afce59a…`, which holds **81/81 Roc and 62/81 libdeflate wins in both complete
M1 orders**, covering 74 and 56 distinct contracts. Both orders support all
those wins at the configured precision. This compiler-only change reuses that
exact worker's qualification; it does not claim a new runtime timing campaign.
The prior 63,804 B comparison had no established candidate/control slowdowns
and seven uncertain cases, all retained in its evidence.

| Latest same-session Apple M1 build | Encoder text | Source to final link | Source through strip | Source CPU | Source peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Prior selected compiler | 63,804 B | 2.207729 s | 2.218989 s | 2.229473 s | 392.7 MB |
| Current default, preview excluded | 63,804 B | **2.205288 s** | **2.216334 s** | 2.228903 s | 392.1 MB |
| Roc | 57,696 B | 2.977513 s | 2.988813 s | 2.934162 s | 387.9 MB |
| libdeflate | 37,080 B | 0.480597 s | 0.491707 s | 0.442477 s | 71.6 MB |

Eight balanced fresh builds include final linking and use fixed support
prebuilt for every actor. The paired difference is **2.407 ms saved**, 7/8
faster, with a 90% interval **1.145–6.921 ms saved**. Current build ratios are
**0.74063× Roc** and **4.58750× C**. This prices the normal compiler after the
diagnostic/planning source additions, with both experimental features absent.
It is not an early-emission gain. The first version with dormant preview code
cost +4.325 ms; isolating that code behind `early-region-planning` recovered
the cost. Both cohorts and raw observations are retained.

The guard scheduler's earlier paired **33.998 ms saving** (8/8 faster) belongs
to its own cohort. The effects are not added across cohorts. Its source and
adverse eager-index prototype remain in the [sparse-rounds evidence](evidence/encoder-size-gap/sparse-guard-rounds/README.md).

Current phase medians are **0.707378 s parsing**, **0.213160 s source semantics**,
**0.006847 s artifact creation**, **0.002528 s encoding**, and **1.212155 s
native emission**. Catalog join wait is 19 microseconds; its work still runs
during parsing. Final link takes **0.035383 s**, strip **0.010892 s**.
Independently calculated medians need not add.

The guard scheduler's **29 focused tests pass**, including both closure schedules against the
original map reference through updates, joins and exhaustive small-state
expiration. Its original named source rebuild reproduces the object, full metadata and
bytecode and passes **162 output checks, six injected failure cases and 12
recoveries**. That earlier compiler source is `0fb8fec139`, binary `e7bb1050…`. Runtime
qualification belongs to fixed-support worker `5afce59a…`; the separately
rebuilt host-support worker `a3325554…` has its own identity and no runtime-win
claim. The original Mech source and all runtime admission/accounting behavior
are unchanged.

The small-controller architecture is still incomplete. A **46,788 B checked
controller** remains, exceeding C's entire encoder by 9,708 B. The remaining
total code gap is **26,724 B**; the latest cohort's median C build-time gap is
**1.724691 s**. The goal remains 1× libdeflate for code and build time, with
at least 81 Roc and 62 C wins and eventually more C wins.

[Selected compiler source, raw builds, exact worker, source checks and verifier](evidence/encoder-size-gap/sparse-guard-rounds/README.md) · [63,804 B runtime qualification and earlier compiler cohort](evidence/encoder-size-gap/preserved-root-registers/README.md) · [Earlier literal-frame 64,444 B option](evidence/encoder-size-gap/shared-literal-frame-cleanup/README.md)

## History review and runtime priorities

The audit reviews 19 historical mechanisms and profiles the exact prior selected
63,804 B Mech/C workers on all 19 remaining named C losses, covering 18 distinct
contracts in 36 useful captures. Large random inputs expose the same hot
collector: it has 72.5% of Mech's encoder samples on 1 MiB, versus 13.2% in
scoring. The qualified excess there is 7.652 ms per call; natural 4 KiB's is
25.8 microseconds. These sample shares are not elapsed phase times.

The audit reference's 1 MiB collector executes 245.0 million instructions and
25.95 million stack-load instructions. Pairing qualified the 63,756 B
predecessor. Its combination with current guard facts and fresh layout then
qualified the selected 63,692 B worker above: 233.15 million instructions,
41.02 million branches, 8.65 million stack stores and 22.27 million stack loads.
The full-size and full-build claims belong to their individual cohorts.

Prioritize the shared random-input collector path and keep natural 4 KiB as
the next-win check. Existing integrated mechanisms are distinguished from
untried repairs. The review snapshots the old worker; the fresh paired recipe
and current build/size figures are recorded above.

[History, all-loss profiles, remaining costs and next experiments](evidence/history-and-profiles-20261119/README.md)

## Natural 4 KiB runtime experiment

The opt-in vector address continuation is now implemented in compiler source.
Its disabled path reproduces the selected object, full metadata and bytecode.
Pointer-only emits **63,820 B** and measures **0.99969× / 0.99293×** the selected
worker's time on `prefix-natural-4096`, in a fresh single-case paired probe.
The forward interval overlaps parity; both orders still lose to C at
**1.06269× / 1.06941×**. This is not a 63rd qualified win or a size win.

Both pointer variants pass 162 outputs, six checked failures, 12 recoveries and
the stack check at 4,848 B. Full runtime qualification and matched complete
build timing are not performed for these unselected workers. Keep the variants
for repair: ending-pointer transport currently adds a remainder-address
calculation inside each vector iteration. The new evidence also retains the
delayed-packing attempt after an exact control-graph layout repair.

[Source, all timings, bounds, adverse attempts and portable replay](evidence/natural4k-runtime-target/README.md)

## Actual early emission experiment

The new `early-region-emission` feature builds the 885-operation codebook unit
before root SSA. Its word/effect proof includes guarded numeric bypasses,
conditional cleanup and copied owner state, while preserving one global source
ownership model and original failure origins. It adds no boundary retain/release
operations. The feature is off by default; the selected artifacts reproduce
exactly when the split is disabled.

Total constructed SSA slots fall **1,407,099 → 1,145,776 (18.6%)**, instructions
**497,878 → 360,196 (27.7%)**, and blocks **174,834 → 128,800 (26.3%)**.
The maximum SSA arena falls **831,378 → 692,257**. Counts include all helpers,
copies and compaction tables; they exclude proof tables and allocator capacity.

**This is not selected.** Encoder text is **69,828 B**, and peak stack is
**5,808 B**, above the unchanged 5,280 B ceiling. The actual worker passes 162
output/alias/decompression observations, six failure cases and 12 recoveries;
17 focused tests and nine native scalar cases pass. Full runtime qualification
has not been performed for this worker, so it has no 81/62 claim.

| New ten-balanced-build M1 cohort | Text | Source through final link | Stripped executable |
| --- | ---: | ---: | ---: |
| Selected reference compiler | 63,804 B | 2.209673 s | 276,072 B |
| Same usable policies, unsplit | 67,364 B | 2.212749 s | 276,056 B |
| Early codebook experiment | 69,828 B | 2.277083 s | 292,568 B |
| Roc | 57,696 B | 2.983069 s | 191,912 B |
| libdeflate | 37,080 B | 0.481339 s | 155,752 B |

The early split costs a paired **68.174 ms** versus the selected reference,
with a 90% interval **59.898–71.740 ms**, and is slower in all ten pairs.
Its native phase is 1.278339 s versus 1.212840 s selected. The earlier
2.205288-second cohort above uses the same selected compiler; results from
separate cohorts are not added or treated as a source regression.

Root-numbered late windows are excluded pending remapping. Even against the
same-policy control, the early split grows the whole encoder by **2,464 B**:
5,232 B removed from the root, plus a 7,348 B helper and 348 B elsewhere/alignment.
Both the construction reduction and these adverse results are retained for repair.

The closest remaining C loss on the **qualified** worker is
**`prefix-natural-4096`**: paired Mech/C **1.04960× / 1.07280×**, requiring
about **6.8% less Mech time** to win both orders and reach 63/81. The next
closest case needs about 25.6%. The ranking uses the exact qualified summaries,
not the early experiment.

[Actual early source, counters, complete builds, contract evidence, earlier attempts and replay](evidence/early-codebook-emission/README.md) · [Ranked remaining C losses](evidence/early-codebook-emission/runtime-targets/nearest.json) · [Historical source planning baseline](evidence/early-codebook-planning/README.md)

## Qualified checked codebook phase alternative

The selected result above stays **63,804 B**. A new late checked-phase split
emits **64,444 B**, a different object from the older qualified literal-frame
recipe of the same size. Its controller is 41,336 B and its extracted phase is
6,008 B. The exact linked workers pass 162 output/alias/decompression checks and
six failure cases, and measured peak native stack falls 4,848 → 4,688 B.

In eight matched fresh builds, source through final link is **2.245295 →
2.231624 s**. The paired saving is **14.000 ms**, 7/8 faster, with a 90% interval
of **12.710–18.280 ms saved**. Roc takes 2.997875 s and C 0.483540 s in this
cohort; the experiment takes 0.74597× Roc and 4.62165× C time. Encoder text is
1.117× Roc and 1.738× C. This is a measured build/stack gain with a 640 B code
cost in its earlier, pre-sparse-rounds compiler cohort. The 14 ms and newer
34 ms effects have not been measured together and are not added. It independently holds **81/81 Roc and 62/81 libdeflate in both complete
orders**, all supported at the configured precision, with no lost peer wins.
V3 returns Retain. There are no established candidate/control slowdowns and
eight uncertain timing cases. All 15 focused checked-region JIT tests pass.
The named `m1-checked-codebook-phase` source rebuild reproduces all artifacts
and passes 162 success observations, six failure cases and 12 recoveries.
The freshly built host-support worker has a separate identity from the
runtime-qualified fixed-support worker. This is a **qualified alternative,
not the selected smaller recipe**. The global plan is still constructed, so
this late split does not finish the small-controller architecture.

[Source, adverse variants, replay, raw builds and verification](evidence/encoder-size-gap/checked-codebook-phase/README.md)

CI at source head `c697f100f8` reports 40 successful jobs, including
native plan, public CLI, hosted applications,
same-binary policies, GPU owners, Windows, Linux and browser checks. Static
architecture contracts still fail on raw native calls in the archived M3 ABI
oracle outside the exact allowlist; the aggregate Static contracts and PR gate
therefore fail too. One dependent job is skipped. This is not an all-green CI result; the
audit policy is unchanged. [Source-head CI](https://github.com/mech-lang/mech/actions/runs/44427678031)

## Apple M3 shared M1 recipe

The integrated compiler's `m1-shared-heap-layout` recipe records **81/81 Roc and 61/81 libdeflate 1.26 level 12 wins in both complete M3 orders**. Raw-record replay verifies all **1,965,312 timed calls**. The **64,652 B** encoder object, complete metadata and canonical bytecode reproduce the named M1 recipe exactly. Peak native stack is **4,784 B**, below the unchanged 5,280 B limit. Both full orders completed without background-activity rejection.

[Complete shared-build qualification and source pins](evidence/apple-m3-win-recovery-20261118/integration/confirmation/README.md) · [Every timing in seconds and comparator multiples](evidence/apple-m3-win-recovery-20261118/integration/confirmation/timings.csv)

## Apple M3 earlier upstream integration

The compiler changes are rebased onto `ca0c778e58618c72e6bd13e7df03d3089d4ec2e3`. The combined encoder contains **63,868 B** of text and uses **4,768 B** peak native stack. In the fresh complete comparison, both the previous and rebased M3 executables record **81/81 Roc in both orders, 58/81 libdeflate forward and 60/81 reverse**. The original upstream deployment recipe reproduces its encoder object, complete metadata and bytecode byte for byte with the combined compiler.

[Complete integration results and reproduction](evidence/apple-m3-rebase-20261118/README.md) · [All timings in seconds and comparator multiples](evidence/apple-m3-rebase-20261118/timings.csv)

The shared compiler contains the M1 heap-setup work and the M3 literal-allocation setup pass. The common M1 recipe is now qualified on M3 as recorded above. Five additional combined builds remain experimental and untimed. [Shared-source verification](evidence/apple-m3-win-recovery-20261118/integration/README.md) · [Recovery trials](evidence/apple-m3-win-recovery-20261118/README.md)

## Apple M3 comparison before upstream integration

The retained word-scan and current-guard optimization records **81/81 against Roc and 61/81 against libdeflate 1.26 level 12 in both orders** in its [complete M3 confirmation](evidence/apple-m3-compact-packets-20261118/word-scans/confirmation/README.md). Encoder text is **64,512 B** and native stack high-water is **4,736 B**, below the unchanged 5,280 B bound. The original Mech source, bytecode, runtime, allocator and peer executables are unchanged.

[All 81 timings, both orders, in seconds and comparator multiples](evidence/apple-m3-compact-packets-20261118/word-scans/confirmation/timings.csv) · [Source and reproduction recipe](evidence/apple-m3-compact-packets-20261118/word-scans/README.md)

## Earlier source-built Apple M1 references

PR #995's integrated compiler at `90d133253a` builds the three named M1 recipes. Each reproduces its archived encoder object, canonical bytecode and complete metadata. The fixed link inputs reproduce all three benchmark and stripped deployment executables. The named command rebuilds its host support and verifies each new worker using 162 success observations, six checked failure cases and 12 exact recovery calls. [Source-built commands](README.md#source-built-m1-reference-encoders) · [Identities and checks](evidence/m1-source-built-20261118/result.json) · [Exact archive worker links](evidence/m1-source-built-20261118/archive-workers-rebased.json)

| Apple M1 build | Encoder text | Source to link | Source to strip | Source CPU | Source peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Mech frame-copies control | 71,192 B | 2.321 s | 2.332 s | 2.361 s | 394.1 MB |
| Mech checked regions | 65,516 B | 2.172 s | 2.183 s | 2.204 s | 392.7 MB |
| Mech compact match equality | 65,276 B | 2.173 s | 2.184 s | 2.205 s | 392.7 MB |
| Mech shared release setup | 65,036 B | 2.176 s | 2.187 s | 2.207 s | 392.7 MB |
| Roc | 57,696 B | 2.983 s | 2.994 s | 2.941 s | 392.9 MB |
| libdeflate | 37,080 B | 0.480 s | 0.491 s | 0.442 s | 71.5 MB |

Twelve balanced fresh builds per recipe use prebuilt deployment support. Shared release setup takes **0.729× Roc** and **4.532× libdeflate** time. It saves a paired median **145.4 ms** against frame-copies. Its encoder text is **1.127× Roc** and **1.754× libdeflate**. The stripped executable is 276,056 B and peak native stack is 4,784 B. [Raw trials and compiler phases](evidence/m1-source-built-20261118/matched-rebased-observations.json.gz)

Shared release setup spends 0.706 s parsing, 0.217 s in source semantics, 0.009 s creating and encoding the artifact and 1.179 s in native emission. Exposed catalog wait is 18 µs. The remaining 0.062 s includes 0.035 s of final link. The complete build is 0.176 s above 2.0 s, 1.176 s above 1.0 s and 1.726 s above 0.45 s. [Phase accounting](evidence/m1-source-built-20261118/result.json)

Frame-copies remains the maintained control. All three named recipes remain opt-in. Each exact archived worker records 81/81 Roc and 62/81 libdeflate wins in each full order, covering 74/74 and 56/74 distinct contracts. The fresh comparison with frame-copies found four cases with established slowdowns in both orders for checked regions and six for compact match equality. Shared release setup still needs a direct runtime comparison with frame-copies. [Runtime qualification](evidence/m1-source-built-20261118/result.json) · [Paired intervals and matcher diagnosis](evidence/m1-source-built-20261118/maintained-runtime/README.md)

The 64,668 B shared-heap trial missed one Roc win in the forward sweep.
Its **64,652 B** successor passes both full M1 sweeps at **81/81 Roc and
62/81 libdeflate**. The previously lost case runs at 0.969× Roc forward
and 0.967× reverse. A fresh source-to-final-link build in the recovery
cohort takes **2.173 s**, or **0.728× Roc** and **4.522× libdeflate**.
Both heap recipes are opt-in. [Original trial](evidence/encoder-size-gap/shared-heap-call-screen/README.md) · [Smaller qualified worker and build timings](evidence/encoder-size-gap/shared-heap-layout-recovery/README.md)

The current native profile attributes the largest traced intervals to root CFG cleanup and phase construction. Retirement-transition caching reuses 3.1% of transitions and adds a paired median 4.7 ms to the complete build. It stays experimental. [Native spans and paired prototype results](evidence/m1-source-built-20261118/native-attribution/README.md)

CI at published head `72691d0265` passes Impact and the GPU owner suite. Its static architecture check fails on the archived M3 ABI oracle's raw native calls outside the exact allowlist; the full log confirms that specific failure. Other native qualification jobs were still running when recorded. The audit policy is unchanged. [Current published-head CI](https://github.com/mech-lang/mech/actions/runs/44400014842). The earlier [3fcb5bf4f8 run](https://github.com/mech-lang/mech/actions/runs/44393821950) passed public CLI, hosted applications, same-binary policies, Linux, Windows and browser standard checks. The new combined-validation source awaits its own CI run.

## Earlier shared-release matched deployment

PR #993 tracks the qualified LLVM-free MechDeflate build and is stacked on #973. The measurements below use Apple M1, macOS 15.6.1 and arm64.

| Apple M1 | Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Source through final link | 2.175 s | 2.977 s | 0.480 s |
| Source through strip | 2.186 s | 2.988 s | 0.490 s |
| Encoder text | 65,036 B | 57,696 B | 37,080 B |
| Stripped executable | 276,056 B | 191,912 B | 155,752 B |

Mech builds in **0.732× Roc** and **4.553× libdeflate** time. Encoder text is **1.127× Roc** and **1.754× libdeflate**. Full orders record 81/81 and 81/81 Roc wins, with 62/81 and 62/81 libdeflate wins.

[Matched measurements and source replay](evidence/encoder-size-gap/shared-release-setup/README.md) · [Scorecard](scorecard.json) · [Policy v3](qualification-policy-v3.json)

An earlier dedicated parser-phase comparison reported **813 ms Mech**
and **3 ms Roc**. Mech's interval includes indexing and its strict check.
[Earlier Mech and Roc parser-phase observations](evidence/sealed-ascii-graphemes/README.md)

## Earlier encoder code-size studies

In the earlier 65,612 B checked-address build, the checked entry accounts for **48,220 B**. The match collector adds **6,224 B**, and the dense scorer adds **1,104 B**. The matched libdeflate object emits **37,080 B**.
[Qualified encoder](evidence/encoder-size-gap/select-checked-default-address/README.md) · [Historical byte attribution](evidence/encoder-size-gap/README.md)

An experimental capacity refusal reuses the checked continuation and
emits **69,528 B**. Both focused orders preserve their peer wins.
The candidate's peak stack is **5,296 B** and natural 8 KiB still loses
to libdeflate.
[Source patch and measured results](evidence/encoder-size-gap/shared-growth-refusal/README.md)

Collector-only packet packing and the shared growth fallback emit
**68,840 B**, with **4,624 B** measured peak stack. Both focused
orders preserve their peer wins. Natural 8 KiB measures **1.00064×**
and **1.00262× libdeflate**. The maintained qualification remains
on the 71,192 B worker.
[Compiler patch, exact replay and comparisons](evidence/encoder-size-gap/scoped-phase-packets/README.md)

Reusing the checked continuations for two more geometry refusals emits
**68,136 B** with **4,768 B** peak stack. The complete orders measured
**81/81 and 81/81 Roc wins** and **61/81 and 61/81 libdeflate wins**.
Natural 8 KiB measured **1.00077×** and **1.00198× libdeflate**.
[Exact replay and complete comparisons](evidence/encoder-size-gap/checked-layout-refusals/README.md)

Keeping the sorter interfaces and trimming other phase results emits
**67,464 B**. In the complete orders it recorded
**81/81 and 81/81 Roc wins** and
**61/81 and 61/81 libdeflate wins**.
[Source replay and results](evidence/encoder-size-gap/checked-layout-refusals/README.md#preserve-the-private-phase-output-interface)

Forwarding 1,837 empty source jumps removes 1,568 blocks before native layout. The exact paired build drops from **2.427 s to 2.387 s**.
The object emits **67,656 B** with **4,672 B** peak stack. In the
complete orders it records **81/81 and 81/81 Roc wins**
and **61/81 and 61/81 libdeflate wins**.
[Compiler patch, build measurements and complete runtime results](evidence/encoder-size-gap/empty-source-routes/README.md)

The dominating-definition experiments emitted an encoder as small as **67,272 B**. The fastest matched source-to-link time was **2.305 s**.
Scalar and non-owning field definitions measured 81/81 and 81/81 Roc wins and 62/81 and 62/81 libdeflate wins in complete orders. The v3 runtime decision was **Retain**.
[Source replay and measured results](evidence/encoder-size-gap/dominating-definitions/README.md)

Keeping the checked returns available in the caller restores two shared continuation sites and emits **67,672 B**.
The 67,664 B encoder won 81/81 and 81/81 Roc cases and 62/81 and 61/81 libdeflate cases in complete orders. V3 returned **Reject**.
[Compiler replay, size and runtime measurements](evidence/encoder-size-gap/shared-continuation-returns/README.md)

The matcher layout emits **67,648 B** with the shared continuations. Complete orders recorded 81/81 and 81/81 against Roc and 62/81 and 62/81 against libdeflate. V3 returned **Retain**.
[Source replay and runtime results](evidence/encoder-size-gap/continuation-matcher-layout/README.md)

The single dense scorer with a checked fallback emits **66,240 B**. It won 81/81 and 81/81 against Roc and 62/81 and 61/81 against libdeflate. The v3 decision was **Reject**.
[Source patch, replay and complete results](evidence/encoder-size-gap/dense-scorer-only/README.md)

Preparing the admitted scorer first skips two unused native bodies and reproduces the qualified 67,648 B encoder object and worker. The source-to-link median is **2.283 s**.
[Source replay and matched builds](evidence/encoder-size-gap/early-scorer-preparation/README.md)

The dense scorer with direct branches emits **66,192 B**. Complete orders recorded 81/81 and 81/81 Roc wins, with 62/81 and 62/81 libdeflate wins. V3 returned **Retain**.
[Source replay and complete observations](evidence/encoder-size-gap/dense-scorer-direct-branches/README.md)

Folding raw grapheme frames and replaying longer statements builds the exact qualified 66,192 B encoder in **2.195 s** through final link.
[Parser source and matched evidence](evidence/sealed-grapheme-and-statement-replay/README.md)

Sharing checked byte reads emits **65,932 B** and builds in **2.198 s** through final link. Both full orders retained 81/81 Roc wins and the 62/81 libdeflate floor.
[Source replay and benchmarks](evidence/encoder-size-gap/shared-defaulted-byte-reads/README.md)

## MechDeflate compile time by encoder build

Each row measures fresh MechDeflate source through final executable
link with the same fixed deployment support prebuilt. Ratios use
Roc and libdeflate trials from the same session.

| Build | Encoder text | MechDeflate build | × Roc | × libdeflate |
| --- | ---: | ---: | ---: | ---: |
| [Collector-only packets](evidence/encoder-size-gap/scoped-phase-packets/deployment.json) | 68,840 B | 2.434 s | 0.814× | 5.046× |
| [Shared layout refusals](evidence/encoder-size-gap/checked-layout-refusals/deployment.json) | 68,136 B | 2.427 s | 0.814× | 5.053× |
| [One-body preparation](evidence/encoder-size-gap/checked-layout-refusals/single-body-preparation/summary.json) | 68,136 B | 2.421 s | 0.811× | 5.046× |
| [All-phase output pruning](evidence/encoder-size-gap/checked-layout-refusals/single-body-preparation/summary.json) | 67,512 B | 2.426 s | 0.814× | 5.059× |
| [Private sorter interfaces](evidence/encoder-size-gap/checked-layout-refusals/non-private-outputs/deployment.json) | 67,464 B | 2.434 s | 0.813× | 5.051× |
| [Forwarding disabled (control)](evidence/encoder-size-gap/empty-source-routes/deployment.json) | 67,464 B | 2.427 s | 0.813× | 5.032× |
| [Forward empty source jumps](evidence/encoder-size-gap/empty-source-routes/deployment.json) | 67,656 B | 2.387 s | 0.798× | 4.953× |
| [Register-preserving free wrapper](evidence/encoder-size-gap/empty-source-routes/deployment.json) | 67,992 B | 2.447 s | 0.817× | 5.060× |
| [Scalar definitions](evidence/encoder-size-gap/dominating-definitions/scalar/deployment.json) | 67,576 B | 2.422 s | 0.810× | 5.034× |
| [Scalar definitions with forwarded jumps](evidence/encoder-size-gap/dominating-definitions/scalar/deployment.json) | 67,272 B | 2.375 s | 0.794× | 4.939× |
| [Non-owning field definitions](evidence/encoder-size-gap/dominating-definitions/fields/deployment.json) | 68,488 B | 2.323 s | 0.777× | 4.834× |
| [Scalar and non-owning field definitions](evidence/encoder-size-gap/dominating-definitions/fields/deployment.json) | 68,424 B | 2.305 s | 0.771× | 4.787× |
| [Shared cold returns](evidence/encoder-size-gap/shared-continuation-returns/deployment.json) | 67,672 B | 2.304 s | 0.770× | 4.796× |
| [Shared cold returns with one additional continuation family](evidence/encoder-size-gap/shared-continuation-returns/deployment.json) | 67,664 B | 2.302 s | 0.770× | 4.794× |
| [Shared continuations with matcher fallthroughs](evidence/encoder-size-gap/continuation-matcher-layout/deployment.json) | 67,648 B | 2.301 s | 0.771× | 4.789× |
| [Dense scorer with early admission and single-body preparation](evidence/encoder-size-gap/dense-scorer-only/deployment.json) | 66,240 B | 2.295 s | 0.768× | 4.699× |
| [Prepare the admitted scorer first](evidence/encoder-size-gap/early-scorer-preparation/deployment.json) | 67,648 B | 2.283 s | 0.764× | 4.757× |
| [Checked dense scorer with direct branches](evidence/encoder-size-gap/dense-scorer-direct-branches/deployment.json) | 66,192 B | 2.274 s | 0.762× | 4.721× |
| [Folded grapheme frames and longer statement replay](evidence/sealed-grapheme-and-statement-replay/deployment.json) | 66,192 B | 2.195 s | 0.737× | 4.578× |
| [Shared checked byte reads](evidence/encoder-size-gap/shared-defaulted-byte-reads/deployment.json) | 65,932 B | 2.198 s | 0.737× | 4.587× |
| [Select the checked default address](evidence/encoder-size-gap/select-checked-default-address/deployment.json) | 65,612 B | 2.205 s | 0.740× | 4.588× |
| [Forward match collector spills](evidence/encoder-size-gap/matcher-spill-forwarding/deployment.json) | 65,516 B | 2.201 s | 0.740× | 4.586× |
| [Index checked region block order](evidence/encoder-size-gap/indexed-checked-regions/deployment.json) | 65,516 B | 2.190 s | 0.735× | 4.577× |
| [Use 16-byte equality groups in the compact matcher](evidence/encoder-size-gap/compact-match-equality/deployment.json) | 65,276 B | 2.184 s | 0.736× | 4.573× |
| [Skip empty parser transactions for resolved terminal misses](evidence/encoder-size-gap/empty-terminal-rewinds/deployment.json) | 65,276 B | 2.182 s | 0.733× | 4.545× |
| [Share owner-release call setup in the checked entry](evidence/encoder-size-gap/shared-release-setup/deployment.json) | 65,036 B | 2.175 s | 0.732× | 4.553× |

Parser, lowering, native compilation and link timings are in the
machine-readable scorecard.

## Earlier shared-release build budget

| Stage | Current | 2.0 s budget | 1.0 s budget | 0.45 s budget |
| --- | ---: | ---: | ---: | ---: |
| Parser | 0.707 s | 0.75 | 0.35 | 0.13 |
| Native preparation through object | 1.183 s | 0.95 | 0.48 | 0.22 |
| Source semantic frontend | 0.213 s | 0.18 | 0.09 | 0.04 |
| Catalog join wait | 0.000 s | 0.05 | 0.02 | 0.01 |
| Artifact, encoding, link and other work | 0.073 s | 0.07 | 0.06 | 0.05 |
| **Total** | **2.175 s** | 2.00 | 1.00 | 0.45 |

That shared-release build needs **1.725 s** less elapsed time to reach the
0.45 s working budget. The phase medians come from the matched measurements above. Catalog work runs during parsing and the table lists the join wait.

## Which build to use

The 64,668 B trial lost one Roc case in the forward sweep. Its V3 result is Reject. [Runtime, timing and replay](evidence/encoder-size-gap/shared-heap-call-screen/README.md)

| Role | Encoder text | Status |
| --- | ---: | --- |
| Historical rebuild default | 72,520 B | `rebuild.py --variant baseline`, compiler opt level 1 |
| Fixed comparison reference | **71,768 B** | Opt-in recipe below and exact linked worker `674d88602946…` |
| Checked-read size candidate | 71,420 B | 5,328 B peak stack. Runtime qualification pending |
| Earlier owner-exchange size candidate | 71,288 B | Rejected by the v2 runtime gate |
| Maintained frame-copy recipe | **71,192 B** | Compiler source below |
| Selected smallest v3-qualified encoder and faster compiler recipe | **63,804 B** | [Sparse-rounds source recipe and exact qualified worker](evidence/encoder-size-gap/sparse-guard-rounds/README.md) |
| Earlier qualified literal-frame option | 64,444 B | [Source, timings and tradeoffs](evidence/encoder-size-gap/shared-literal-frame-cleanup/README.md) |
| Qualified faster checked-codebook alternative | 64,444 B | [Separate object, source and complete qualification](evidence/encoder-size-gap/checked-codebook-phase/README.md) |

~~~sh
python3 -B benchmarks/mech-deflate/native-profile/native-compact-20261116/rebuild.py \
  /tmp/mech-frame-copies-71192 --variant frame-copies \
  --compiler-opt-level 3 --direct-source --preselect-root-inlining --skip-refused-general-ssa
~~~

The fixed reference object is `7bf9df1aa610ba130490c1092db15fceeace77609c16a60d35ff04e47a48ab7b`.
The frame-copy object is `96696eeed083f0f11b51a8b26a01fe9077735821f65e7f08decea56881c6329b`.
[Frame-copy worker verification](evidence/frame-copy-v2/README.md)
[Exact recipe and build receipts](evidence/refused-general-ssa/README.md)

## Earlier build with the benchmark host

These measurements are pinned to the earlier evidence revision `dae46c686703560230150a93f36c44c4978ffdc7`.
They include the original benchmark executable and its larger host support.
[Earlier source-through-link comparison](evidence/refused-general-ssa/README.md)

| Benchmark dimension | Fixed Mech | Roc | libdeflate |
| --- | ---: | ---: | ---: |
| Historical benchmark source through final link | 3.73777 s | 2.98844 s | 0.48067 s |
| Generated encoder text | 71,768 B | 57,696 B | 37,080 B |
| Complete encoder object | 159,384 B | 79,160 B | 52,128 B |
| Linked executable text | 474,692 B | 150,416 B | 129,900 B |
| Benchmark executable file | 956,056 B | 230,600 B | 210,168 B |
| Compiler peak RSS | 401,498,112 B | 395,739,136 B | 71,499,776 B |

The fixed worker's measured private payload peak is 10,548,878 B. Its
checked root frame is 3,728 B and the full measured stack high-water is
5,280 B. A study that claims stack nonregression should pin its bound
explicitly in its plan. [Reference runtime receipts](evidence/bound-71768-runtime/README.md)

## Runtime and integration

| Campaign | Forward Roc/C | Reverse Roc/C | Runtime gate |
| --- | --- | --- | --- |
| 71,192-byte frame-copy worker vs the fixed 71,768-byte worker | 81/61 | 81/61 | v2 Retain |
| Exact 71,768-byte worker vs 71,960-byte immediate control | 81/61 | 81/61 | v2 runtime Retain on the archived orders. Historical v1 Unresolved |
| Exact 71,768-byte worker as contemporaneous control in owner exchange | 81/61 | 81/60 | Absolute C floor missed in reverse |
| 71,288-byte owner exchange vs the fixed 71,768-byte control | 81/61 | 81/60 | v2 Reject, with one C win lost in reverse |

[Exact reference v2 replay](evidence/reference-v2/README.md)

The reference's historical case intervals and the candidate's slowdown
diagnostics have separate entries in the scorecard. The owner-exchange candidate
loses `random-1024` and gains `320-overlap` in reverse.
[Complete v2 candidate evidence](evidence/owner-exchange-71288-v2/README.md)

The journal module layout is repaired and all syntax unit tests pass. The
Related Posts after-4 binding has static proof and awaits compiled validation.
[Checked after-4 binding](evidence/related-post-linked-binding/checked-after-4/README.md)
The optimized compact path is diagnostic. Finite admission, cumulative
accounting and recovery under the ordinary entry remain required. The bounded control-drift check recovered all four affected periodic wins in
both orders. The earlier transient's cause remains open.
[Fixed-worker sentinel diagnostic](evidence/fixed-reference-drift/README.md)
[Current #993 checks](https://github.com/mech-lang/mech/pull/993/checks) · [Related Posts after-4 failure](https://github.com/mech-lang/mech/actions/runs/44204215687/job/134466751590) · [Local format and contract checks](evidence/pinned-formatter-repair/README.md)
· [Related Posts binding audit](evidence/related-post-linked-binding/README.md)

## Runtime scorer study

The caller-address and layout experiment reduces encoder text by **144 B**
to **71,048 B**, or **1.231× Roc** and
**1.916× libdeflate**. Its full runtime orders
and checked-failure receipts are below. The caller-address candidate increases measured peak stack from **5,280 B to 5,296 B**, above the study's
nonregression bound. Both candidates preserve existing individual wins in their respective
screens. The measured deployment build uses the qualified frame-copy recipe.
The experiment reached the 62-win high-water mark in its forward order.
The v3 policy requires 62/81 in both full orders.

| Scope | Order | Control wins vs Roc/C | Candidate wins vs Roc/C | Control 8 KiB vs C | Candidate 8 KiB vs C |
| --- | --- | ---: | ---: | ---: | ---: |
| Focus | Forward | 15/12 | 15/13 | 1.0293× | 0.9941× |
| Focus | Reverse | 15/12 | 15/12 | 1.0427× | 1.0025× |
| Full | Forward | 81/61 | 81/62 | 1.0336× | 0.9976× |
| Full | Reverse | 81/61 | 81/61 | 1.0418× | 1.0055× |

[Raw runtime observations, checked stack and compiler patch](evidence/caller-address-scorer/README.md)

## Scorer and parser experiments

The direct scorer call arguments emit **71,000 B** and pass the linked
checked-failure probes. Peak stack is **5,296 B**, 16 B above the earlier
bound. Both conditional item-selection variants lost peer wins. The
current deployment recipe uses the qualified 71,192 B worker.
[Scorer timing, counter trace and checked stack](evidence/ordered-scorer-visit-costs/README.md)

The parser shortcut preserves all three native artifact hashes and passes
537 syntax tests. Its paired standalone parser saving is
**28.5 ms**.
[Parser shortcut and matched final-link results](evidence/sealed-ascii-literal-miss/README.md)

## Ordered reduction follow-up

A bottom-tested vector loop shrinks encoder text to **71,048 B**
and wins natural 8 KiB in both focused orders without losing a peer case.
The candidate wins 81/81 against Roc in both full orders.
It wins **61/81 and 61/81 against libdeflate**, with natural 8 KiB at
**0.99779× and 1.00436×**.

Restoring the qualified root layout emits **71,096 B**.
The candidate wins 81/81 against Roc in both full orders.
It wins **62/81 and 61/81 against libdeflate**, with natural 8 KiB at
**0.99286× and 0.99866×**.

Its reverse order misses `320-overlap` in both Mech slots.
The linked checks cover all 162 exact outcomes, six checked failures and
a measured peak stack of **5,280 B**.
[Bottom-tested loop build and observations](evidence/ordered-vector-latch/README.md)

The preceding vector-cursor screens eliminated repeated address or lane
work. They established an extra 43,020 scorer stack loads on natural
8 KiB for either address-carrying version. Lane-position deferral alone
kept 5,280 B peak stack and preserved its focused peer wins.
[Vector ablation and counts](evidence/ordered-vector-cursor-ablations/README.md)

## Native compile-time attribution

Incremental branch guard propagation emits exact artifacts and passes
17 guard tests and 162 linked checks. The matched probe is
**4.58 ms slower** than the
maintained compiler through final link. Four compiler profiles put
SSA construction, cleanup and constant-phi work among the larger remaining
native costs. Reusing Tail graph discovery saves **0.19 ms** in the trace.
[Guard propagation](evidence/native-guard-edge-closure/README.md) ·
[Compiler samples](evidence/ordered-vector-cursor-ablations/compiler-sample/)

## Overlap backing and SSA storage

The 32-page word mapping threshold takes about 0.5% longer on
`320-overlap` in both focused orders. Natural 8 KiB is **1.0047×**
libdeflate in reverse. The checked root and its runtime calls
account for about 59% of the sampled native encoder on `320-overlap`.
[Overlap profile and timings](evidence/runtime-overlap-backing/README.md)

The exact bottom-tested scorer repeated at **62/81 and 60/81**
against libdeflate in the complete orders. [Full repeat](evidence/runtime-overlap-backing/root-confirmation/README.md)

Inline single-definition storage measured a paired saving of
**5.88 ms** through final link with exact outputs.
The 90% interval is **0.86 to 17.41 ms**.
[SSA results](evidence/ssa-local-definitions/README.md)

## Short scorer visits

Scalar pairing and earlier four-lane entry use the same admitted windows and
preserve strict winner order. The linked validation covers the complete
81-case fixture set and six checked exits. The 71,192 B frame-copy encoder remains the maintained build. The next runtime mechanism is the repeated bound reload.
[Short-visit results](evidence/ordered-scorer-short-visits/README.md)

## Scalar entry literals

The checked root bypasses provisional SSA lookups for **1,389**
immutable scalar literal lanes in the measured build.
A paired **40.63 ms** saving brings the matched M1 build to **2.495 s** from source through final link.
The 90% interval is **33.08 to 47.17 ms**.
The frame-copy recipe uses this path.
[Exact build and paired measurements](evidence/immutable-root-literals/README.md)

## Scorer cost length

The shorter scorer uses the caller's admitted minimum and preserves
the full-length check for larger offsets. Natural 8 KiB falls from
**43,074 to 54** stack loads in the placed scorer. The **71,128 B**
encoder wins the forward 8 KiB comparison at **0.99426× libdeflate**
and finishes at **1.00101×** in reverse. The focused comparison keeps
both overlap wins and every other peer win. The candidate remains
experimental on the v3 runtime gate.
[Scorer patch, phase counts and paired observations](evidence/scorer-admitted-cost-length/README.md)

## Source catalog overlap

The owned syntax iterator saves **5.87 ms**
in semantic lowering. Its isolated through-link interval spans zero.
Starting catalog construction during parsing brings the matched build
to **2.427 s**. Enabling overlap in the same compiler saves
**60.88 ms** through final link.
The direct frame-copy build uses the measured recipe.
[Catalog overlap and exact build](evidence/inline-syntax-child-traversal/catalog/README.md)

## Active work

- **Native compiler.** The scalar entry literal comparison is complete.
  Price the remaining repeated SSA work with the root-lane census.
  The bounded freshly built and post-phi graph reuse is complete.
  [Current SSA census](evidence/source-and-ssa-attribution/README.md) ·
  [Verified graph reuse](evidence/verified-flowgraphs/README.md)
- **Parser.** Reprice only the remaining outer identifier-choice and frame
  work after the literal-mismatch shortcut. Bound the failure-prefix
  experiment with the current parser and preserve parser event and allowance
  boundaries. [Earlier grammar census](evidence/verified-flowgraphs/parser-choice-census/report.json)
- **Runtime and emitted size.** Recover the `320-overlap` win with margin
  while holding natural 8 KiB. Natural 4 KiB is the next additional win.
  Keep both complete orders and checked carrier evidence under v3.
  [Latest scorer observations](evidence/ordered-vector-latch/README.md)
- **Source semantic frontend.** Use the current exclusive capture to
  select one measured lowering mechanism. Catalog startup, installation
  and validation have separate timings in the same report.
  [Source and catalog attribution](evidence/source-and-ssa-attribution/README.md)

- **Integration and qualification.** Compile and test the checked Related
  Posts producer binding. The 24-block control-drift diagnostic recovers the
  four affected peer wins in both orders. The archived reference has a v2
  runtime Retain decision.

Initial diagnosis is bounded to roughly 60–90 minutes and the first
implementation to half a day. Park a direction after two flat or adverse
iterations unless new evidence changes its expected benefit. Review
the portfolio after two experiments or a substantial decision.

## Completed and parked compiler work

| Area | Decision |
| --- | --- |
| Variable, function and bounded statement replay, including nested observation | Complete |
| Adjacent phi verification reuse | Complete |
| Fresh and post-phi flowgraph reuse | Complete for the bounded cleanup family |
| Inline single SSA definition storage | Retained with a paired 5.9 ms through-link saving |
| Shared immutable facts | Retained and closed at about 1 ms less materialization |
| Declaration snapshots | Closed at about 0.048 ms |
| Minimal deployment without std or JSON | Complete |
| Conservative scalar demand and mixed scalar transport | Parked. The strict model found zero removals. Reopen the 14 protected reads with evidence of material downstream savings |
| Narrow range-pass CFG reuse | Bounded to about 3.2 ms |
| Additional backend workers and initialization-proof rewrite | Deprioritized by worker activity and the 3.8 ms proof ceiling |

[Replay evidence](evidence/scoped-function-replay/README.md) ·
[Verification reuse](evidence/phi-verifier-reuse/README.md) ·
[Native preparation limits](evidence/checked-root-work/README.md) ·
[Minimal deployment](evidence/alloc-only-deployment/README.md)

## Parked work

| Item | Reopen when |
| --- | --- |
| Tiny-case padding or clock-resolution tuning | A measured peer win or material control drift depends on it under v2 |
| Unchanged stale-root batching | A distinct mechanism can account for the prior size and random-data costs |
| Equivalent-vector or cached-join follow-ups | An exclusive elapsed-time profile shows materially removable work |
| Rescreening the smaller historical objects | A specific source change can recover its measured peer losses |
| Additional parser replay families | The post-change profile identifies enough repeated work and conservative admission coverage |
| Wider Related Posts historical probes | The checked producer route is available and each archive identity can be preserved |
| Scorer lowering and table-slot partition selection | A runtime slot reopens after the compiler investigation, with the separate source and causal ablation ready. [Partition evidence](evidence/match-cache-partition-cause/README.md) |

## Sequential scalar repair, not selected

[Source, all sixteen runtime orders and fourteen complete build trials](evidence/encoder-size-gap/scalar-sequential-study/README.md) are published with four portable normal/checked source replays and a passing public verifier. Sequential updates and a proved one-visit odd tail save48B and51,435 natural4KiB instructions versus the earlier paired helper, but still spill more than the selected scorer and lose the existing natural8KiB C win. The **81/63 point-win working selection remains m1-packed-eight-entry,63,868B**. Both partial gains remain available for repair.

The optional parallel low-word carry repair at source89dd67efb passes22 optimized tests in four configurations, including30,720 new loop/multi-edge native calls per configuration, and57 older recipes reproduce exactly. It changes the literal cost recurrence to use the separately carried low cost while the full word is stored. Instruction and branch counts and stack traffic remain unchanged. Its two experimental workers pass output/error/recovery/stack and normal/checked gates; no whole-call speed gain or promotion is claimed yet. The new brief prioritizes current matched Mech/C owner and semantic-work traces after this bounded check.
