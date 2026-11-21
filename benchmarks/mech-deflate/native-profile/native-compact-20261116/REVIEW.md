**Selected: `m1-identical-shared-vector-linear`, 64,408 B, every named 81/81 Roc and 63/81 libdeflate win held in both full 81-case, 96-block orders.** Dominance-proved SSA argument coalescing with shared-Boolean lowering takes **23.506–23.508 ms versus 20.349–20.353 ms C** on random 1 MiB, paired **1.154592–1.155961× C**. About **3.16 ms, or 13.4–13.5% less Mech time**, is still needed for parity. No 64th C win is established. The original 2,719-line Mech source and canonical bytecode are unchanged.

| Qualified arm | Text B | Random 1 MiB Mech ms, forward /reverse | C ms, forward /reverse | Paired Mech ÷ C |
| --- | ---: | ---: | ---: | ---: |
| New default, shared-vector-linear | 64,408 | 23.506125 /23.507917 | 20.353292 /20.348604 | 1.154592 /1.155961× |
| New vector-linear alternative | 64,364 | 23.591333 /23.597542 | 20.348937 /20.350479 | 1.160017 /1.160308× |

The default saves **4 B** versus the preceding64,412-B selection. The alternative saves **48 B** and also retains all 81 Roc/63 Cwins in both complete orders. Default full-suite geometric means are **0.999440× /0.999564×** control; alternative **0.998341× /0.998738×**. These small broad differences are not an established large runtime gain. Small slower cases remain included; point wins and confidence are separate. The previously qualified **63,372-B compact recipe remains available**.

Matched complete builds take **2.199275 s new default /2.192953 s alternative /2.204447 s prior control /2.986321 s Roc /0.481612 s C**. Twelve balanced serial fresh-process orders include final executable linking with compilers/support equally prebuilt; no compile-time gain is claimed. Default paired time is **0.736279× Roc /4.563207× C**. Parse/frontend/native medians are **0.703250 /0.218701 /1.204642 s**. The stripped Mech image remains **276,072 B**; peak stack is **4,832 B**, below 5,280 B. The alternative has 4,736 B peak stack. Text is **1.116× Roc /1.737× C**; stripped image **1.439× Roc /1.772× C**.

The actual main default and explicit alternative reproduce all three artifacts exactly. **34 public normal/checked source replays**, **79 optimized scalar tests**, **7,360 new focused native executions**, all strict output/resource/error/recovery/stack checks and **486 actual timed stripped-image executions** pass. Portable verification checks **10,031,040 saved runtime calls**, including 1,450,135 retained zero samples, **144 actual training counter cases**, both profile derivations and all 12 complete-build orders. Compiler source is rebasable at `10a4b06f8`.

[All17 retained source recipes, timings and tradeoffs](evidence/identical-argument-coalescing/README.md) · [All81 elapsed times and remaining parity gaps for both new arms](evidence/identical-argument-coalescing/PARITY.md).

The next broad investigation is relational unsigned bounds: the existing selected-edge range code narrows variable comparisons only when one operand is constant. A true `x<y` can prove `x<MAX` even with an unknown upper bound, which may remove redundant checked increments. Count the actual remaining hot guards first, preserve width/signedness and boundary errors, and combine proved facts with the new SSA normalization. Constant private input-packet words, shared comparison ANDs, immutable byte-load reuse and the retained compact recipe are also recorded for follow-up. **64 C wins is next; 1× libdeflate across all 81 remains the goal.**

Earlier review checkpoints follow.

**Selected: `m1-argument-all-sqrt`, 64,412 B, every named 81/81 Roc and 63/81 libdeflate win retained in both full 81-case, 96-block orders.** Pure SSA argument transport forwarding with fresh profiles takes **1.49–1.59% less elapsed time across the full suite by geometric mean** than the previous 63,468-B selection. Its measured peak stack is 4,736 B. The **63,372-B compact alternative** also retains every named win in both full orders, saving 96 B and taking 0.87–0.90% less full-suite time; random 1 MiB costs 0.23–0.30% more than its matching control. Both retain the original 2,719-line source and canonical bytecode.

**The nearest remaining loss is random 1 MiB: 23.68–23.69 ms Mech versus 20.34–20.35 ms C, paired 1.164–1.165× C. About 3.34 ms, or 14.1% of Mech time, must still be removed to tie.** No 64th C win is established. Milliseconds below are per-call medians; ratios come from paired observations, so dividing displayed aggregate medians need not reproduce the paired ratio.

| Test | Mech ms, forward /reverse | C ms, forward /reverse | Paired Mech ÷ C | Mech time reduction still needed |
| --- | ---: | ---: | ---: | ---: |
| Random 1 MiB | 23.691354 /23.683562 | 20.353500 /20.343750 | 1.163816 /1.164937× | 14.08% /14.16% |
| Distribution change, 16 KiB | 0.579458 /0.587708 | 0.482479 /0.483042 | 1.200310 /1.209133× | 16.69% /17.30% |
| Random 4 KiB | 0.107542 /0.108750 | 0.076312 /0.076979 | 1.426908 /1.425898× | 29.92% /29.87% |
| Periodic 1 MiB | 12.004729 /12.004251 | 18.465896 /18.409959 | 0.649893 /0.652389× | 0.00% /0.00% |

