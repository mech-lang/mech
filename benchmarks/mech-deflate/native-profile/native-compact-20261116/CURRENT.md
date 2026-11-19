# PR #995 current status

**Active objective: earn the 63rd libdeflate runtime win while preserving every previous peer win and all 81 Roc wins.** Qualified selection remains **63,532 B, 81/81 Roc and 62/81 libdeflate**. No new promotion or 63rd win is claimed.

Current measured compiler implementation is **`546af95af3735591a77455932dc48920a8fc071d`**. It adds an opt-in certified vector mismatch handoff and scalar-word strategy retaining saved-prefix machinery where admitted. Original Mech source and canonical bytecode are unchanged; no LLVM dependency is added. Disabled mode reproduces retained artifacts exactly.

The latest nine-candidate, nineteen-case, two-order study puts the **63,388 B word/complement candidate at 1.023495× / 1.033400× C on natural 4 KiB**, saving 144 B and improving that case 2.84–3.61% against its same-run control. It still needs 2.30–3.23% less candidate time for parity. The vector/narrow candidate is **63,628 B**, **1.025877× / 1.039387× C**. Both keep the tight natural-8-KiB, periodic-1-KiB and 320-overlap C wins in discovery. Word/complement slows random 4 KiB 1.49–4.11%; word-reuse/narrow loses 320-overlap forward. All adverse results remain available for repair. No full score is inferred from this panel.

Fresh complete-build medians: retained **2.212654 s**, word/complement **2.214373 s**, vector/narrow **2.213084 s**, Roc **2.991040 s**, C **0.482117 s**. Ten balanced serial fresh-process orders per actor include final linking; compilers/support are prebuilt equally and strip is separate. Retained paired ratios are **0.739292× Roc / 4.588892× C**. Mech stripped executables remain **276,072 B**. No established compilation-time change is claimed.

The package verifies **20 emissions, 18 clean runtime orders, ten complete-build orders and 1,288,960 native comparisons plus 60 refusal comparisons**. All timed candidates pass 162 output/inflation/alias observations, six failures and the stack cap. No new full score or twelve-recovery campaign is claimed. This codec admits no saved interval under the word-reuse scope, so no cache benefit is claimed. The earlier interrupted timing campaign is excluded entirely. [All evidence and portable reproduction](evidence/vector-mismatch-20261119/README.md).

The full GPU test build failed on disk exhaustion before running tests; the exact-module native harness passes. Formatting, module layout and warning policy pass. The repository unsafe audit remains blocked by its pre-existing missing archived allocator file.

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
