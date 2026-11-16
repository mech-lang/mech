# R242: move dead phase-exit owner tokens into their results

Retain this bounded R241-based experiment for the periodic input. Periodic33,027B improves **1.652601x -> 1.627365x reference**, 1.5270% in pooled balanced block means. Both fixed cohorts improve. Eight bytes regresses **0.510449x -> 0.522129x**. Another 38.55% of current periodic time must disappear to reach parity.

All25 inputs/sizes are timed after the change, including the five held-out cases through131073B. Eleven pooled block means are slower. The table below retains every adverse row and both independent cohorts. Initial eight-byte block latency rises5.3197%, failing the old1.05 gate; confirmation falls0.3836%. Pooled gates pass25/25. The prospective user rule permits eight up to1x while driving the primary periodic case to parity; it does not erase an observed regression or turn the initial failure green.

Git parent a646d9d0a1e0f309c43d573728d2198fae87fb60; PR base92f75db84e2523693648084bea16cd52ad60777a. Control is the R241 experiment over R240/R234. Retained R237 f92415a31dbf0398ae641e66c419f435a3e3cb7c remains measured separately. Ordinary compiler R21542c95cfd79e0dc3773d03aeb9f553d32ff3922c5, IR/work/ABI/MBC215/107/39/14, remains unchanged.

## Every complete-computation timing

Two fixed24-balanced-block cohorts, all24 four-executable permutations, equal200ms warmup, level12/runtime inputs, matched live output ownership, pinned allocator and shared Rust clock. Full compression, scratch construction, output transfer and cleanup remain timed. Validation/preparation/logging/previous-output drop remain outside. Builds, tests and profiles do not overlap timing or counter cohorts. Multiples below use pooled observation medians against the current matched reference; block changes use medians of balanced block means. Their signs can differ, and neither is silently substituted for the other.

| Input | R241/reference | R242/reference | R237/reference | Pooled block change | Initial block change | Confirmation block change |
|---|---:|---:|---:|---:|---:|---:|
| empty | 1.976190x | 1.976190x | 1.976190x | +2.0429% | +1.2983% | +3.8440% |
| one-stored | 1.976190x | 1.976190x | 2.000000x | -1.8564% | -3.6473% | +1.6808% |
| seven-stored-prefix | 2.000000x | 2.000000x | 2.000000x | -0.6599% | -2.8048% | +1.0400% |
| eight-literals | 0.510449x | 0.522129x | 0.511459x | +1.5578% | +5.3197% | -0.3836% |
| eight-overlap | 0.495838x | 0.495983x | 0.496555x | +0.8878% | +2.1855% | -0.0975% |
| 320-overlap | 0.562933x | 0.561549x | 0.558222x | +0.4363% | +3.2076% | -2.9179% |
| eight-literals-prefix | 0.497989x | 0.499211x | 0.497488x | +1.1984% | +1.7703% | +0.4765% |
| 128-biased-dynamic | 0.505874x | 0.507024x | 0.501792x | +0.4566% | +0.8189% | +0.6130% |
| 32-high-entropy-stored | 0.536067x | 0.531638x | 0.528498x | -0.1658% | +1.4224% | -0.8490% |
| 16384-distribution-change | 2.748297x | 2.753283x | 2.774996x | +0.1814% | +0.5782% | -0.1857% |
| 33027-periodic-window | 1.652601x | 1.627365x | 1.755508x | -1.5270% | -1.5738% | -1.6643% |
| random-64 | 0.564475x | 0.565899x | 0.562340x | +0.5095% | +3.4518% | -1.4066% |
| random-256 | 0.655508x | 0.661644x | 0.659551x | +1.8336% | +1.2893% | +0.7007% |
| random-1024 | 0.749071x | 0.753119x | 0.756358x | +0.1051% | -0.3828% | +0.7451% |
| repeated-1024 | 0.661149x | 0.651061x | 0.668949x | -1.7432% | -1.4197% | -1.5510% |
| random-32769 | 2.758147x | 2.751177x | 2.861047x | -0.2527% | -0.2008% | -0.2128% |
| repeated-32769 | 1.552739x | 1.525077x | 1.649655x | -1.7815% | -1.5947% | -1.9011% |
| reymont-prefix-1024 | 0.931764x | 0.929778x | 0.917000x | +0.5273% | +0.0981% | +0.2194% |
| reymont-prefix-16384 | 2.198784x | 2.187019x | 2.094080x | -0.5351% | -0.5533% | -0.5165% |
| reymont-prefix-32769 | 2.245602x | 2.235129x | 2.151049x | -0.4664% | -0.3127% | -0.5758% |
| heldout-window-32767 | 2.237491x | 2.230366x | 2.148356x | -0.3185% | -0.3572% | -0.4215% |
| heldout-window-32768 | 2.241366x | 2.230940x | 2.151418x | -0.4652% | -0.4264% | -0.5482% |
| heldout-slides-65537 | 2.307768x | 2.298387x | 2.226041x | -0.4065% | -0.4434% | -0.3767% |
| heldout-random-65537 | 3.018658x | 3.008478x | 3.186790x | -0.3372% | -0.3127% | -0.3028% |
| heldout-mixed-131073 | 2.473993x | 2.465542x | 2.534329x | -0.3416% | -0.2902% | -0.3959% |