Latest matched complete builds on **Apple M1** take **2.200294 s Mech /2.984815 s Roc /0.485399 s C**, paired **0.735180× Roc /4.521701× C**. Twelve balanced serial fresh-process orders include final executable link with compilers and fixed support equally prebuilt. Paired time versus the previous compiler/recipe is 0.998819×; **no compile-time gain is established**. Selected parse/frontend/native medians are 0.706155 /0.217352 /1.203106 s. Stripped executables remain 276,072 B Mech /191,912 B Roc /155,752 B C. Selected encoder text is 1.116× Roc /1.737× C; compact text is 1.098× Roc /1.709× C.

Sixteen public normal/checked source replays and both actual main source recipes, including the new main default, reproduce object, full metadata and bytecode exactly. Every candidate passes strict original output/resource/error/recovery checks and the 5,280-B stack cap. The portable verifier checks **6,801,408 saved runtime calls**, 935,509 retained zero samples, **45 actual training counter cases**, both profile derivations, all twelve complete-build orders and **972 saved stripped deployment streams** with independent inflation. The compiler source has **5 SSA regression tests and 79 optimized scalar tests passing**, including direct checked Bool/U64/I64/F64 original/decoded paths. Source implementation is committed and rebasable at `a0bc28d87`; no LLVM dependency or shipping native-byte patch is added.

[All eight source recipes, tradeoffs, raw measurements and verification](evidence/argument-jump-forwarding/README.md) · [All 81 elapsed times and remaining parity gaps](evidence/argument-jump-forwarding/PARITY.md).

The next broad proof investigates masked observation-counter indices and repeated read/write bounds in the actual matcher, reusing existing admission and word-range machinery. Confirm which checks remain before adding a pass; the existing bounded byte-histogram optimization is already enabled. Report elapsed milliseconds, paired ×C and the remaining time reduction for every target case. **64 C wins is next; 1× libdeflate across all 81 runtime cases remains the goal.**

Earlier checkpoints follow; their selected recipes and timing labels apply to those versions.

**Selected remains `m1-root-packet-combined`: 63,468 B, every named 81/81 Roc and 63/81 libdeflate win retained.** Five broader register-frequency/layout experiments reduce actual matcher reloads by up to 46% and instructions by about 4.4%, but do not improve broad elapsed time. Both 21-case discovery orders preserve all observed point wins; all five arms and their slowdowns remain available for later combinations. The selected object/metadata/bytecode and exact worker are unchanged; no new C win or experimental full81 qualification is claimed.

Latest matched complete builds on **Apple M1** take **2.200666 s selected Mech /2.993788 s Roc /0.485272 s C**, paired **0.734807× Roc /4.546587× C**. Sixteen balanced serial fresh-process orders include final executable link with equal prebuilt compilers/support. Experimental Mech medians span 2.198552–2.202191 s; no compile-time gain is established. Selected parse/frontend/native medians are 0.703388 /0.217314 /1.205074 s. Mech stripped images remain 276,072 B; selected text remains 1.100× Roc /1.712× C.

The independent frequency generator fixes assigning hot counts to unvisited blocks, using every static head and the actual code extent. Two layout repairs were also attempted. Ten public normal/checked source rebuilds reproduce all three artifacts exactly; every candidate passes strict output/resource/error/recovery checks and 4,752-B measured peak stack. The portable verifier checks **1,794,240 saved runtime calls**, 293,742 retained zero samples, all 54 actual matcher counter cases, both frequency/layout derivations, sixteen complete-build orders and **648 saved stripped deployment streams**. No Rust changes in this study; the same compiler source has 79 optimized scalar tests passing. [All five retained source recipes, raw results, build phases and verification](evidence/static-register-frequency-study/README.md).

The next broad structural pass composes arguments through jump-only blocks to reduce copied loop state and compilation work. It must preserve escaping SSA uses, per-edge argument permutations, aliases, cold/debug boundaries and efficient checked scalar paths. **64 C wins is next; 81 remains the full goal.**

Earlier checkpoints follow; their timing/default labels apply to those versions.

The source-to-block diagnostic is now integrated, with **79 optimized scalar tests**, eight exact public normal/checked source replays and byte-identical selected artifacts and worker. The selected **63,468 B /81 Roc /63 C** result is unchanged. The latest eight-order matched complete-build cohort measures **2.199818 s Mech /2.988482 s Roc /0.481730 s C**, paired **0.736360× Roc /4.562243× C**. Against the previous compiler the paired ratio is **0.999881×**; no timing change is established. All 324 actual stripped deployment streams pass independent inflation.

On seven of nine mixed inputs, mapped private search-tree and skip-tree step counts match C exactly. The two larger inputs retain a checked-continuation scope remainder. This points the broader investigation toward register reloads, copied loop state and repeated checks per step. Missing/merged source labels and unvisited machine blocks are recorded without inventing counts. [Compiler diagnostic, actual object mappings, raw counts, build phases and portable verification](evidence/semantic-block-map/README.md).

**Working selection: `m1-root-packet-combined`, 63,468 B, preserving every named 81/81 Roc and 63/81 libdeflate win in both full 81-case, 96-block orders.** It combines retained input-packet qualification with generic checked-entry jump cleanup. The original **2,719-line Mech codec and canonical bytecode remain unchanged**. Runtime is effectively flat against the preceding selected build; all named wins survive. Confidence is separate; no 64th C win is established.

