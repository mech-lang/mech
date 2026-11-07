# Current affine child funding — R174

Retained. Repeated32KB improves **3.682374× →3.241896× Roc** (-11.961799%) and periodic33KB improves **4.227835× →3.764477×** (-10.959703%). All eight balanced blocks improve in each case. Across all20 inputs,13 medians improve and7 worsen; geometric elapsed improves1.427851%. Natural32KB regresses **4.247301× →4.305974×** (+1.381432%), with all eight blocks adverse. Those observations are part of the retention decision, not discarded noise.

This compiler-only change preserves the complete pinned source and bytecode. It checks a current maximum visit cost once at an already admitted immutable child entry, then retains the original debit at every actual reached visit. It introduces no source shortcut, whole-loop prepayment, speculative source operation, deferred settlement, ownership grant or runtime ABI. The optional R172 source variant remains separate and its gain is not combined.

## Complete-call measurements

One predeclared eight-block balanced cohort contains3968 primary/4048 total calls. The largest input is33027B. All20 unchanged1.05 elapsed gates pass. Exact raw reference bytes, independent inflate, current input aliases, output prefixes/capacities, source work and zero final managed bytes/nodes pass. Reference timings are contemporaneous; historical ratios are not substituted. The table retains unresolved ratios when the reference median is zero.

| Input | Before / Roc | After / Roc | Elapsed change | Instructions change |
|---|---:|---:|---:|---:|
|repeated-32769|3.682374×|3.241896×|-11.961799%|-7.174753%|
|33027-periodic-window|4.227835×|3.764477×|-10.959703%|-6.726526%|
|random-32769|5.434707×|5.456511×|+0.401203%|-0.799959%|
|reymont-prefix-32769|4.247301×|4.305974×|+1.381432%|+0.469477%|
|eight-literals|4.830847×|4.762135×|-1.422365%|+0.019654%|
|empty|unresolved (zero reference median)|unresolved (zero reference median)|+0.520000%|-0.125394%|
|one-stored|unresolved (zero reference median)|unresolved (zero reference median)|+0.224221%|-0.129350%|
|seven-stored-prefix|unresolved (zero reference median)|unresolved (zero reference median)|+0.260597%|-0.128974%|
|eight-overlap|4.527947×|4.475214×|-1.164606%|+0.022837%|
|320-overlap|4.340532×|4.305025×|-0.818025%|-0.219339%|
|eight-literals-prefix|4.608019×|4.594676×|-0.289549%|+0.029076%|
|128-biased-dynamic|3.941564×|3.929516×|-0.305685%|-0.007692%|
|32-high-entropy-stored|4.314691×|4.303435×|-0.260879%|+0.024272%|
|16384-distribution-change|7.103876×|7.110180×|+0.088737%|-0.551539%|
|random-64|4.344397×|4.311638×|-0.754043%|+0.018355%|
|random-256|4.436793×|4.385875×|-1.147645%|-0.013593%|
|random-1024|3.988568×|3.976799×|-0.295080%|-0.151279%|
|repeated-1024|4.409484×|4.358567×|-1.154731%|-1.092559%|
|reymont-prefix-1024|4.961282×|4.937438×|-0.480589%|-0.022412%|
|reymont-prefix-16384|5.075964×|5.119493×|+0.857541%|+0.275138%|

Eight literals improve4.830847× →4.762135× in the pooled median, but only3 blocks improve and5 are adverse. Their instruction median rises0.019654% and cycle median rises1.788438%. The word-loop mechanism is not established on that tiny case. Random32KB rises0.401203% elapsed despite0.799959% fewer instructions; distribution16KB rises0.088737% despite0.551539% fewer instructions. Natural16KB rises0.857541%. Separate CPU cohorts and contrary pairs are retained.

## Executable mechanism and counters

| Input | Instructions before | Instructions after | Instructions change | Cycles change |
|---|---:|---:|---:|---:|
|eight-literals|12541718.5|12544183.5|+0.019654%|+1.788438%|
|16384-distribution-change|72258373.0|71859840.0|-0.551539%|+1.039620%|
|random-32769|79867006.0|79228103.0|-0.799959%|+0.609164%|
|repeated-32769|80026812.5|74285086.0|-7.174753%|-12.188622%|
|33027-periodic-window|85821337.5|80048542.5|-6.726526%|-11.726979%|
|reymont-prefix-32769|246833145.0|247991969.0|+0.469477%|+2.292092%|

Thirteen instruction medians fall and7 rise;12 cycle medians rise. No allocation or copying improvement is claimed. All20 allocation-request, requested-byte, reallocation and before/after/peak usable-live-heap medians are exactly equal. Eight literals remain3354 requests; repeated32KB3135; periodic33KB3679; random32KB5201; natural32KB6449.