Periodic absolute medians: R241 2.0846875ms; R242 2.0528540ms; reference 1.2614585ms. Eight 147.541us -> 150.917us, reference 289.042us. All25 descriptive geometric block mean changes-0.0502%; large11 changes-0.5697%. These averages do not hide the adverse cases or determine the one-case acceptance decision. Preserved R237 remains faster on natural32: 2.151049x versus R242 2.235129x. Historical and fresh reference denominators remain separate.

## Independent whole-call counters

The separate CPU/allocation cohorts observe5200 complete calls and retain4800. Periodic instructions 40,478,852 -> 39,686,254, -1.9581%; cycles -2.3491%. Eight instructions 1,233,622 -> 1,232,276, while cycles rise1.1299%. Across25, instruction medians fall21/rise1/tie3; cycle medians fall16/rise9. They are not the timing cohort and must not be multiplied by sampled function shares.

| Input | Instruction change | Cycle change | R242 instructions/reference | Payload/heap fields |
|---|---:|---:|---:|---|
| empty | +0.0000% | -0.5383% | 1.122089x | identical |
| one-stored | +0.0000% | -7.5494% | 1.122949x | identical |
| seven-stored-prefix | +0.0000% | +4.3446% | 1.151643x | identical |
| eight-literals | -0.1091% | +1.1299% | 0.904856x | identical |
| eight-overlap | -0.1227% | +2.4896% | 0.891594x | identical |
| 320-overlap | -0.5382% | +0.8823% | 1.082785x | identical |
| eight-literals-prefix | -0.1160% | +2.5294% | 0.904755x | identical |
| 128-biased-dynamic | -0.0350% | -0.0559% | 0.974786x | identical |
| 32-high-entropy-stored | -0.1599% | +0.9851% | 1.028410x | identical |
| 16384-distribution-change | -0.0979% | -0.4643% | 3.699123x | identical |
| 33027-periodic-window | -1.9581% | -2.3491% | 2.046237x | identical |
| random-64 | -0.1023% | +1.5378% | 1.106076x | identical |
| random-256 | -0.0530% | +1.9228% | 1.426864x | identical |
| random-1024 | +0.0301% | +0.7745% | 1.773355x | identical |
| repeated-1024 | -0.9694% | -0.0726% | 1.336929x | identical |
| random-32769 | -0.2980% | -0.2346% | 4.163762x | identical |
| repeated-32769 | -1.9650% | -1.9917% | 2.040263x | identical |
| reymont-prefix-1024 | -0.3452% | -2.6200% | 1.849931x | identical |
| reymont-prefix-16384 | -0.5279% | -0.7076% | 2.673416x | identical |
| reymont-prefix-32769 | -0.5605% | -0.3222% | 2.802346x | identical |
| heldout-window-32767 | -0.5632% | -0.2135% | 2.803018x | identical |
| heldout-window-32768 | -0.5668% | -0.3703% | 2.803029x | identical |
| heldout-slides-65537 | -0.5829% | -0.4892% | 2.901142x | identical |
| heldout-random-65537 | -0.4903% | -0.5144% | 4.504316x | identical |
| heldout-mixed-131073 | -0.6331% | -0.3189% | 3.184741x | identical |