Latest matched complete builds on **Apple M1** take **2.199591 s selected Mech**, **2.227699 s preceding Mech**, **2.992954 s Roc**, **0.490219 s C**, including final executable linking with compilers and fixed support equally prebuilt. Twelve balanced serial orders check every artifact. Selected paired time is **0.986513× preceding Mech / 0.734967× Roc / 4.521845× C**. The paired median saving is **30.041 ms**, with a descriptive 90% bootstrap interval **21.707–43.436 ms**. The native pipeline falls **1.232435 → 1.201935 s**. Training, compiler bootstrap and strip are separate.

The combination saves another **48 B** of native text: **1.100× Roc / 1.712× C**. The stripped executable remains **276,072 B**; measured peak stack is **4,752 B**, under the 5,280 B cap. Root cleanup increases the checked entry by 276 B; the packet proof saves 320 B in helpers, and padding yields the 48 B net saving. The **63,196 B** packet-only arm remains available with all native checks and both panel orders; it has no full81 qualification. All three variants and every adverse observation are retained.

Six public normal/checked source replays and three actual main named recipes reproduce object, full metadata and bytecode exactly. The portable verifier checks **3,041,856 saved runtime calls**, every required named win, twelve complete-build orders, the bootstrap derivation and **486 actual stripped deployment streams**. The same compiler feature has **79 optimized scalar tests** passing. No Rust source changes in this combination. [Source recipes, all runtime/build observations, intervals and portable verification](evidence/encoder-size-gap/packet-family-root-study/README.md).

`rebuild.py` now defaults to `m1-root-packet-combined` on Apple ARM64. No LLVM dependency or shipping native-byte patch is added. The next larger investigation compares actual Mech/C matching, skip and tree work, with source-to-code mappings that preserve emitted bytes. **64 C wins is next; 81 C wins remains the full goal.**

Earlier checkpoints follow; their selected/default labels apply to those versions.

**Working selection: `m1-forwarded-register-linear`, 63,516 B, preserving every named 81/81 Roc and 63/81 libdeflate win in both full 81-case, 96-block orders.** Generic forward-jump cleanup with fresh register and matcher/Huffman layout profiles saves another **96 B**, takes **1.25–1.33% less random-1-MiB time** and reduces measured peak stack to **4,720 B**. The original **2,719-line codec and canonical bytecode remain unchanged**. Confidence is separate; no 64th C win is established.

Latest matched complete builds on **Apple M1**: **2.229091 s selected Mech**, **2.230269 s preceding Mech**, **2.988087 s Roc**, **0.486367 s C**, including final executable link with compilers/fixed support equally prebuilt. Ten balanced serial fresh-process orders check all artifacts. Selected paired ratios are **0.745744× Roc / 4.595310× C**; encoder text **1.101× Roc / 1.713× C**; stripped executable **276,072 B**. **No compile-time gain is established.**

The actual selected control and both final matcher candidates have fresh nine-case native work counts. On random 1 MiB, selected instructions fall **198,169,162 → 195,425,468** and stack loads **20,228,175 → 18,128,229**, with 35 calls each. Every native output/resource/error/recovery/stack check passes. Four public normal/checked rebuilds and both main named recipes reproduce all three artifacts exactly. The portable verifier checks **2,683,008 saved runtime calls**, seven source/counter bindings, both register-profile derivations, every named full-suite win, ten complete-build orders and **405 actual stripped deployment streams**. The compiler feature has **79 optimized scalar tests** passing. [Current source recipes, raw results, phases and verification](evidence/encoder-size-gap/forward-jump-register-study/README.md).

The **63,468 B** square-root alternative remains available with both panel orders and all native gates, including its **3.0–3.4% distribution slowdown**; it has no full81 qualification. `rebuild.py` now defaults to `m1-forwarded-register-linear` on Apple ARM64. No LLVM dependency or shipping native-byte patch is added.

Broader work now revisits retained input-packet qualification and checked-entry construction across input families. The checked entry remains **46,572 B**. **64 C wins is next; 81 C wins remains the full goal.**

Earlier checkpoints follow; their selected/default labels apply to those versions.

**Current selection: `m1-profiled-register-sqrt`, 63,612 B, every named 81 Roc / 63 libdeflate win retained in both full 96-block orders.** Original 2,719-line source and bytecode remain unchanged. It takes 3.07–3.14% less time across the full suite by geometric mean, 7.2–7.4% less on random 1 MiB and 3.2–3.5% less on distribution change. No 64th C win is established; confidence is separate.

Source through final executable link takes **2.236726 s**, paired **0.746096× Roc / 4.591963× C** in fourteen balanced orders. Encoder text **1.103× Roc / 1.716× C**; stripped executable **276,072 B**; measured stack **4,736 B**. No compile-time gain is established. Eight public normal/checked rebuilds, both main named recipes, 5,366,016 saved runtime calls and 567 actual stripped executable checks pass. The same compiler feature has all 79 optimized scalar tests passing.

[Current scoreboard and continuation](CURRENT.md) · [Source recipes, full raw results, build phases and portable verifier](evidence/encoder-size-gap/compact-register-weight-study/README.md). The next broad experiment uses generic forward-jump cleanup and fresh register/layout profiles. The 81/63 named-win floor remains; 64 C wins is next, 81 is the full goal.

Earlier review checkpoints follow; their selected/default labels apply to those versions.

**Working selection: `m1-narrow-packet-words`, 63,804 B, 81/81 Roc and 63/81 libdeflate in both complete 96-block orders.** Every required named point win is retained, and all required confidence checks also pass in both directions. This saves 48 B versus the preceding 63,852 B working build and 64 B versus the original eight-entry 63-win build.