Actual Object disassembly of the same source-matched header1392/root837 shows **25 →19** instructions on a selected steady full-word matching visit; the inspected pinned reference has10. Funding/visit/paid-state instructions fall10 →4. Three stack reads and two input-word reads remain. The new loop removes the repeated balance comparison and paid-flag branch, retaining the actual balance decrement and visit increment. The selected hot loop is compact rather than split around cold code; no cache-miss events were measured, so layout is not assigned a numerical saving. `funding-inspection/selected-path.json` retains exact PCs, instructions, assumptions and all Object/reference hashes. These static counts are not dynamic instruction attribution or a prediction of elapsed gain.

The current compiler selects nine statically funded children at root837:1372,1392,1781,2713,2733,3122,3928,3948,4337. Other parents already wholly fund their phases and are unchanged. A generic original/decoded direct/restored witness observes three actual selected activations versus zero with only ChildFunding disabled, including finite compute/comparison limits1<<60. There is no whole-compressor activation census; nine static sites must not be reported as nine dynamic calls.

Generated accounted text **8974316 →8951784B** (-22532B); reported entry stack **15840 →15792B** (-48B). Canonical plan and complete numeric recipes remain exactly equal. The independent metadata-only mapping build regenerates the byte-identical whole current Object, making its current source map valid for the timed executable. No previous map is applied to changed PCs.

## Current profiles and memory scope

Five fresh exact-primary-executable CPU profiles cover16506 complete calls. Repeated word extension falls from the separately retained R171 sample share22.794918% to18.481428%; current natural word extension is0.641026%. Current main-entry shares are11.782750% eight literals,73.713543% random32,55.218112% distribution16,73.743627% repeated32 and84.120879% natural32. Natural32 has37.380952% unattributed native samples; source site15529 in iteration171 alone carries18.553114% of all Running/Timer-Fired samples. The compact cost-reduction body inherits initializer context and is not fully included in the narrow path-cost label. Unknown native origins remain explicit.

Outside-entry includes other generated helpers, useful runtime algorithms, allocator and harness validation. The measured profile encompasses validation between complete calls; it is not an evaluate_into-only hardware-instruction partition. CPU-time samples are not retired instruction samples. This change removes native execution machinery; it does not lower another host operation or establish a new host/native percentage.

The first CPU/heap cohort has an adverse process-RSS observation (roughly31% higher on large cases), while its physical-footprint readings are slightly lower and every usable-live-heap metric is equal. A separate four-fresh-process ABBA diagnostic does not reproduce that gap: RSS medians323190784 →323305472B (+0.035486%); physical footprint233326336 →233350880B (+0.010519%). Its16 complete calls and region-only vmmap tables are retained. vmmap affects residency; neither cohort is silently replaced, and the original RSS variation has no established causal attribution. Requested capacity, initialized bytes copied, live heap, RSS and physical footprint remain distinct metrics.

## Proof and semantic qualification

The existing captured-window recurrence proves a positive constant stride, a checked k+step<=limit selected edge, invariant captures on every latch and an otherwise acyclic own scope. After ordinary child initialization pays visit one, the new gate bounds additional headers by floor((limit-start)/step). It checks current compute/comparison balances, high-word multiplication overflow and remaining counter space. A successful gate enters the original source header body after the already paid debit; backedges perform their actual original debits without repeated comparisons. A refused gate enters the unchanged checked header. Outside exits resume checked code. Each fresh activation derives its bound from its current arguments and current loaned ledger.

All fallible arithmetic, bounds/shape checks, lexical errors, early exits, first-change behavior, ownership, publication and cleanup remain in their existing paths. Fact preservation is not totality: even a deliberately overflowing checked add inside a selected funded visit retains the exact disabled-pass error/publication/retry behavior. No totality or completed-loop-prepayment refusal is relaxed.

Three focused tests cover original/decoded artifacts, direct/restored images, enabled/disabled funding, current changing coordinates, all first mismatch positions, short/tail/zero/unselected cases, row/column/general geometry, overflow/refused recurrence, finite work and library limits, actual checked-header fallback, live aliases, previous output preservation, immediate cleanup and recovery. Native test-only atomic counters prove actual selected entries while preserving the successful-body prohibition on general calls; opt-in test compilation alone emits them. A precise audited allowlist entry names the static lifetime/alignment/atomic invariant and grants no production authority.

Qualification passes632 GPU tests with the same seven inherited exclusions. All215 CI-selected names execute and pass (strict inventory212 →215 for the three new controls). The existing236-component/8-composition/1-real-capacity-path qualification is rerun using the byte-identical prior wrapper, with expected values and allowances unchanged. All12 balanced original/decoded Bool/U64/I64/F64 and short-loop controls pass the unchanged1.05 latency and allocation gates. All15 architecture/catalog checks,132 workflow tests, runtime-only, formatting and warnings-as-errors pass.

Exact commands and feature flags are retained in `unit-final-fixed/{build,test}.json`, `gpu-compatible-final*.json`, `ci-inventory.json`, `qualification/*.json`, `scalar-controls/*.json`, `scalar-balanced/*` and `checks/receipts.json`. GPU/root harnesses alone are rebuilt with frozen R145 Core/Engine/math/dependencies; optimized performance library opt-level3, unit library opt-level1, nightly-2026-03-03/aarch64. The native profile enables canonical-native, canonical-native-program, their runtime features, jit and aot; runtime-only omits compilation features. A fresh all-feature/dependency build, multi-megabyte inputs, an additional standalone AOT harness and the separate ten-workload suite are skipped. The full compressor itself executes through the linked Object, and generic error/publication controls also exercise restored images.