All25 payload allocation/requested-byte/initialized-copy/zero/private-peak and whole-heap request/requested-byte/reallocation/usable-peak fields are exactly equal. Periodic still has208 heap requests versus84 reference and9,500,860B private payload peak. This is a token-transport reduction, with no allocation, streaming or physical-copy win claimed. Complete RSS/physical-footprint counters are in counter-results.json; they are whole-executable/lifetime measures, not payload-copy volume.

## Proof, reached mechanism and emitted code

Two source files change: compact/phases.rs and one summary call in compact/emit.rs. Use existing array_outputs word/packet identity and OwnerFlow::retire_bypass. A result owner can receive one token from an exact current input owner home only if that home is dead on the successful exit, no result overwrites it, source and destination differ, and that source token has not already been transferred. Readonly results use the current view word; writable results use the exact current returned loan packet, including new backing after growth. All fallible source operations and result reads finish before clearing the old home. Prior-result release order and subsequent retirement remain. Shared and genuinely live aliases retain their own tokens; this grants no mutation uniqueness. Unknown identities use the existing retain/retire path. Failed native calls and shared-entry fallback remain unchanged.

The first one-prior-result census observed zero events in two correct full periodic calls. It is preserved as a disproved hypothesis; its uninstrumented complete Object matches R241. The refined dead-loan census observes33,205 nonnull exact-owner transfer opportunities on each of two correct periodic calls; every current identity agrees and payload fields match R241. Instrumented seconds are excluded from performance claims. Static selection is170 result/source pairs across the same134 phases, not a dynamic call count. Transfer is enabled only by MECH_PHASE_OWNER_TRANSFER=1 in the private standalone producer; original/decoded source controls enable it explicitly.

Root text shrinks5,654,028 ->5,630,692B (-23,336B); root stack stays55,152B. All136 numeric/map bodies are byte-identical. Existing64-byte equality groups and12 private-write intervals remain. There is no new numeric phase class, new runtime callback in the measured executable, or compressor source change. Separate diagnostic flags and a local observer exist only in the diagnostic build.

## Current sampled execution and next bounded target

33027-periodic-window: 5514 current-executable PC samples; _mech_compact_closed_v2 41.06%, _mech_compact_numeric_2624 33.33%, _mech_compact_numeric_15296 11.95%, _mech_compact_numeric_5538 2.96%.

eight-literals: 5459 current-executable PC samples; _mech_compact_closed_v2 28.70%, _mech_compact_numeric_17556 1.19%, __RNvNtNtCselbn9KGBQg0_11miniz_oxide7inflate4core9init_tree 0.93%, _mech_compact_numeric_16010 0.70%.

reymont-prefix-32769: 5363 current-executable PC samples; _mech_compact_numeric_15296 47.10%, _mech_compact_numeric_1279 24.07%, _mech_compact_closed_v2 21.87%, _mech_compact_numeric_15737 1.77%.

These are sampled time, not instruction partitions or percentages that can all be removed. Root transport remains substantial. Next trace where fixed length/row/column facts disappear through allocation, writable-buffer helpers and phase result packets. Preserve only independently proved invariant geometry across calls, refresh pointer-dependent facts after backing changes, and keep general matrix/alias/growth/error fallback. This larger target needs a minimal source witness and generated-code evidence before implementation; no speedup is claimed yet. Keep periodic primary, eight<=1x, and all25 timing/counter/regression controls. Deferred natural-terminal work stays parked.

## Qualification, failures and scope

703 maintained tests pass with six native features and-Dwarnings. The compiler library/producer use opt3; this round's maintained test root uses opt1, explicitly different from R241's opt3 test root. Three focused selector tests also pass separately. There are5,788 native matching controls, parent/candidate and original/decoded, exact errors/origins/current values/shapes/aliases/old publication/cleanup/retry. Matching resident rows are frozen R230 observations with exact source/library provenance plus an independent source-operation oracle. No fresh resident matching-call claim or allowance change.