A separately pinned **2,732-line Huffman source repair** now retains every **81 Roc / 63 C** named win in both full 96-block orders. It takes **11.4–12.5% less random-4-KiB time** and **10.0–10.8% less random-16-KiB time** than selected. The first all-bucket version lost overlap; the sparse/small-alphabet Shell path recovers it, and U32 counters save 224 B versus the repaired U64 arm. No 64th win is established. The repaired U32 alternative emits **72,196 B**, builds through final executable link in **2.346140 s**, and measures **4,752 B** stack. The smaller default remains selected. Eight public normal/checked source rebuilds are exact; complete raw/adverse/repaired orders, 212-vector C/Mech algorithm replay and the 14-trial matched build cohort are published. [Source, qualification, limits and reproduction](evidence/huffman-sort-replay-study/README.md).

Current setup/retirement diagnostics cover **nine cases, both orders, 72 batches and 3,024 saved calls** around the exact selected and C encoder objects. Natural 1 KiB makes **65 Mech payload requests**, versus one C compressor context plus output; its coarse diagnostic gap is about **14 µs**. Mapping/unmapping alone does not explain that gap. Random 1 MiB records roughly **53–55 µs** in instrumented allocation/retirement methods against a roughly **5.8 ms** whole-call gap, supporting continued matcher work. Per-operation clocks visibly perturb small inputs; their totals are not removable-time budgets. Payload counts exclude owner/registry allocations, and task faults do not quantify clearing traffic or isolated first-touch time. The selected source, artifacts, build time and **81 Roc / 63 C** score remain unchanged. [Raw records, source, explicit limits and portable verification](evidence/setup-retirement-study/README.md).

A separate ordinary-Mech search-call specialization now retains **81/81 Roc and 63/81 C in both complete 96-block orders**, with every required named win. Its composite candidate takes **3.2–6.2% less time on random 4 KiB** and **4.9–5.4% less on distribution change** in those full matrices; no 64th win appears. The isolated source edit costs about 2.4–2.5% on random 1 MiB versus its matched original-source control, so that store/lifetime cost remains a repair target. It has **69,060 B** text and builds in **2.497319 s**, while the selected recipe remains **63,804 B**. Full source, explicit generation recipes, four exact public normal/checked replays, all raw timings and failures are published. [Retained source alternative and complete qualification](evidence/source-search-specialization-study/README.md).


Current attribution now includes **45 fresh owner captures, exact nine-case Mech/C parsing-work alignment, C search counters and 212 real Huffman vectors**, plus current matcher instruction and stack counts. Both Mech scorers match C exactly on passes, node updates, no-match positions, match records and candidate lengths. The opt-in carried-cost candidate takes **2.42–2.51% less time on random 1 MiB** in its two 21-case discovery orders. All observed wins survive, but no 64th win or full 81-case qualification is claimed. The selected 63-win recipe is unchanged. Comparable total matcher/setup/output work, Mech frequency capture, phase replay and aligned coarse phase times remain outstanding. [Current evidence, all raw observations and portable verification](evidence/current-phase-work-study/README.md).


Natural 4 KiB measures **0.981214× / 0.992852× C**. The point-win requirement remains the selection rule; confidence is reported alongside it. The one narrow discovery loss did not discard this candidate; complete 96-block evaluation established all 63 named C wins.

Source through final executable link on Apple M1 takes **2.219190 s**, paired **0.741637× Roc / 4.578823× C**, in the latest fourteen-trial matched cohort. Roc takes **2.986472 s** and C **0.485813 s**. Stripped executable **276,072 B**; peak stack **4,736 B**. Encoder text **1.106× Roc / 1.721× C**. No material compile-time gain is established. All compilers and fixed support are prebuilt equally; final linking is included and stripping/bootstrap are separate. The preceding 18-trial 2.222473 s result remains historical evidence. [Latest complete builds](evidence/huffman-sort-replay-study/README.md).

`rebuild.py` defaults to this recipe on Apple AArch64. Explicit selection: `--variant m1-narrow-packet-words`. Actual named-recipe normal/checked emission reproduces the measured object, full metadata and canonical bytecode exactly. [Current source, both full orders, every named win, confidence bounds and reconstruction](evidence/encoder-size-gap/packed-word-packet-selection/README.md). [Prior twelve-recipe study, tests, gates, counters and build evidence](evidence/encoder-size-gap/packed-word-repair-study/README.md).

Original 2,719-line Mech source and canonical bytecode remain unchanged; no LLVM dependency is added. The next milestone is **64/81 C**, preserving all 63 existing C names and 81/81 Roc; the full goal remains **1× libdeflate across all 81 cases**. The next major diagnostic is a current Mech/C phase-and-work comparison across the remaining losses, as requested in the new brief. Existing unsafe audit still has the unrelated missing archived allocator blocker.

Historical checkpoints follow; their original decisions and observations are retained.

Preceding experiments:

Compiler source **`84628d979e`** restores the historical paired scalar continuation as an opt-in path. It passes **20 optimized tests in four configurations**, including **22,528 new native calls** per configuration, and all **33 older recipes reproduce exactly**. Both main candidates and four register-setting workers pass output/error/stack/recovery gates and portable normal/checked replay. The qualified **63,612 B / 81 Roc / 62 C** default remains selected; the preceding eight-entry candidate still retains 63 C point wins in both full96 orders, with reverse confidence crossing parity.