## Failed attempts and inherited coverage

All prototype failures are retained. Initial gate arguments were inserted into the wrong helper; a simple witness was already wholly funded and did not exercise checked-child funding. Adding an ordinary unbounded sibling scan to the generic caller establishes the intended checked-child context without changing the recognizer. Callback instrumentation was correctly rejected by the existing no-general-call audit; opt-in native atomics replace it. An unused guard field and misplaced test-only policy assertion were repaired. The retry witness initially compared different prior-publication states; the corrected differential uses identical current/disabled states and preserves exact outputs, errors, work and settlements.

A same-name module file/directory collision was repaired. The component wrapper was initially absent, then copied byte-for-byte from the qualified baseline. The first unsafe-boundary audit rejected the new test counter address; one explicit invariant entry fixes it and the full audit passes. A differently named rlib container did not hash equally; rebuilding with the exact same output name proves byte identity. Copied CI/test-revision metadata initially named old tests/revisions; retained raw listing/build receipts identify215 current names and clean source65021c. Corrected metadata and its initial versions are both retained. These are repaired introduced prototype/harness defects, not silently excluded failures or relaxed contracts.

The same seven inherited exclusions remain: three batched AOT controls (rejected updates, concurrent builds, ISA identity), two batched axis/legacy controls (singular solve, matrix multiply), the batched JIT rejected-update control, and the unchanged knucleotide owned-map control. Their exact names and inherited baseline9017ab40d19047f39bf1832c5c85d05f00accb5d are in the GPU spec. No newly failing functional test remains. The natural-text elapsed/instruction regression and original nonreproduced RSS discrepancy remain visible limitations. Hosted ordinary CI is cancelled by standing instruction; review automation remains enabled and #973 stays draft. No green-hosted-CI claim.

## Brief disposition, pins and next bounded step

The latest brief is **partly addressed**. Its source ledger and one selected frequency-reuse variant were completed independently in R172; exact-model stability had zero hits, so that shortcut was not implemented. The broader mutation-aware transfer/parent-to-child fact project remains unimplemented: observed repeated child-setup weight was about1.03% repeated,0.24% natural and0% random/distribution, below its approximately10% research hurdle. Parent scope size, nested edges and terminal IntegrityFailure also obstruct qualification; merely allowing writes is insufficient. This patch addresses the separate early-exit funding opportunity on an already admitted immutable child. It does not claim the requested mutable-parent proof has landed, nor that the entire brief or parity goal is complete. Private store ordering, numeric outlining and reusable owner-slot workspace remain unselected experiments.

Parent `fb37ded7af3dd59bb6cd5f13f5044aa29e91833e`; measured production source initially `306c100555f60a81cc8616b7bf2a3bb55f059fca`, final compiler/test source `65021c3593137219762477ccde0f2d3ad7993977`. Their only difference is cfg(test) repair; a fresh same-output-name optimized library is byte-identical (SHA-256 `4959b828202c2d3f11f8ed910218b94ed9661cee1853750f0f76e2f670b22f31`). The public patch reconstructs all12 compiler/workflow/fixture file hashes from the parent. The final publication adds the explicit test-counter invariant, design report and evidence index to the same optimization commit. Comparison compiler `9a0fb83a20ae9ce12f4251d209c03b754dbc82da` (retained R171; rejected R173 is not used as the baseline). IR/work/ABI/MBC194/107/39/14.

Current Object `e535001f09ccffbc9100d2fea31b6e862086fc34ac60d4000f4e0a4ce43820ce`; worker `9ef812c014c40c0c3a33dc497adcbbf909f32311410b94ea38d97b7357aef1df`. Source `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`; bytecode `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`. Preserve the level12 algorithm, tables, search/EOF/tie rules, depth300/nice258, source allocation sites,262144-entry U16 tree,1500513-word U32 cache,305257 U64 path capacity, output13 clear/reuse, current input aliases, pinned mimalloc and full evaluate_into timing. No compressor-source edit is part of this compiler result. Logical source work is not a hardware instruction count.

Next bounded investigation: inspect the actual hot DP initializer/reduction continuation at source site15529 and its unknown neighboring native spans against the reference. Separate useful per-candidate reduction work from repeated state/control transport before choosing a compiler change. This now has much more natural-text weight than word-loop setup. The remaining three word-loop stack reads are a separate concrete instruction target. Neither investigation has a new speedup claim yet.

Every timing case was sealed immediately; the hash-indexed public packets retain source patch, source/fixture hashes, raw elapsed/counter/heap observations, current profiles/maps/disassembly, all test/check receipts, contrary pairs and receipt-only replay. Native binaries, bytecode, raw trace containers and private environment headers are excluded. One optimization commit in existing #973, kept draft; the active1× goal remains open.