An additional416 native boundary controls use four existing source programs in four forms. Growing and live-alias sources select the changed phase exit: real capacity1 growth, current owner change, shared fallback then native, assertion/arithmetic rejection, unchanged prior publication, cleanup and retry. The two fanout sources are unselected on clean R241 and candidate: retain160 controls each as fallback evidence, with320 fresh resident calls (236 values,72 source errors,12 inherited finite default denials) and exact root reference counts. Do not claim selected compiled-fanout qualification; unit tests cover single-source-token fanout selection. All16 small original/decoded image pairs are byte-identical. Fifty complete smokes and every timed/counter observation verify all25 exact reference streams, independent inflate, aliases, capacity and zero final nonreturned private ownership.

Introduced setup issues were fixed before accepted measurement: a private diagnostic import declaration and an invalid runner ternary. The runner's selected-fanout assumption failed on the clean parent too; classify it as an inherited unsupported selection class and retain unchanged expected source behavior. Disk exhaustion caused linker errno28 and a rustc output failure: completed88.8MB linked clients had filled storage. Delete2,247,728,656B of24 completed clients plus one prior completed test binary, preserving sources/Objects/receipts/dependencies/workers. Scripts now delete completed control clients after agreement. Exact failed receipts, cleanup hashes and corrected receipts are retained. No remaining semantic test failure or compressor resource denial is hidden.

Seven inherited maintained tests remain excluded by exact name in the command receipt. Experimental owner_flow.rs plus owner_flow/ layout debt remains assigned before ordinary integration. Hosted CI is cancelled by standing request; no green CI or clean exact-head review is claimed. Full Cargo/math/dependency rebuilding, multi-MB inputs and the separate ten-workload suite are not run. Complete source-vs-MBC Object equivalence is reused from R240; this round executes the strict revision14 producer and all changed-class original/decoded controls.

This standalone path still omits ordinary exact admission/resource/accounting/canonical publication/recovery contracts; they are neither restored nor counted as optimized away. Ordinary IR215 stays unchanged. Preserve source and bytecode pins, full level12 algorithm/tables/depth300/nice258/near-EOF rules,262144U16 tree,1500513U32 cache,305257U64 path capacity, eight-byte output13 clear/reuse, source allocation sites, aliases and timing endpoints. No LLVM change or new branch/PR.

## Pins and replay

- object: `07c378e3da5a030260d4a3864b74a2bbbd9f786d3f4e551d5078a9c10c1a0ad0`
- worker: `32f1a6b7cc1c3b127e6e978ff71ec78e0d1798fa97326faac18476aa6cd94283`
- compiler_patch: `11b7f1615dc932927b20b1031704d831757eeac6e867d67fdab80c0b832d8d1f`
- compiler_source_manifest: `596428c7b42be1e57d3d9ca3d4080b89efdb189773195d21fefb079a6c686e66`
- source: `331e659e92181d803b3d87218effe88f711c2e8ecad9f31838fc50991de1c5c9`
- mbc14: `a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7`
- reference_worker: `ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c`
- pinned_allocator: `38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874`

The packet retains exact source closure/patch, build and command specifications, every25-case observation, portable input/prefix/expected fixtures, focused assembly, counter/profile summaries and failure receipts. analyze-primary.py, analyze-counters.py and make-report.py regenerate the numerical reports; replay.py asserts five byte-identical reports and verify-packet.py verifies every archived file hash. Native binaries/Objects, bytecode, full executable metadata/descriptors, raw traces/raw stack samples, credentials and private environment exports are excluded.

[Complete tested source, patch and all25 observations](https://github.com/mech-lang/mech/releases/download/evidence-native-compression-experiments-6/r242-phase-owner-transfer-0547f382bc85.tar.gz). SHA-256`0547f382bc85dbe8de5c3dfc5043f05a0a82304cd7fcbd384799879880a39e9d`;5344191B. All1,669 file hashes and five numerical report replays verify after public retrieval.