Pairing increases natural 4 KiB stack loads **17,310 → 105,720** and instructions **3,266,626 → 3,589,068**, despite removing 3,881 branches. Its original head variant costs **1.96–2.44% more natural 4 KiB time** and **4.21–4.86% more natural 8 KiB time** than its held control, losing the old 8 KiB C win in both orders. Existing register settings leave the scorer body and counts unchanged; their four additional orders are preserved. Keep the source and repair the cost/best/item lifetimes. A separate eleven-case census also identifies the literal-only DP route behind random-input losses. [All eight orders, checks, counts, builds and replay](evidence/encoder-size-gap/scalar-pair-revisit-study/README.md).

Head text: **64,108 B**; complete build **2.219618 s**, paired **0.743139× Roc / 4.606172× C**; stripped executable **276,072 B**; peak stack **4,736 B**. No compile-time gain is shown. Next, test sequential updates inside the unrolled pair and optimize the literal-only route separately. These follow-ups are not yet implemented.

Preceding guarded eight-entry checkpoint:

Compiler source **`60e942f462`** adds guarded eight-candidate entry to the paired packed scorer. Head candidate: **63,868 B**, **4,736 B** stack. It removes **66,937 natural-4-KiB scorer instructions and 36,501 branches** versus the sixteen-entry head worker, with unchanged stack traffic. Every original named 81 Roc / 62 C win remains precision-supported in both full96 orders. The new reverse natural-4-KiB interval still crosses parity, so the 63rd C win is not yet qualified. The qualified 63,612 B histogram remains selected until a separate promotion. [All eight runtime orders, confidence results, source, tests, counts, builds and portable replay](evidence/encoder-size-gap/packed-eight-entry-study/README.md).

Forward96: **81 Roc / 63 C point wins**, natural 4 KiB **0.993901× C**, 90% interval **-1.144% to -0.096%**. Reverse96: **81 Roc / 63 C point wins**, natural 4 KiB **0.997675× C**, 90% interval **-0.818% to +0.388%**.

Head complete build: **2.215637 s**, paired **0.742654× Roc / 4.604948× C**, stripped executable **276,072 B**. Nineteen optimized tests pass in three configurations; 31 older recipes and both portable normal/checked artifacts reproduce exactly. No compilation-time gain is established.

Preceding head/item checkpoint:

Compiler source **`2b6ed311d`** keeps the paired scorer and tests head-group processing plus wider item decoding. The head candidate is **63,836 B**, saving **16 B** and **10,308 executed natural-4-KiB scorer instructions**. It measures **0.998497× / 1.000382× C**, with **1.000019× / 0.998029×** time versus the exact paired-carried control. All prior wins present in eight complete 21-case discovery orders survive. No new full score or qualified 63rd win is claimed; keep the saving and continue the paired scorer. The qualified default remains **63,612 B / 81 Roc / 62 C**.

Wide-only and head+wide are exact artifact no-ops for this codec. Combining width changes with the older deferred-item option removes 2,955 instructions, but the deferred family does not establish a latency gain. All six workers pass output/error/stack/recovery gates and portable normal/checked replay. Nineteen optimized tests pass in four configurations; 25 older recipes reproduce all artifacts exactly. Head build is **2.222766 s**, paired **0.744078× Roc / 4.610974× C**, with **276,072 B** stripped executable and **4,736 B** peak stack. No compile-time gain is shown. [Source, all eight distinct-code orders, negative results, counts, checks and complete builds](evidence/encoder-size-gap/packed-margin-repair-study/README.md).

Next, enter at eight only under the packed bounds and retain the original short wide-cost scalar path. This revisits the earlier short-vector work with the new paired scorer; it is not implemented in the linked source epoch.

Preceding paired-accumulator checkpoint:

Preceding compiler source **`31910aff6`** adds two independent packed accumulators over eight candidates. The **63,852 B** carried probe takes **1.50% / 1.59% less natural-4-KiB time** than its matching one-accumulator control. Both full81/24-block orders win **81 Roc / 63 C**. Full96 then retains every original **81 Roc / 62 C** named win with confidence support, but natural 4 KiB is **0.984695× C forward / 1.000362× C reverse**. Its reverse 90% interval spans parity (−0.429% to +0.528%). **Keep the candidate and repair this one margin; no qualified 63rd win is claimed.** The smaller qualified 63,612 B histogram remains selected.

Complete carried-probe build is **2.215453 s**, paired **0.742850× Roc / 4.606302× C**; encoder text is **1.107× Roc / 1.722× C**, stripped executable **276,072 B** and peak stack **4,736 B**. All prior named wins, output/error/recovery/stack gates, three configurations of eighteen optimized tests, 23 exact older recipes and both portable normal/checked emissions pass. No compile-time gain is shown. [Source, all eight runtime orders, both confidence reports, counts, complete builds and replay](evidence/encoder-size-gap/paired-packed-accumulator-study/README.md).

Next, test optional-head peeling and a widened packed ordinal separately and together. This targets redundant loop-bound setup and item-index reconstruction while retaining this exact paired candidate. These repairs are not yet implemented.

Preceding packed-key checkpoint:

Preceding compiler source **`f3d74d948`** integrates bounded packed cost/ordinal keys with the original optimized vector fallback. The packed probe takes **0.89% / 0.88% less natural-4-KiB time** than its matching control, reaching **1.005062× / 1.008491× libdeflate**. No 63rd win is established. Carrying indices removes more instructions; a broader profile repairs its distribution-change slowdown to **0.39% / 0.57% less time** than its matching revised-profile control. All prior peer wins observed in fourteen complete 21-case orders survive; the qualified **63,612 B / 81 Roc / 62 C** recipe remains selected.

The repaired probe is **63,660 B**, **1.103× Roc / 1.717× C encoder text**, with **4,736 B** peak stack. Its complete executable build is **2.221298 s**, paired **0.743629× Roc / 4.605955× C**; the same-cohort qualified build is **2.218823 s**. No compile-time gain is shown. Eighteen optimized tests pass in all three feature configurations; sixteen old recipes and all seven portable normal/checked emissions reproduce exactly. All seven workers pass output/error/stack/recovery gates. [Source, all fourteen raw orders, counts, build profiles, verifier and reproduction](evidence/encoder-size-gap/packed-cost-ordinal-study/README.md).

Next, test two independent packed accumulators over eight candidates to reduce the key/minimum dependency and loop branches. Preserve the existing vector fallback, exact tails, all named peer wins and every adverse observation. This is the next experiment, not an implemented result.

Preceding conditional-winner checkpoint:

Preceding source **`46dc3230f`** completes the conditional winner experiment. The combined word is **62,796 B** and saves about 1% on distribution change against its 62,844 B control, while natural 4 KiB stays near **1.025× / 1.035× C**. The index-only guard is slower. Moving item reconstruction to the same strict-winner edge recovers its 32 B growth and removes 19,427 natural-4-KiB scorer instructions, but still adds 3,550 branches and establishes no latency win. All prior peer wins observed in twenty complete twenty-one-case orders survive; the qualified **63,612 B / 81 Roc / 62 C** result remains selected.

The mapped word is **62,796 B**, or **1.088× Roc / 1.694× C encoder text**, and builds through final link in **2.215644 s**, paired **0.741685× Roc / 4.605130× C**. All ten variants pass output/error/stack/recovery and checked replay. Sixteen/seventeen optimized tests and 8,192/16,384 native fixture calls pass. A separate 8,192-call packed-key prototype emits one horizontal minimum instead of two under explicit U16 bounds; it is not codec-integrated or a speed claim. Next, admit its bounds and retain the old vector path on a miss. [All twenty raw orders, source, counts, complete builds, prototype and reproduction](evidence/encoder-size-gap/conditional-winner-item-study/README.md).

Preceding phase-selection checkpoint:

Source **`3fe63cb1d`** allows exact phase selection for the existing packet-load qualifier. Selecting nine complete routine families recovers **320 B** of the original 416 B saving: word **62,844 B**, probe **63,116 B**, both with 4,736 B peak stack. All prior peer wins observed in fourteen complete twenty-one-case discovery orders survive; no full score or win #63 is established. The current all-phase probe still costs 8.80–9.26% on distribution change, while the narrower selection costs 0.11–0.45% in the probe orders and 0.20–0.63% in the word orders. The smaller 62,796 B non-scorer candidate retains its 5.86–6.20% penalty for repair.

The selected-routine word builds through final link in **2.213414 s**, paired **0.741357× Roc / 4.606720× C**. All six new candidates pass outputs/errors, twelve failures/recoveries, stack gates and normal/checked artifact replay. The qualified 63,612 B / 81 Roc / 62 C result is unchanged. Next, combine the word scorer-only improvement with the nine-family saving and isolate the four extra families responsible for the remaining 48 B tradeoff. [All source, fourteen raw orders, exact-mode checks, full build cohort and reproduction](evidence/encoder-size-gap/selected-packet-phase-study/README.md).

Preceding fixed-read checkpoint:

Source **`2dcf8babd`** reuses the selected `window >= 5` edge to admit a four-byte read at `position + 1`, without a new runtime guard or cloned body. Word is **63,164 B**, probe **63,436 B**, and packet-qualified **63,020 B**, all with 4,736 B stack. The probe takes 0.13–0.38% less natural-4-KiB time and 0.44–0.50% less random-1-MiB time than its matching clamped-window base. Natural 4 KiB remains **1.010619× / 1.017073× C**. Four historical carry/load/hash combinations retain the panel wins but earn no additional win; hash narrowing slows random 1 MiB by about 4.1%. All fourteen complete twenty-one-case discovery orders remain available. Full 81-case confidence qualification is outstanding.

The probe builds through final link in **2.217276 s**, paired **0.741816× Roc / 4.587254× C**, in its twelve-trial cohort. Twenty-five relevant repository tests pass, including 72,752 native calls; all seven candidates pass output/error/stack/recovery gates and checked artifact replay. The qualified 81 Roc / 62 C selection is unchanged. Next, isolate packet-load qualification by phase to recover its 416 B saving without the earlier hot-scorer slowdown. [Source, all observations, dynamic counts, complete builds and reproduction](evidence/encoder-size-gap/fixed-read-window-study/README.md).

Preceding clamped-window checkpoint:

Latest compiler source is **`330416904`**, adding the clamped-window proof and consuming late constants before native lowering. Word is **63,180 B**, probe **63,484 B** and packet-qualified **63,068 B**, all with 4,736 B peak stack. The probe improves natural 4 KiB by 1.04–1.53% and random 1 MiB by 1.61–1.62% against its matching base, while remaining **1.011199× / 1.009384× C** on natural 4 KiB. Every previously required peer win observed in six complete twenty-one-case orders remains a point win. Full qualification and win #63 are outstanding. Complete build is **2.213320 s**, paired **0.742111× Roc / 4.602746× C**. Twenty-three relevant tests, 47,552 native calls, fresh output/error/stack/recovery checks and checked replay pass. The earlier constant-branch version and all adverse counts remain available. The qualified result stays 63,612 B / 81 Roc / 62 C. Next, use the selected `window >= 5` edge to admit the remaining four-byte input read. [Source, all raw orders, machine counts, complete-build profiles and reproduction](evidence/encoder-size-gap/clamped-input-window-study/README.md).

Earlier experimental source is `253736215`: a guarded fixed-view floor removes two collector checks. The word candidate reaches 63,388 B / 4,720 B stack and preserves every prior win in both full 24-block point orders, with fresh outputs/failures/recoveries and checked emission. It remains unpromoted pending full confidence qualification. The qualified 63,612 B histogram worker remains selected at 81 Roc / 62 C. [Current experiment, adverse counts, recovered build-mode cost and reproduction](evidence/encoder-size-gap/bounded-index-range-study/README.md).

# PR #995 review and continuation handoff

**Latest qualified throughput recipe: `m1-bounded-byte-histogram`, 63,612 B, preserving every prior 81 Roc / 62 C win.** It improves random 1 MiB by 2.44–2.73% against its same-run 63,388 B reference. Forward96 and reverse288 pooled blocks retain all original data; the fresh reverse192 repeat independently supports all wins. The original reverse96 had 2 uncertain Roc comparisons and remains archived. Build 2.214983 s, paired 0.740659× Roc / 4.602734× C; stripped executable 276,072 B. No 63rd win. [Full evidence and reproducible recipe](evidence/encoder-size-gap/bounded-byte-histogram-study/README.md).

The 63,388 B recipe below remains the compact choice. The first bounded-index reuse prototype passed its native differential test but admitted no new checks in this codec. Inspection shows the 262,144-entry history floor is already exploited; observation counters use a different small array. Carry a separately guarded current extent for that array, or prove a checked read/write continuation, before making a performance claim. Preserve the scorer composition.

## Preceding compact reference and mechanisms

**Goal: reach 1× libdeflate performance. The next milestone is 63/81 C wins while preserving every existing individual C win and all 81 Roc wins.** The qualified reference is now `m1-word-complement-current-reads`: **63,388 B, 81/81 Roc and 62/81 C**, both point estimates and configured confidence checks in both complete 96-block orders. This is 144 B smaller than the previous reference. There is no 63rd C win.

Work is in [mech-lang/mech PR #995](https://github.com/mech-lang/mech/pull/995), branch `codex/pr993-m3-optimization`, targeting #993's `codex/native-compact-source-handoff`, stacked on #973. The branch contains compiler source, the original 2,719-line Mech codec, reproducible build recipes and the raw evidence. No LLVM dependency is added. Canonical bytecode remains `c6529d934f020f07f8e59f13fab40927e066649e202269818ad3f009a942a8c3`.

```sh
git fetch origin codex/pr993-m3-optimization
git switch -c codex/review-native-compact origin/codex/pr993-m3-optimization
```

## Qualified reference and reproduction

The selected timing worker is `fab303151eca902a95986488684102fc1707ec7f8df1324ebfd9c55543cfc368`. All 162 output/inflation/alias checks, six failures, twelve injected failures and twelve recoveries pass. Peak active stack is **4,864 B**, below the 5,280 B cap. Normal and allocation-checked compilers reproduce the complete object, metadata and bytecode. Qualification transfers between compiler builds only when the resulting artifacts and linked worker match exactly.

```sh
python3 rebuild.py /tmp/mech-word-complement --variant m1-word-complement-current-reads
python3 evidence/encoder-size-gap/word-complement-current-read-study/verify.py
python3 evidence/review-followup-20261119/verify.py
```

Use the existing optional allocator argument to link/check a worker. Different support can produce a different worker and requires its own runtime evidence. [Full qualification, original required win sets, recovery and reproduction](evidence/encoder-size-gap/word-complement-current-read-study/README.md).

The original timing compiler is 5bcc; integrated source `546af95af3735591a77455932dc48920a8fc071d` reproduces all artifacts with compiler 47ee. Commit `ce5abdb0d13cae82cf01f4935a43ab667971ce0c` adds the opt-in first-word probe, with disabled mode reproducing the same selected linked worker. Commit `00765f3be` makes an inapplicable saved-prefix strategy leave the graph unchanged and report no reuse. The latter passes repository tests and reproduces the exact qualified artifacts and linked worker in normal/checked word and word-reuse modes, with no misleading reuse metadata.

## Keep both finalists

The recipes already combine previous-DP-cost forwarding, cleanup and proved compared-byte reuse. Word/complement uses scalar word comparisons and complementary predicates; vector/narrow uses mismatch forwarding and narrow DP carry. Do not add their gains to earlier isolated DP gains.

The new direct cohort runs the control, both finalists, Roc and C together: **19 cases, all 120 five-actor permutations per case, both complete case orders**. Vector/narrow is faster on natural 4 KiB and small random inputs. Word/complement is smaller and stronger on fragile overlap/periodic wins. Keep both rather than choosing from separate cohorts' C ratios.

| Natural 4 KiB | Word/complement / C | Vector/narrow / C |
| --- | ---: | ---: |
| Direct cohort, forward | 1.037325× | 1.031229× |
| Direct cohort, reverse | 1.029387× | 1.021762× |

The word/vector differences are supported by the 90% paired intervals. Vector/narrow's reverse `320-overlap` result is **1.000442× C**, with interval **0.998664–1.002287**; repair this fragile case before treating it as a full-score replacement. A single near loss is a repair target, not grounds to discard the gain. Small slowdowns are allowed when every existing peer win remains intact.

The full qualified word result is a different cohort: **1.034702× / 1.043348× C** on natural 4 KiB, improving **2.73–2.74% against its same-run previous reference**. All six established small slowdowns and four uncertain control comparisons are retained. The earlier lower discovery ratios remain archived and do not replace the full result. [Direct comparison, all cases and confidence intervals](evidence/review-followup-20261119/README.md).

The secondary word-reuse/complement's original random-1-MiB reverse regime change is preserved, all 24 blocks included, at **1.015314× Roc**. Repeating its exact worker and original complete reverse order gives **0.960743× Roc**, without the earlier jump. This alternative is distinct from the qualified word-current worker. Neither run is selectively edited.

## Next mechanisms

Give substantial effort to the common path, particularly the bounded literal histogram and collector position/window facts. The histogram still performs five defensive checks per byte and executes **23,068,768 instructions / 7,340,036 branches** on random 1 MiB. Connect the exact input span, byte index range, fixed histogram extent, unique writable ownership, zero initialization and maximum increments. A guarded span/extent specialization can retain the checked U32 increment until initialization/count bounds are actually proved. Proof work must be bounded, and proof misses must leave the original checked path available. Carry position/extent relationships across entry and backedges; revisit the repeated observation read/write guard and producer-defined scorer roles.

The incremental first-word probe forwards a loaded XOR directly on failure, advances eight bytes on success and retains vectors for long comparisons. It admits both codec equality loops. On natural 4 KiB, **2,983 of 5,562 probes fail immediately**, while **2,579 enter vectors**. Its three profiled alternatives pass output/failure/stack gates; latency is still unmeasured here. Compare against both finalists and corresponding vector controls with other flags fixed.

Scorer winner-frequency measurement is complete: natural 4 KiB has **1,449 vector replacements among 3,550 reductions (40.82%)**. Natural 1/8 KiB are 34.77%/41.68%. Measured random/distribution-change cases have no vector reductions. Current code already reconstructs the horizontal argmin at reduction end, rather than every SIMD chunk. Deferred reconstruction must budget work for the roughly 41% of natural reductions that supply a winner. It does not directly address the random losses.

The old overwritten stack load and duplicate checked-subtraction comparison are absent in the current object. Do not reuse their historical instruction budgets. Retain source/error/ownership authority and direct Bool/U64/I64/F64 native paths.

## Build time, size and validation

Current-source matched complete-build medians are **2.240053 s Mech, 3.007427 s Roc and 0.487046 s C**, eight balanced fresh-process orders with compilers/support prebuilt for everyone and final linking included. The paired Mech ratios are **0.741226× Roc / 4.593452× C**. The previous compiler produces the same encoder in 2.229851 s; the paired difference is not established. The earlier ten-trial selection cohort measured 2.214373 s Mech and is archived separately. Bootstrap and strip are separate metrics.

Encoder text is **63,388 B Mech / 57,696 B Roc / 37,080 B C**. Stripped executables are **276,072 B / 191,912 B / 155,752 B**. Keep build time and artifact size tracked after each runtime repair; do not infer speed from size alone.

The relevant repository build now passes all **10 wide-equality tests**, including the new probe and exact no-op proof miss; **1,350 tests were filtered out**. The prior disk-exhausted build is retained. The repository unsafe audit still has the pre-existing missing allowlisted `benchmarks/mech-deflate/comparison/rust/src/pinned_allocator.rs`; no all-CI-green claim is made. Diagnostic-entry qualification does not establish ordinary managed-entry admission/recovery.

Keep the full-catalog repository test target separate from the smaller compiler target. Machine crates emit shared unhashed rlib/cdylib names, so switching feature sets in one target directory can leave a fresh fingerprint pointing at the other build's library. The original test binary and benchmark workers remain pinned while the compiler's affected libraries are rebuilt separately.

Historical alternatives remain available for repair: the [source-unit experiment](evidence/early-source-unit-repair/README.md), [history and loss profiles](evidence/history-and-profiles-20261119/README.md), [DP forwarding](evidence/previous-dp-cost-20261119/README.md), [compared-byte bounds](evidence/compared-byte-bounds-20261119/README.md), and [original mismatch discovery](evidence/vector-mismatch-20261119/README.md). The preceding 63,532 B and 63,692 B qualified artifacts are retained. The goal remains unfinished.

## Sequential scalar repair, not selected

[Source, all sixteen runtime orders and fourteen complete build trials](evidence/encoder-size-gap/scalar-sequential-study/README.md) are published with four portable normal/checked source replays and a passing public verifier. Sequential updates and a proved one-visit odd tail save48B and51,435 natural4KiB instructions versus the earlier paired helper, but still spill more than the selected scorer and lose the existing natural8KiB C win. The **81/63 point-win working selection remains m1-packed-eight-entry,63,868B**. Both partial gains remain available for repair.

The optional parallel low-word carry repair at source89dd67efb passes22 optimized tests in four configurations, including30,720 new loop/multi-edge native calls per configuration, and57 older recipes reproduce exactly. It changes the literal cost recurrence to use the separately carried low cost while the full word is stored. Instruction and branch counts and stack traffic remain unchanged. Its two experimental workers pass output/error/recovery/stack and normal/checked gates; no whole-call speed gain or promotion is claimed yet. The new brief prioritizes current matched Mech/C owner and semantic-work traces after this bounded check.
